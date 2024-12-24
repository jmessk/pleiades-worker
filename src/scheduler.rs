use std::{
    ops::AddAssign,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::mpsc;

use crate::{
    contractor, executor,
    helper::{ExecutorManager, WorkerIdManager},
    pending_manager,
    pleiades_type::{Job, JobStatus},
    updater,
};

/// Controllers
///
///
///
///
struct Controllers {
    scheduler: Controller,
    contractor: contractor::Controller,
    updater: updater::Controller,
    pending: pending_manager::Controller,
}

/// Scheduler
///
///
///
///
pub struct Scheduler {
    command_receiver: mpsc::Receiver<Command>,
    task_counter: Arc<AtomicUsize>,

    contracting: Duration,
    pending: Duration,

    controllers: Controllers,
    executor_manager: ExecutorManager,
    worker_id_manager: WorkerIdManager,
}

impl Scheduler {
    /// new
    ///
    ///
    pub fn new(
        worker_id_manager: WorkerIdManager,
        contractor_controller: contractor::Controller,
        updater_controller: updater::Controller,
        pending_manager_controller: pending_manager::Controller,
        executor_manager: ExecutorManager,
    ) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(128);

        let controller = Controller { command_sender };

        let scheduler = Self {
            command_receiver,
            task_counter: Arc::new(AtomicUsize::new(0)),
            contracting: Duration::default(),
            pending: Duration::default(),
            controllers: Controllers {
                scheduler: controller.clone(),
                contractor: contractor_controller,
                updater: updater_controller,
                pending: pending_manager_controller,
            },
            executor_manager,
            worker_id_manager,
        };

        (scheduler, controller)
    }

    /// run
    ///
    ///
    pub async fn run(&mut self) {
        tracing::info!("running");

        self.fast_contract_policy().await;

        tracing::info!("shutdown");
    }

    /// wait_for_shutdown
    ///
    ///
    async fn background_wait_shutdown(&self) {
        let task_counter = self.task_counter.clone();
        let scheduler_controller = self.controllers.scheduler.clone();

        tokio::spawn(async move {
            tracing::info!("waiting for shutdown");

            while task_counter.load(Ordering::Relaxed) > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                scheduler_controller.signal_shutdown().await;

                tracing::debug!("tasks: {}", task_counter.load(Ordering::Relaxed));
            }
        });
    }

    /// contract_background
    ///
    ///
    async fn background_contract(&self, worker_id: &str) {
        let handle = self
            .controllers
            .contractor
            .try_contract(worker_id.to_string())
            .await
            .unwrap();

        let scheduler_controller = self.controllers.scheduler.clone();
        let task_counter = self.task_counter.clone();

        tokio::spawn(async move {
            task_counter.fetch_add(1, Ordering::Relaxed);

            let response = handle.recv().await;
            match response.contracted {
                Some(job) => scheduler_controller.enqueue(job).await,
                None => scheduler_controller.signal_no_job().await,
            }

            task_counter.fetch_sub(1, Ordering::Relaxed);
        });
    }

    async fn background_execute(&self, job: Job, executor_controller: &executor::Controller) {
        let handle = executor_controller.register(job).await;
        let scheduler_controller = self.controllers.scheduler.clone();
        let task_counter = self.task_counter.clone();

        tokio::spawn(async move {
            task_counter.fetch_add(1, Ordering::Relaxed);

            let response = handle.response_receiver.await.unwrap();
            scheduler_controller.enqueue(response.job).await;

            task_counter.fetch_sub(1, Ordering::Relaxed);
        });
    }

    async fn background_pend(&self, job: Job) {
        let handle = self.controllers.pending.register(job).await;
        let scheduler_controller = self.controllers.scheduler.clone();
        let task_counter = self.task_counter.clone();

        tokio::spawn(async move {
            task_counter.fetch_add(1, Ordering::Relaxed);

            let response = handle.response_receiver.await.unwrap();
            scheduler_controller.enqueue(response.job).await;

            task_counter.fetch_sub(1, Ordering::Relaxed);
        });
    }
}

impl Scheduler {
    async fn fast_contract_policy(&mut self) {
        let mut shutdown_flag = false;

        let default_worker_id = self.worker_id_manager.get_default();

        for _ in 0..self.controllers.contractor.max_concurrency {
            // for _ in 0..8 {
            self.background_contract(&default_worker_id).await;
        }

        while let Some(command) = self.command_receiver.recv().await {
            // println!("------command: {:?}", command);
            match command {
                Command::Enqueue(enqueue::Request { job }) => match job.status {
                    JobStatus::Assigned | JobStatus::Ready(_) => {
                        let executor = self.executor_manager.shortest();
                        self.background_execute(job, executor).await;
                        // println!("Enqueued job to Executor {:?}", executor.id);
                    }
                    JobStatus::Finished(_) => {
                        // println!("Job is finished");
                        // println!("Job remaining time: {:?}", job.remaining_time);
                        self.controllers.updater.update_job(job).await;

                        if shutdown_flag {
                            continue;
                        }

                        self.background_contract(&default_worker_id).await;
                    }
                    JobStatus::Pending(_) => {
                        // println!("Job is pending");
                        self.background_pend(job).await;
                    }
                    JobStatus::Cancelled => {
                        // println!("Job is cancelled");
                        self.controllers.updater.update_job(job).await;

                        if shutdown_flag {
                            continue;
                        }

                        self.background_contract(&default_worker_id).await;
                    }
                    _ => unreachable!(),
                },
                Command::NoJob => {
                    // println!("No job is available");

                    if shutdown_flag {
                        continue;
                    }

                    self.background_contract(&default_worker_id).await;
                }
                Command::Shutdown => {
                    if !shutdown_flag {
                        self.background_wait_shutdown().await;
                        shutdown_flag = true;
                    }
                }
            }

            if shutdown_flag && self.task_counter.load(Ordering::Relaxed) == 0 {
                break;
            }
        }
    }

    async fn deadline_policy(&mut self) {
        const JOB_DEADLINE: Duration = Duration::from_millis(500);

        let mut shutdown_flag = false;
        let default_worker_id = self.worker_id_manager.get_default();

        for _ in 0..self.controllers.contractor.max_concurrency {
            self.background_contract(&default_worker_id).await;
        }

        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Command::Enqueue(enqueue::Request { job }) => match job.status {
                    JobStatus::Assigned => {
                        self.contracting -= JOB_DEADLINE;
                        let executor = self.executor_manager.shortest();

                        self.background_execute(job, executor).await;
                    }
                    JobStatus::Ready(_) => {
                        let executor = self.executor_manager.shortest();
                        self.background_execute(job, executor).await;
                    }
                    JobStatus::Finished(_) => {
                        self.controllers.updater.update_job(job).await;

                        if shutdown_flag {
                            continue;
                        }

                        self.background_contract(&default_worker_id).await;
                    }
                    JobStatus::Pending(_) => self.background_pend(job).await,
                    JobStatus::Cancelled => {
                        self.controllers.updater.update_job(job).await;

                        if shutdown_flag {
                            continue;
                        }

                        self.background_contract(&default_worker_id).await;
                    }
                    _ => unreachable!(),
                },
                Command::NoJob => {
                    self.contracting -= JOB_DEADLINE;

                    if shutdown_flag {
                        continue;
                    }

                    self.background_contract(&default_worker_id).await;
                }
                Command::Shutdown => {
                    if !shutdown_flag {
                        self.background_wait_shutdown().await;
                        shutdown_flag = true;
                    }
                }
            }

            if shutdown_flag && self.task_counter.load(Ordering::Relaxed) == 0 {
                break;
            }
        }
    }

    // pub async fn
}

/// Controller
///
///
///
///
#[derive(Clone)]
pub struct Controller {
    command_sender: mpsc::Sender<Command>,
}

impl Controller {
    /// enqueue_ready
    ///
    pub async fn enqueue(&self, job: Job) {
        let request = Command::Enqueue(enqueue::Request { job });
        self.command_sender.send(request).await.unwrap();
    }

    pub fn enqueue_nowait(&self, job: Job) {
        let request = Command::Enqueue(enqueue::Request { job });
        self.command_sender.blocking_send(request).unwrap();
    }

    pub async fn signal_no_job(&self) {
        let request = Command::NoJob;
        self.command_sender.send(request).await.unwrap();
    }

    pub async fn signal_shutdown(&self) {
        let request = Command::Shutdown;
        self.command_sender.send(request).await.unwrap();
    }

    // /// enqueue_assigned
    // ///
    // pub async fn enqueue_assigned(&self, job: Job) {
    //     let request = Command::Assigned(assigned::Request { job });
    //     self.command_sender.send(request).await.unwrap();
    // }

    // /// enqueue_pending
    // ///
    // pub async fn enqueue_pending(&self, job: Job) {
    //     let request = Command::Assigned(assigned::Request { job });
    //     self.command_sender.send(request).await.unwrap();
    // }
}

#[derive(Debug)]
pub enum Command {
    Enqueue(enqueue::Request),
    NoJob,
    // Assigned(assigned::Request),
    // Pending(pending::Request),
    Shutdown,
}

pub mod enqueue {
    use super::*;

    #[derive(Debug)]
    pub struct Request {
        pub job: Job,
    }
}

// pub mod assigned {
//     use super::*;

//     pub struct Request {
//         pub job: Job,
//     }
// }

// pub mod pending {
//     use super::*;

//     pub struct Request {
//         pub job: Job,
//     }
// }
