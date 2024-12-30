use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{mpsc, Semaphore};

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
    // FastContract,
    // BlockingPipeline(Duration),
    CooperativePipeline,
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

    queuing: Arc<Mutex<Duration>>,
    pending: Arc<Mutex<Duration>>,

    local_sched: Controller,
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
    ) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(64);
        let queuing = Arc::new(Mutex::new(Duration::ZERO));
        let pending = Arc::new(Mutex::new(Duration::ZERO));

        let controller = Controller {
            command_sender,
            queuing: queuing.clone(),
            pending: pending.clone(),
        };

        let local_sched = Self {
            id,
            command_receiver,
            semaphore: Arc::new(Semaphore::new(Self::MAX_CONCURRENCY)),
            queuing,
            pending,
            local_sched: controller.clone(),
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
            Policy::CooperativePipeline => self.cooperative_pipeline().await,
        }

        tracing::info!("LocalSched {}: shutdown", self.id);
    }

    /// wait_for_shutdown
    ///
    ///
    async fn schedule_shutdown(&self) {
        let local_sched = self.local_sched.clone();
        let semaphore = self.semaphore.clone();

        tokio::spawn(async move {
            let _ = semaphore.acquire_many(Self::MAX_CONCURRENCY as u32).await;
            local_sched.signal_shutdown_done().await;
        });

        tracing::info!("LocalSched {}: scheduled shutdown", self.id);
    }

    async fn enqueue_execute(&self, job: Job) {
        let prev_rem_time = job.rem_time;
        let handle = self.executor.enqueue(job).await;
        let local_sched = self.local_sched.clone();
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
        let prev_rem_time = job.rem_time;
        let handle = self.pending_manager.register(job).await;
        let local_sched = self.local_sched.clone();
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        tokio::spawn(async move {
            let response = handle.response_receiver.await.unwrap();
            local_sched.ready(response.job, prev_rem_time).await;

            drop(permit);
        });
    }

    fn add_queuing(&mut self, duration: Duration) {
        *self.queuing.lock().unwrap() += duration;
    }

    fn sub_queuing(&mut self, duration: Duration) {
        *self.queuing.lock().unwrap() -= duration;
    }

    fn add_pending(&mut self, duration: Duration) {
        *self.pending.lock().unwrap() += duration;
    }

    fn sub_pending(&mut self, duration: Duration) {
        *self.pending.lock().unwrap() -= duration;
    }
}

impl LocalSched {
    /// cooperative_pipeline
    ///
    ///
    ///
    ///
    async fn cooperative_pipeline(&mut self) {
        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Command::Enqueue(enqueue::Request { job, prev_rem_time }) => match job.status {
                    JobStatus::Assigned => {
                        self.add_queuing(job.rem_time);
                        self.enqueue_execute(job).await;
                    }
                    JobStatus::Ready(_) => {
                        self.sub_pending(job.rem_time);
                        self.enqueue_execute(job).await;
                    }
                    JobStatus::Pending(_) => {
                        self.add_pending(job.rem_time);
                        self.enqueue_pend(job).await;
                    }
                    JobStatus::Finished(_) | JobStatus::Cancelled => {
                        self.sub_queuing(prev_rem_time);
                        self.updater.update_job(job).await;
                    }
                    _ => unreachable!(),
                },
                Command::ShutdownReq => self.schedule_shutdown().await,
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
    command_sender: mpsc::Sender<Command>,
    queuing: Arc<Mutex<Duration>>,
    pending: Arc<Mutex<Duration>>,
}

impl Controller {
    pub async fn assign(&self, job: Job) {
        let request = Command::Enqueue(enqueue::Request {
            job,
            prev_rem_time: Duration::ZERO,
        });
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

    pub fn pending(&self) -> Duration {
        *self.pending.lock().unwrap()
    }

    pub fn holding(&self) -> Duration {
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
