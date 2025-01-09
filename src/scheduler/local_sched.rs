use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{mpsc, watch, Semaphore};

use crate::{
    executor, pending_manager,
    pleiades_type::{Job, JobStatus},
    updater,
};

/// Policy
///
///
///
///
pub enum Policy {
    Cooperative,
    Blocking,
}

/// Scheduler
///
///
///
///
pub struct LocalSched {
    id: usize,
    command_receiver: mpsc::Receiver<Command>,
    semaphore: Arc<Semaphore>,

    action_sender: watch::Sender<()>,

    _queuing: Arc<Mutex<Duration>>,
    _pending: Arc<Mutex<Duration>>,

    controller: Controller,
    executor: executor::Controller,
    updater: updater::Controller,
    pending_manager: pending_manager::Controller,
}

impl LocalSched {
    const MAX_CONCURRENCY: usize = 128;

    /// new
    ///
    ///
    pub fn new(
        id: usize,
        executor_controller: executor::Controller,
        updater_controller: updater::Controller,
        pending_manager: pending_manager::Controller,
        action_sender: watch::Sender<()>,
    ) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(64);
        let queuing = Arc::new(Mutex::new(Duration::ZERO));
        let pending = Arc::new(Mutex::new(Duration::ZERO));

        let controller = Controller {
            id,
            command_sender,
            queuing: queuing.clone(),
            pending: pending.clone(),
        };

        let local_sched = Self {
            id,
            command_receiver,
            semaphore: Arc::new(Semaphore::new(Self::MAX_CONCURRENCY)),
            action_sender,
            _queuing: queuing,
            _pending: pending,
            controller: controller.clone(),
            executor: executor_controller,
            updater: updater_controller,
            pending_manager,
        };

        (local_sched, controller)
    }

    /// run
    ///
    ///
    pub async fn run(&mut self, policy: Policy) {
        tracing::info!("LocalSched {}: running", self.id);

        match policy {
            Policy::Cooperative => self.cooperative().await,
            Policy::Blocking => self.blocking().await,
        }

        tracing::info!("LocalSched {}: shutdown", self.id);
    }

    /// wait_for_shutdown
    ///
    ///
    async fn schedule_shutdown(&self) {
        let local_sched = self.controller.clone();
        let semaphore = self.semaphore.clone();

        tokio::spawn(async move {
            let _ = semaphore.acquire_many(Self::MAX_CONCURRENCY as u32).await;
            local_sched.signal_shutdown_done().await;
        });

        tracing::info!("LocalSched {}: scheduled shutdown", self.id);
    }

    async fn enqueue_execute(&self, job: Job) {
        let prev_rem_time = job.remaining;
        let handle = self.executor.enqueue(job).await;
        let local_sched = self.controller.clone();
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        let id = self.id;

        tokio::spawn(async move {
            let response = handle.response_receiver.await.unwrap();
            local_sched.ready(response.job, prev_rem_time).await;
            tracing::debug!("LocalSched {}: enqueued job to executor", id);

            drop(permit);
        });
    }

    async fn enqueue_pend(&self, job: Job) {
        let prev_rem_time = job.remaining;
        let handle = self.pending_manager.register(job).await;
        let local_sched = self.controller.clone();
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        tokio::spawn(async move {
            let response = handle.response_receiver.await.unwrap();
            local_sched.ready(response.job, prev_rem_time).await;

            drop(permit);
        });
    }

    async fn signal_local_action(&self) {
        self.action_sender.send(()).unwrap();
    }
}

impl LocalSched {
    /// cooperative_pipeline
    ///
    ///
    ///
    ///
    async fn cooperative(&mut self) {
        let mut shutdown_flag = false;

        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Command::Enqueue(enqueue::Request { job, prev_rem_time }) => match job.status {
                    JobStatus::Assigned => {
                        // don't need to add_queuing
                        self.enqueue_execute(job).await;
                    }
                    JobStatus::Ready(_) => {
                        self.controller.sub_pending(prev_rem_time);
                        self.controller.add_queuing(job.remaining);
                        self.enqueue_execute(job).await;
                    }
                    JobStatus::Pending(_) => {
                        self.controller.sub_queuing(prev_rem_time);
                        self.controller.add_pending(job.remaining);
                        self.enqueue_pend(job).await;

                        if !shutdown_flag {
                            self.signal_local_action().await;
                        }
                    }
                    JobStatus::Finished(_) | JobStatus::Cancelled => {
                        self.controller.sub_queuing(prev_rem_time);
                        self.updater.update_job(job).await;

                        if !shutdown_flag {
                            self.signal_local_action().await;
                        }
                    }
                    _ => unreachable!(),
                },
                Command::ShutdownReq => {
                    self.schedule_shutdown().await;
                    shutdown_flag = true;
                }
                Command::ShutdownDone => break,
            }
        }
    }

    async fn blocking(&mut self) {
        let mut shutdown_flag = false;

        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Command::Enqueue(enqueue::Request { job, prev_rem_time }) => {
                    let mut job = job;

                    loop {
                        job = match job.status {
                            JobStatus::Assigned | JobStatus::Ready(_) => {
                                let handle = self.executor.enqueue(job).await;
                                let response = handle.response_receiver.await.unwrap();

                                response.job
                            }
                            JobStatus::Pending(_) => {
                                let handle = self.pending_manager.register(job).await;
                                let response = handle.response_receiver.await.unwrap();

                                response.job
                            }
                            JobStatus::Finished(_) | JobStatus::Cancelled => {
                                self.controller.sub_queuing(prev_rem_time);
                                self.updater.update_job(job).await;

                                if !shutdown_flag {
                                    self.signal_local_action().await;
                                }

                                break;
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                Command::ShutdownReq => {
                    self.schedule_shutdown().await;
                    shutdown_flag = true;
                }
                Command::ShutdownDone => break,
            }
        }
    }
}

/// Controller
///
///
///
///
#[derive(Clone)]
pub struct Controller {
    pub id: usize,
    command_sender: mpsc::Sender<Command>,
    queuing: Arc<Mutex<Duration>>,
    pending: Arc<Mutex<Duration>>,
}

impl Controller {
    pub async fn assign(&mut self, job: Job) {
        // this is necessary to avoid double counting
        self.add_queuing(job.remaining);

        let prev_rem_time = job.remaining;
        let request = Command::Enqueue(enqueue::Request { job, prev_rem_time });

        self.command_sender.send(request).await.unwrap();
    }

    pub async fn ready(&self, job: Job, prev_rem_time: Duration) {
        let request = Command::Enqueue(enqueue::Request { job, prev_rem_time });
        self.command_sender.send(request).await.unwrap();
    }

    pub async fn signal_shutdown_req(&self) {
        let request = Command::ShutdownReq;
        self.command_sender.send(request).await.unwrap();
    }

    pub async fn signal_shutdown_done(&self) {
        let request = Command::ShutdownDone;
        self.command_sender.send(request).await.unwrap();
    }

    pub fn queuing(&self) -> Duration {
        *self.queuing.lock().unwrap()
    }

    fn add_queuing(&mut self, duration: Duration) {
        *self.queuing.lock().unwrap() += duration;
    }

    fn sub_queuing(&mut self, duration: Duration) {
        *self.queuing.lock().unwrap() -= duration;
    }

    pub fn pending(&self) -> Duration {
        *self.pending.lock().unwrap()
    }

    fn add_pending(&mut self, duration: Duration) {
        *self.pending.lock().unwrap() += duration;
    }

    fn sub_pending(&mut self, duration: Duration) {
        *self.pending.lock().unwrap() -= duration;
    }

    pub fn used_time(&self) -> Duration {
        self.queuing() + self.pending()
    }
}

#[derive(Debug)]
pub enum Command {
    Enqueue(enqueue::Request),
    ShutdownReq,
    ShutdownDone,
}

pub mod enqueue {
    use super::*;

    #[derive(Debug)]
    pub struct Request {
        pub job: Job,
        pub prev_rem_time: Duration,
    }
}
