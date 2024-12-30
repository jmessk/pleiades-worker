use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, Semaphore};

use crate::{
    contractor, executor,
    helper::WorkerIdManager,
    pending_manager,
    pleiades_type::{Job, JobStatus},
    updater,
};

/// Controllers
///
///
///
///
#[derive(Clone)]
pub struct Controllers {
    pub contractor: contractor::Controller,
    pub updater: updater::Controller,
    pub pending: pending_manager::Controller,
}

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
pub struct LocalScheduler {
    id: usize,
    command_receiver: mpsc::Receiver<Command>,
    semaphore: Arc<Semaphore>,

    executor_deadline: Duration,
    contracting: Duration,
    pending: Duration,

    controllers: Controllers,
    sched_controller: SchedController,
    exec_controller: executor::Controller,
    worker_id_manager: WorkerIdManager,
}

impl LocalScheduler {
    const MAX_CONCURRENCY: usize = 128;
    /// new
    ///
    ///
    pub fn new(
        id: usize,
        controllers: Controllers,
        executor_controller: executor::Controller,
        worker_id_manager: WorkerIdManager,
        executor_deadline: Duration,
    ) -> (Self, SchedController) {
        let (command_sender, command_receiver) = mpsc::channel(64);

        let sched_controller = SchedController { command_sender };

        let scheduler = Self {
            id,
            command_receiver,
            semaphore: Arc::new(Semaphore::new(Self::MAX_CONCURRENCY)),
            executor_deadline,
            contracting: Duration::ZERO,
            pending: Duration::ZERO,
            controllers,
            sched_controller: sched_controller.clone(),
            exec_controller: executor_controller,
            worker_id_manager,
        };

        (scheduler, sched_controller)
    }

    /// run
    ///
    ///
    pub async fn run(&mut self, policy: Policy) {
        tracing::info!("Scheduler {}: running", self.id);

        match policy {
            Policy::CooperativePipeline => self.cooperative_pipeline().await,
        }

        tracing::info!("Scheduler {}: shutdown", self.id);
    }

    /// wait_for_shutdown
    ///
    ///
    async fn schedule_shutdown(&self) {
        let scheduler_controller = self.sched_controller.clone();
        let semaphore = self.semaphore.clone();

        tokio::spawn(async move {
            let _ = semaphore.acquire_many(Self::MAX_CONCURRENCY as u32).await;
            scheduler_controller.signal_shutdown_done().await;
        });

        tracing::info!("Scheduler {}: scheduled shutdown", self.id);
    }

    /// contract_background
    ///
    ///
    async fn bg_contract(&self, worker_id: &str) {
        let handle = self
            .controllers
            .contractor
            .try_contract(worker_id.to_string())
            .await
            .unwrap();

        let scheduler_controller = self.sched_controller.clone();
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        tokio::spawn(async move {
            let response = handle.recv().await;
            match response.contracted {
                Some(job) => scheduler_controller.enqueue(job).await,
                None => scheduler_controller.signal_no_job().await,
            }

            drop(permit);
        });
    }

    async fn enqueue_execute(&self, job: Job) {
        let handle = self.exec_controller.enqueue(job).await;
        let scheduler_controller = self.sched_controller.clone();
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        let id = self.id;

        tokio::spawn(async move {
            let response = handle.recv().await;
            scheduler_controller.enqueue(response.job).await;
            tracing::debug!("Scheduler {}: enqueued job to executor", id);

            drop(permit);
        });
    }

    async fn enqueue_pend(&self, job: Job) {
        let handle = self.controllers.pending.register(job).await;
        let scheduler_controller = self.sched_controller.clone();
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        tokio::spawn(async move {
            let response = handle.response_receiver.await.unwrap();
            scheduler_controller.enqueue(response.job).await;

            drop(permit);
        });
    }

    fn capacity(&self) -> Duration {
        let max = self.executor_deadline;
        let queuing = self.exec_controller.max_queueing_time();
        let pending = self.pending;
        let contracting = self.contracting;

        // dbg!(max, queuing, pending, contracting);
        if max < queuing + pending + contracting {
            println!("over capacity");
        }

        max.checked_sub(queuing + pending + contracting)
            .unwrap_or(Duration::ZERO)
    }

    async fn contract(&mut self, job_deadline: Duration, worker_id: &str) {
        let capacity = self.capacity();
        let available_jobs = capacity.div_duration_f32(job_deadline) as usize;

        tracing::debug!(
            "Scheduler {}: capacity: {capacity:?}, available_jobs: {available_jobs}",
            self.id,
        );

        for _ in 0..available_jobs {
            self.add_contracting(job_deadline);
            self.bg_contract(worker_id).await;
        }
    }

    fn add_contracting(&mut self, duration: Duration) {
        self.contracting += duration;
    }

    fn sub_contracting(&mut self, duration: Duration) {
        self.contracting = self.contracting.checked_sub(duration).unwrap();
    }

    fn add_pending(&mut self, duration: Duration) {
        self.pending += duration;
    }

    fn sub_pending(&mut self, duration: Duration) {
        self.pending = self.pending.checked_sub(duration).unwrap();
    }
}

impl LocalScheduler {
    /// cooperative_pipeline
    ///
    ///
    ///
    ///
    async fn cooperative_pipeline(&mut self) {
        let mut shutdown_flag = false;
        let (default_worker_id, default_job_deadline) = self.worker_id_manager.get_default();

        self.contract(default_job_deadline, &default_worker_id)
            .await;

        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Command::Enqueue(enqueue::Request { job }) => match job.status {
                    JobStatus::Assigned => {
                        self.sub_contracting(default_job_deadline);
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
                    JobStatus::Finished(_) => self.controllers.updater.update_job(job).await,
                    JobStatus::Cancelled => self.controllers.updater.update_job(job).await,
                    _ => unreachable!(),
                },
                Command::NoJob => self.sub_contracting(default_job_deadline),
                Command::ShutdownReq => {
                    shutdown_flag = true;
                    self.schedule_shutdown().await;
                }
                Command::ShutdownDone => break,
            }

            if !shutdown_flag {
                self.contract(default_job_deadline, &default_worker_id)
                    .await;
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
pub struct SchedController {
    command_sender: mpsc::Sender<Command>,
}

impl SchedController {
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

    pub async fn signal_shutdown_req(&self) {
        let request = Command::ShutdownReq;
        self.command_sender.send(request).await.unwrap();
    }

    pub async fn signal_shutdown_done(&self) {
        let request = Command::ShutdownDone;
        self.command_sender.send(request).await.unwrap();
    }
}

#[derive(Debug)]
pub enum Command {
    Enqueue(enqueue::Request),
    NoJob,
    ShutdownReq,
    ShutdownDone,
}

pub mod enqueue {
    use super::*;

    #[derive(Debug)]
    pub struct Request {
        pub job: Job,
    }
}
