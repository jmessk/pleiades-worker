pub mod policy;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{sync::mpsc, task::JoinSet};

use crate::{
    contractor::{self, contract},
    data_manager, executor,
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

    /// scheduler_controller
    ///
    scheduler_controller: Controller,

    /// contractor_controller
    ///
    contractor_controller: contractor::Controller,

    updater_controller: updater::Controller,

    /// executor_controller
    ///
    // executor_list: Vec<(executor::Controller, Duration)>,
    executor_manager: ExecutorManager,
}

impl Scheduler {
    /// new
    ///
    ///
    pub fn new(
        contractor_controller: contractor::Controller,
        updater_controller: updater::Controller,
        executor_manager: ExecutorManager,
    ) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(128);

        let controller = Controller { command_sender };

        let updater = Self {
            command_receiver,
            task_counter: Arc::new(AtomicUsize::new(0)),
            scheduler_controller: controller.clone(),
            contractor_controller,
            updater_controller,
            executor_manager,
        };

        (updater, controller)
    }

    /// run
    ///
    pub async fn run(&mut self) {
        // self.algorithm_1().await;

        self.wait_for_shutdown().await;
    }

    async fn algorithm_1(&mut self) {
        while let Some(Command::Enqueue(enqueue::Request { job })) =
            self.command_receiver.recv().await
        {
            match job.status {
                JobStatus::Assigned | JobStatus::Ready(_) => {
                    let executor = self.executor_manager.current_shortest();
                    executor.enqueue(job).await;
                }
                JobStatus::Finished(_) => {
                    self.updater_controller.update_job(job).await;
                    println!("Job is finished");
                }
                JobStatus::Cancelled => {
                    self.updater_controller.update_job(job).await;
                    println!("Job is cancelled");
                }
                _ => {
                    unreachable!()
                }
            }
        }
    }

    async fn algorithm_2(&mut self) {
        // let mut job_buf: Vec<Command> = Vec::with_capacity(64);
        // let job_buf_capacity = job_buf.capacity();

        loop {
            // 1. get jobs in the scheduler queue
            // let num_jobs = self
            //     .command_receiver
            //     .blocking_recv_many(&mut job_buf, job_buf_capacity);

            // 2. get current max queueing time of the executor

            // next, if jobs are in the queue, determine which executor to use

            todo!()
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
    async fn task_contract_background(handle: contract::Handle, scheduler_controller: Controller) {
        let response = handle.recv().await;

        if let Some(job) = response.contracted {
            scheduler_controller.enqueue(job).await
        }
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

pub mod contract_join_set {
    use super::*;

    struct ContractJoinSet {
        scheduler_controller: Controller,
    }

    impl ContractJoinSet {
        pub fn new(scheduler_controller: Controller) -> Self {
            Self {
                scheduler_controller,
            }
        }

        // pub fn run(&self) {
        //     let contract_join_set = JoinSet::new();

        //     while let Some(command) = self.command_receiver.recv().await {
        //         contract_join_set.spawn(async move {})
        //     }
        // }
    }

    pub struct Request {
        pub job: Job,
    }

    pub struct Response {
        pub job: Job,
    }
}

#[derive(Default)]
struct ExecutorManager {
    list: Vec<(executor::Controller, Duration)>,
}

impl ExecutorManager {
    pub fn builder() -> ExecutorManagerBuilder {
        ExecutorManagerBuilder::new()
    }

    pub fn insert(&mut self, controller: executor::Controller) {
        self.list.push((controller, Duration::from_secs(0)));
    }

    pub fn current_shortest(&self) -> &executor::Controller {
        let executor_controller = self
            .list
            .iter()
            .min_by_key(|(c, _)| c.max_queueing_time())
            .unwrap();
        &executor_controller.0
    }

    pub fn estimated_shortest(&self) -> &executor::Controller {
        let executor_controller = self
            .list
            .iter()
            .min_by_key(|(_, duration)| duration)
            .unwrap();
        &executor_controller.0
    }

    pub fn update_queuing_time(&mut self) {
        self.list
            .iter_mut()
            .for_each(|(c, d)| *d = c.max_queueing_time());
    }
}

pub struct ExecutorManagerBuilder {
    list: Vec<executor::Controller>,
}

impl ExecutorManagerBuilder {
    pub fn new() -> Self {
        Self { list: Vec::new() }
    }

    pub fn insert(&mut self, controller: executor::Controller) {
        self.list.push(controller);
    }

    pub fn build(self) -> anyhow::Result<ExecutorManager> {
        if self.list.is_empty() {
            anyhow::bail!("ExecutorManagerBuilder: no executor controller is provided");
        }

        let list = self
            .list
            .into_iter()
            .map(|c| (c, Duration::from_secs(0)))
            .collect();

        Ok(ExecutorManager { list })
    }
}
