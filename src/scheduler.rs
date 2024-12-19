pub mod policy;

use boa_engine::job;
use pleiades_api::api::worker;
use std::{
    collections::HashMap,
    default,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::sync::mpsc;

use crate::{
    contractor::{self, contract},
    helper::{ExecutorManager, WorkerIdManager},
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

    /// executor_controller
    ///
    executor_manager: Option<ExecutorManager>,
}

impl Scheduler {
    /// new
    ///
    ///
    pub fn new(
        worker_id_manager: WorkerIdManager,
        contractor_controller: contractor::Controller,
        updater_controller: updater::Controller,
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
            executor_manager: None,
        };

        (updater, controller)
    }

    /// set_controller
    pub fn set_controller(&mut self, executor_manager: ExecutorManager) {
        self.executor_manager = Some(executor_manager);
    }

    /// run
    ///
    ///
    pub async fn run(&mut self) {
        let command_receiver = &mut self.command_receiver;
        let worker_id_manager = &mut self.worker_id_manager;
        let scheduler_controller = &mut self.scheduler_controller;
        let contractor_controller = &mut self.contractor_controller;

        let executor_manager = self
            .executor_manager
            .as_mut()
            .expect("ExecutorManager is not set");

        let updater_controller = &mut self.updater_controller;

        Self::default_policy(
            command_receiver,
            worker_id_manager,
            scheduler_controller,
            contractor_controller,
            updater_controller,
            executor_manager,
        )
        .await;

        self.wait_for_shutdown().await;
    }

    async fn default_policy(
        command_receiver: &mut mpsc::Receiver<Command>,
        worker_id_manager: &mut WorkerIdManager,
        scheduler_controller: &mut Controller,
        contractor_controller: &mut contractor::Controller,
        updater_controller: &mut updater::Controller,
        executor_manager: &mut ExecutorManager,
    ) {
        let default_worker_id = worker_id_manager.get("default").unwrap();

        for _ in 0..executor_manager.num_executors() {
            Self::contract_background(
                &default_worker_id,
                contractor_controller,
                scheduler_controller,
            )
            .await;
        }

        while let Some(command) = command_receiver.recv().await {
            match command {
                Command::Enqueue(enqueue::Request { job }) => match job.status {
                    JobStatus::Assigned | JobStatus::Ready(_) => {
                        let executor = executor_manager.current_shortest();
                        executor.enqueue(job).await;
                    }
                    JobStatus::Finished(_) => {
                        updater_controller.update_job(job).await;
                        println!("Job is finished");
                    }
                    JobStatus::Cancelled => {
                        updater_controller.update_job(job).await;
                        println!("Job is cancelled");
                    }
                    _ => unreachable!(),
                },
                Command::NoJob => {}
            }


        }
    }

    /// wait_for_shutdown
    ///
    async fn wait_for_shutdown(&self) {
        println!("Scheduler is shutting down");

        while self.task_counter.load(Ordering::Relaxed) > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        println!("Scheduler is shut down");
    }
}

/// tasks
///
///
impl Scheduler {
    /// task_contract_background
    ///
    ///
    async fn contract_background(
        worker_id: &str,
        contractor_controller: &contractor::Controller,
        scheduler_controller: &Controller,
    ) {
        let handle = contractor_controller
            .try_contract(worker_id.to_string())
            .await
            .unwrap();
        let scheduler_controller = scheduler_controller.clone();

        tokio::spawn(async move {
            let response = handle.recv().await;
            if let Some(job) = response.contracted {
                scheduler_controller.enqueue(job).await;
            }
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
