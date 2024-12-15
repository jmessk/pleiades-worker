use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::{sync::mpsc, task::JoinSet};

use crate::{
    contractor::{self, contract},
    data_manager, executor,
    pleiades_type::{Job, JobStatus},
};

/// Scheduler
///
///
///
///
pub struct Scheduler {
    client: Arc<pleiades_api::Client>,

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

    /// executor_controller
    ///
    executor_controller_list: Vec<executor::Controller>,
}

impl Scheduler {
    /// new
    ///
    pub fn new(
        client: Arc<pleiades_api::Client>,
        contractor_controller: contractor::Controller,
        executor_controller_list: Vec<executor::Controller>,
    ) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(128);

        let controller = Controller { command_sender };

        let updater = Self {
            client,
            command_receiver,
            task_counter: Arc::new(AtomicUsize::new(0)),
            scheduler_controller: controller.clone(),
            contractor_controller,
            executor_controller_list,
        };

        (updater, controller)
    }

    /// run
    ///
    pub async fn run(&mut self) {
        let contract_join_set = JoinSet::new();

        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Command::Enqueue(enqueue::Request { job }) => {}
            }
        }
        
        self.wait_for_shutdown().await;
    }

    async fn run_join_set(&self, join_set: JoinSet) {
        join_set.await;
    }

    /// task
    ///
    async fn task_wait_contract(handle: contract::Handle, scheduler_controller: Controller) {
        let response = handle.recv().await;

        if let Some(job) = response.contracted {
            scheduler_controller.enqueue_global(job).await
        }
    }

    /// wait_for_shutdown
    ///
    pub async fn wait_for_shutdown(&self) {
        println!("Scheduler is shutting down");

        while self.task_counter.load(Ordering::Relaxed) > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        println!("Scheduler is shut down");
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
    /// get_blob
    ///
    pub async fn enqueue_global(&self, job: Job) {
        let request = Command::Enqueue(enqueue::Request { job });
        self.command_sender.send(request).await.unwrap();
    }
}

pub enum Command {
    Enqueue(enqueue::Request),
}

pub mod enqueue {
    use super::*;

    pub struct Request {
        pub job: Job,
    }
}

struct ContractJoinSet {
    scheduler_controller: Controller,
}

impl ContractJoinSet {
    pub fn run(&self) {
        let contract_join_set = JoinSet::new();

        while let Some(command) = self.command_receiver.recv().await {
            match command {
            }

            
        }
    }
}
