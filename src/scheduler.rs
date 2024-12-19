pub mod policy;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::mpsc;

use crate::{
    contractor, executor,
    helper::{ExecutorManager, WorkerIdManager},
    pending_manager,
    pleiades_type::{Job, JobStatus},
    updater,
};

/// Scheduler
///
///
///
///
pub struct Scheduler {
    /// interface to access this component
    ///
    command_receiver: mpsc::Receiver<Command>,

    /// number of jobs currently being contracted
    ///
    task_counter: Arc<AtomicUsize>,

    /// worker_id_set
    ///
    worker_id_manager: WorkerIdManager,

    /// scheduler_controller
    ///
    scheduler_controller: Controller,

    /// contractor_controller
    ///
    contractor_controller: contractor::Controller,

    /// updater_controller
    ///
    updater_controller: updater::Controller,

    /// pending_controller
    ///
    pending_manager_controller: pending_manager::Controller,

    /// executor_controller
    ///
    executor_manager: ExecutorManager,
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

        let updater = Self {
            command_receiver,
            task_counter: Arc::new(AtomicUsize::new(0)),
            worker_id_manager,
            scheduler_controller: controller.clone(),
            contractor_controller,
            updater_controller,
            pending_manager_controller,
            executor_manager,
        };

        (updater, controller)
    }

    /// set_controller
    // pub fn set_controller(&mut self, executor_manager: ExecutorManager) {
    //     self.executor_manager = Some(executor_manager);
    // }

    /// run
    ///
    ///
    pub async fn run(&mut self) {
        self.default_policy().await;
        self.background_wait_shutdown().await;
    }

    async fn default_policy(&mut self) {
        let default_worker_id = self.worker_id_manager.get_default();

        for _ in 0..self.executor_manager.num_executors() {
            self.background_contract(&default_worker_id).await;
        }

        let mut shutdown_flag = false;

        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Command::Enqueue(enqueue::Request { job }) => match job.status {
                    JobStatus::Assigned | JobStatus::Ready(_) => {
                        println!("Job is assigned");

                        let executor = self.executor_manager.current_shortest();
                        self.background_execute(job, executor).await;
                    }
                    JobStatus::Finished(_) => {
                        println!("Job is finished");
                        self.updater_controller.update_job(job).await;

                        if shutdown_flag {
                            continue;
                        }

                        self.background_contract(&default_worker_id).await;
                    }
                    JobStatus::Pending(_) => {
                        println!("Job is pending");
                        self.background_pend(job).await;
                    }
                    JobStatus::Cancelled => {
                        println!("Job is cancelled");
                        self.updater_controller.update_job(job).await;

                        if shutdown_flag {
                            continue;
                        }

                        self.background_contract(&default_worker_id).await;
                    }
                    _ => unreachable!(),
                },
                Command::NoJob => {
                    println!("No job is available");

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

        println!("Scheduler is shut down");
    }

    /// wait_for_shutdown
    ///
    async fn background_wait_shutdown(&self) {
        let task_counter = self.task_counter.clone();
        let scheduler_controller = self.scheduler_controller.clone();

        tokio::spawn(async move {
            println!("Scheduler is shutting down");

            while task_counter.load(Ordering::Relaxed) > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                scheduler_controller.signal_shutdown().await;
            }
        });
    }
}

/// tasks
///
///
impl Scheduler {
    /// contract_background
    ///
    ///
    async fn background_contract(&self, worker_id: &str) {
        let handle = self
            .contractor_controller
            .try_contract(worker_id.to_string())
            .await
            .unwrap();

        let scheduler_controller = self.scheduler_controller.clone();
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

        let scheduler_controller = self.scheduler_controller.clone();
        let task_counter = self.task_counter.clone();

        tokio::spawn(async move {
            task_counter.fetch_add(1, Ordering::Relaxed);

            let response = handle.response_receiver.await.unwrap();
            scheduler_controller.enqueue(response.job).await;

            task_counter.fetch_sub(1, Ordering::Relaxed);
        });
    }

    async fn background_pend(&self, job: Job) {
        let handle = self.pending_manager_controller.register(job).await;

        let scheduler_controller = self.scheduler_controller.clone();
        let task_counter = self.task_counter.clone();

        tokio::spawn(async move {
            task_counter.fetch_add(1, Ordering::Relaxed);

            let response = handle.response_receiver.await.unwrap();
            scheduler_controller.enqueue(response.job).await;

            task_counter.fetch_sub(1, Ordering::Relaxed);
        });
    }
}

/// Api
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

pub enum Command {
    Enqueue(enqueue::Request),
    NoJob,
    // Assigned(assigned::Request),
    // Pending(pending::Request),
    Shutdown,
}

pub mod enqueue {
    use super::*;

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
