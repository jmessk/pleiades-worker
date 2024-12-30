use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, watch, Semaphore};

use crate::{contractor, helper::LocalSchedManager, pleiades_type::Job, WorkerIdManager};

/// Scheduler
///
///
///
///
pub struct GlobalSched {
    command_receiver: mpsc::Receiver<Command>,
    semaphore: Arc<Semaphore>,

    action_receiver: watch::Receiver<()>,

    contracting: Duration,

    global_sched: Controller,
    contractor: contractor::Controller,

    local_sched_manager: LocalSchedManager,
    worker_id_manager: WorkerIdManager,
}

impl GlobalSched {
    const MAX_CONCURRENCY: usize = 128;

    /// new
    ///
    ///
    pub fn new(
        contractor_controller: contractor::Controller,
        local_sched_manager: LocalSchedManager,
        worker_id_manager: WorkerIdManager,
        action_receiver: watch::Receiver<()>,
    ) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(64);

        let controller = Controller { command_sender };

        let global_sched = Self {
            command_receiver,
            semaphore: Arc::new(Semaphore::new(Self::MAX_CONCURRENCY)),
            action_receiver,
            contracting: Duration::ZERO,
            global_sched: controller.clone(),
            contractor: contractor_controller,
            local_sched_manager,
            worker_id_manager,
        };

        (global_sched, controller)
    }

    /// run
    ///
    ///
    pub async fn run(&mut self) {
        tracing::info!("running");

        let global_sched = self.global_sched.clone();
        tokio::spawn(Self::local_action_receiver(
            self.action_receiver.clone(),
            global_sched,
        ));

        self.sched_loop().await;

        tracing::info!("shutdown");
    }

    async fn local_action_receiver(
        mut action_receiver: watch::Receiver<()>,
        global_sched: Controller,
    ) {
        while action_receiver.changed().await.is_ok() {
            global_sched.signal_local_action().await;
        }
    }

    /// wait_for_shutdown
    ///
    ///
    async fn schedule_shutdown(&self) {
        let scheduler_controller = self.global_sched.clone();
        let semaphore = self.semaphore.clone();

        tokio::spawn(async move {
            let _ = semaphore.acquire_many(Self::MAX_CONCURRENCY as u32).await;
            scheduler_controller.signal_shutdown_done().await;
        });

        tracing::info!("scheduled shutdown");
    }

    /// schedule_contract
    ///
    ///
    async fn schedule_contract(&self, worker_id: &str) {
        let handle = self
            .contractor
            .try_contract(worker_id.to_string())
            .await
            .unwrap();

        let global_sched = self.global_sched.clone();
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        tokio::spawn(async move {
            let response = handle.recv().await;

            match response.contracted {
                Some(job) => global_sched.enqueue_job(job).await,
                None => global_sched.signal_no_job().await,
            }

            drop(permit);
        });
    }

    async fn contract_up_to_deadline(&mut self, job_deadline: Duration, worker_id: &str) {
        let max = self.local_sched_manager.deadline_sum;
        let local_sched_used = self.local_sched_manager.used_sum();
        let contracting = self.contracting;

        let capacity = max.checked_sub(local_sched_used + contracting).unwrap();
        let available_jobs = capacity.div_duration_f32(job_deadline) as usize;

        tracing::debug!(
            "capacity: {:?}, available_jobs: {}",
            capacity,
            available_jobs
        );

        for _ in 0..available_jobs {
            self.add_contracting(job_deadline);
            self.schedule_contract(worker_id).await;
        }
    }

    fn add_contracting(&mut self, duration: Duration) {
        self.contracting += duration;
    }

    fn sub_contracting(&mut self, duration: Duration) {
        self.contracting = self.contracting.checked_sub(duration).unwrap();
    }
}

impl GlobalSched {
    /// cooperative_pipeline
    ///
    ///
    ///
    ///
    async fn sched_loop(&mut self) {
        let (default_worker_id, default_job_deadline) = self.worker_id_manager.get_default();

        self.contract_up_to_deadline(default_job_deadline, &default_worker_id)
            .await;

        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Command::Contracted(job) => {
                    let local_sched = self.local_sched_manager.shortest();
                    local_sched.assign(job).await;
                    tracing::debug!("assigned job to LocalSched: {}", local_sched.id);
                    self.sub_contracting(default_job_deadline);
                }
                Command::NoJob => self.sub_contracting(default_job_deadline),
                Command::LocalAction => {
                    self.contract_up_to_deadline(default_job_deadline, &default_worker_id)
                        .await
                }
                Command::ShutdownReq => {
                    self.schedule_shutdown().await;
                }
                Command::ShutdownDone => {
                    self.local_sched_manager.signal_shutdown_req().await;
                    break;
                }
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
}

impl Controller {
    /// enqueue_ready
    ///
    pub async fn enqueue_job(&self, job: Job) {
        self.command_sender
            .send(Command::Contracted(job))
            .await
            .unwrap();
    }

    pub async fn signal_no_job(&self) {
        self.command_sender.send(Command::NoJob).await.unwrap();
    }

    pub async fn signal_local_action(&self) {
        self.command_sender
            .send(Command::LocalAction)
            .await
            .unwrap();
    }

    pub async fn signal_shutdown_req(&self) {
        self.command_sender
            .send(Command::ShutdownReq)
            .await
            .unwrap();
    }

    pub async fn signal_shutdown_done(&self) {
        self.command_sender
            .send(Command::ShutdownDone)
            .await
            .unwrap();
    }
}

#[derive(Debug)]
pub enum Command {
    Contracted(Job),
    NoJob,
    LocalAction,
    ShutdownReq,
    ShutdownDone,
}
