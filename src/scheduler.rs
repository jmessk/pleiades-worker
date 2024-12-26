use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, Semaphore};

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

pub enum Policy {
    FastContract,
    BlockingPipeline(Duration),
    CooperativePipeline(Duration),
}

/// Scheduler
///
///
///
///
pub struct Scheduler {
    command_receiver: mpsc::Receiver<Command>,
    max_concurrency: usize,
    semaphore: Arc<Semaphore>,

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
        let (command_sender, command_receiver) = mpsc::channel(256);

        let controller = Controller { command_sender };

        let scheduler = Self {
            command_receiver,
            max_concurrency: 256,
            semaphore: Arc::new(Semaphore::new(256)),
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
    pub async fn run(&mut self, policy: Policy) {
        tracing::info!("running");

        match policy {
            Policy::FastContract => self.fast_contract_policy().await,
            Policy::BlockingPipeline(job_deadline) => self.blocking_pipeline(job_deadline).await,
            Policy::CooperativePipeline(job_deadline) => {
                self.cooperative_pipeline(job_deadline).await
            }
        }

        tracing::info!("shutdown");
    }

    /// wait_for_shutdown
    ///
    ///
    async fn schedule_shutdown(&self) {
        let scheduler_controller = self.controllers.scheduler.clone();
        let semaphore = self.semaphore.clone();
        let max_concurrency = self.max_concurrency;

        tokio::spawn(async move {
            let _ = semaphore.acquire_many(max_concurrency as u32).await;
            scheduler_controller.signal_shutdown_done().await;
        });

        tracing::info!("scheduled shutdown");
    }

    /// contract_background
    ///
    ///
    async fn back_contract(&self, worker_id: &str) {
        let handle = self
            .controllers
            .contractor
            .try_contract(worker_id.to_string())
            .await
            .unwrap();

        let scheduler_controller = self.controllers.scheduler.clone();
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

    async fn enqueue_execute(&self, job: Job, executor_controller: &executor::Controller) {
        let handle = executor_controller.enqueue(job).await;
        let scheduler_controller = self.controllers.scheduler.clone();
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        let id = executor_controller.id;

        tokio::spawn(async move {
            let response = handle.response_receiver.await.unwrap();
            scheduler_controller.enqueue(response.job).await;
            tracing::debug!("enqueued job to executor {:?}", id);

            drop(permit);
        });
    }

    async fn enqueue_pend(&self, job: Job) {
        let handle = self.controllers.pending.register(job).await;
        let scheduler_controller = self.controllers.scheduler.clone();
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        tokio::spawn(async move {
            let response = handle.response_receiver.await.unwrap();
            scheduler_controller.enqueue(response.job).await;

            drop(permit);
        });
    }

    async fn contract(&mut self, job_deadline: Duration, worker_id: &str) {
        let deadline_sum = self.executor_manager.deadline_sum;
        let queuing_time_sum = self.executor_manager.queuing_time_sum();
        let pending = self.pending;
        let contracting = self.contracting;

        let capacity_sum = deadline_sum - (queuing_time_sum + pending + contracting);

        let available_jobs = std::cmp::min(
            capacity_sum.div_duration_f32(job_deadline) as usize,
            self.controllers.contractor.max_concurrency,
        );

        tracing::debug!("capacity_sum: {capacity_sum:?}, available_jobs {available_jobs}");
        for _ in 0..available_jobs {
            self.add_contracting(job_deadline);
            self.back_contract(worker_id).await;
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

impl Scheduler {
    /// fast_contract_policy
    ///
    ///
    ///
    ///
    async fn fast_contract_policy(&mut self) {
        let mut shutdown_flag = false;

        let default_worker_id = self.worker_id_manager.get_default();

        for _ in 0..self.controllers.contractor.max_concurrency {
            self.back_contract(&default_worker_id).await;
        }

        while let Some(command) = self.command_receiver.recv().await {
            // println!("------command: {:?}", command);
            match command {
                Command::Enqueue(enqueue::Request { job }) => match job.status {
                    JobStatus::Assigned | JobStatus::Ready(_) => {
                        let executor = self.executor_manager.shortest();
                        self.enqueue_execute(job, executor).await;
                        // println!("Enqueued job to Executor {:?}", executor.id);
                    }
                    JobStatus::Finished(_) => {
                        // println!("Job is finished");
                        // println!("Job remaining time: {:?}", job.remaining_time);
                        self.controllers.updater.update_job(job).await;

                        if shutdown_flag {
                            continue;
                        }

                        self.back_contract(&default_worker_id).await;
                    }
                    JobStatus::Pending(_) => {
                        // println!("Job is pending");
                        self.enqueue_pend(job).await;
                    }
                    JobStatus::Cancelled => {
                        // println!("Job is cancelled");
                        self.controllers.updater.update_job(job).await;

                        if shutdown_flag {
                            continue;
                        }

                        self.back_contract(&default_worker_id).await;
                    }
                    _ => unreachable!(),
                },
                Command::NoJob => {
                    // println!("No job is available");

                    if shutdown_flag {
                        continue;
                    }

                    self.back_contract(&default_worker_id).await;
                }
                Command::ShutdownReq => {
                    self.schedule_shutdown().await;
                    shutdown_flag = true;
                }
                Command::ShutdownDone => break,
            }
        }
    }

    /// blocking_pipeline
    ///
    ///
    ///
    ///
    async fn blocking_pipeline(&mut self, job_deadline: Duration) {
        let mut shutdown_flag = false;
        let default_worker_id = self.worker_id_manager.get_default();

        self.contract(job_deadline, &default_worker_id).await;

        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Command::Enqueue(enqueue::Request { job }) => match job.status {
                    JobStatus::Assigned => {
                        self.sub_contracting(job_deadline);
                        let executor = self.executor_manager.shortest();
                        self.enqueue_execute(job, executor).await;
                    }
                    JobStatus::Ready(_) => {}
                    JobStatus::Pending(_) => {
                        let handle = self.controllers.pending.register(job).await;
                        let job = handle.response_receiver.await.unwrap().job;

                        println!("Job is pending");

                        let executor_controller = self.executor_manager.shortest();
                        self.enqueue_execute(job, executor_controller).await;
                    }
                    JobStatus::Finished(_) => self.controllers.updater.update_job(job).await,
                    JobStatus::Cancelled => self.controllers.updater.update_job(job).await,
                    _ => unreachable!(),
                },
                Command::NoJob => self.sub_contracting(job_deadline),
                Command::ShutdownReq => {
                    shutdown_flag = true;
                    self.schedule_shutdown().await;
                }
                Command::ShutdownDone => break,
            }

            if shutdown_flag {
                continue;
            }

            if self.contracting.is_zero() {
                self.contract(job_deadline, &default_worker_id).await;
            }
        }
    }

    /// cooperative_pipeline
    ///
    ///
    ///
    ///
    async fn cooperative_pipeline(&mut self, job_deadline: Duration) {
        let mut shutdown_flag = false;
        let default_worker_id = self.worker_id_manager.get_default();

        self.contract(job_deadline, &default_worker_id).await;

        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Command::Enqueue(enqueue::Request { job }) => match job.status {
                    JobStatus::Assigned => {
                        self.sub_contracting(job_deadline);
                        let executor = self.executor_manager.shortest();
                        self.enqueue_execute(job, executor).await;
                    }
                    JobStatus::Ready(_) => {
                        self.sub_pending(job.rem_time);
                        let executor = self.executor_manager.shortest();
                        self.enqueue_execute(job, executor).await;
                    }
                    JobStatus::Pending(_) => {
                        self.add_pending(job.rem_time);
                        self.enqueue_pend(job).await;
                    }
                    JobStatus::Finished(_) => self.controllers.updater.update_job(job).await,
                    JobStatus::Cancelled => self.controllers.updater.update_job(job).await,
                    _ => unreachable!(),
                },
                Command::NoJob => self.sub_contracting(job_deadline),
                Command::ShutdownReq => {
                    shutdown_flag = true;
                    self.schedule_shutdown().await;
                }
                Command::ShutdownDone => break,
            }

            if shutdown_flag {
                continue;
            }

            if self.contracting.is_zero() {
                self.contract(job_deadline, &default_worker_id).await;
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
