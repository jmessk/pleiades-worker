use std::time::{Duration, Instant};

// use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{
    helper::LocalSchedManager,
    pleiades_type::{Blob, Job, JobStatus, Lambda},
    WorkerConfig,
};

// use super::local_sched::Policy;

/// Scheduler
///
///
///
///
pub struct GlobalSched {
    command_receiver: mpsc::Receiver<Command>,
    // semaphore: Arc<Semaphore>,

    // action_receiver: watch::Receiver<()>,

    // contracting: Duration,
    global_sched: Controller,
    // contractor: contractor::Controller,
    local_sched_manager: LocalSchedManager,
    // worker_id_manager: WorkerIdManager,
    // policy: Policy,
    code: bytes::Bytes,

    config: WorkerConfig,
}

impl GlobalSched {
    // const MAX_CONCURRENCY: usize = 128;

    /// new
    ///
    ///
    pub fn new(
        // contractor_controller: contractor::Controller,
        local_sched_manager: LocalSchedManager,
        // worker_id_manager: WorkerIdManager,
        // action_receiver: watch::Receiver<()>,
        // policy: Policy,
        code: bytes::Bytes,
        config: WorkerConfig,
    ) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(128);

        let controller = Controller { command_sender };

        let global_sched = Self {
            command_receiver,
            // semaphore: Arc::new(Semaphore::new(Self::MAX_CONCURRENCY)),
            // action_receiver,
            // contracting: Duration::ZERO,
            global_sched: controller.clone(),
            // contractor: contractor_controller,
            local_sched_manager,
            // worker_id_manager,
            // policy,
            code,
            config,
        };

        (global_sched, controller)
    }

    /// run
    ///
    ///
    pub async fn run(&mut self) {
        tracing::info!("running");

        // let global_sched = self.global_sched.clone();
        // tokio::spawn(Self::local_action_receiver(
        //     self.action_receiver.clone(),
        //     global_sched,
        // ));

        match self.config.policy.as_str() {
            "blocking" => self.blocking().await,
            "cooperative" => self.cooperative().await,
            _ => unreachable!(),
        }

        tracing::info!("shutdown");
    }

    // async fn local_action_receiver(
    //     mut action_receiver: watch::Receiver<()>,
    //     global_sched: Controller,
    // ) {
    //     while action_receiver.changed().await.is_ok() {
    //         global_sched.signal_local_action().await;
    //         tokio::time::sleep(Duration::from_millis(200)).await;
    //     }
    // }

    // /// wait_for_shutdown
    // ///
    // ///
    async fn schedule_shutdown(&self) {
        let scheduler_controller = self.global_sched.clone();
        // println!("signal_shutdown {}", semaphore.available_permits());

        tokio::spawn(async move {
            // let _ = semaphore.acquire_many(Self::MAX_CONCURRENCY as u32).await;
            loop {
                if scheduler_controller.command_sender.capacity()
                    == scheduler_controller.command_sender.max_capacity()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            scheduler_controller.signal_shutdown_done().await;
        });

        tracing::info!("scheduled shutdown");
    }

    // /// schedule_contract
    // ///
    // ///
    // async fn schedule_contract(&self, groupe: &str, worker_id: &str) {
    // let global_sched = self.global_sched.clone();
    // let permit = self.semaphore.clone().acquire_owned().await.unwrap();

    // let handle = self
    //     .contractor
    //     .try_contract(groupe.to_string(), worker_id.to_string())
    //     .await
    //     .unwrap();

    // tokio::spawn(async move {
    //     let response = handle.recv().await;

    //     match response.contracted {
    //         Some(job) => global_sched.enqueue_job(job, response.groupe).await,
    //         None => global_sched.signal_no_job().await,
    //     }

    //     drop(permit);
    // });
    // }

    // async fn contract_up_to_deadline(&mut self, job_deadline: Duration, worker_id: &str) {
    //     let max = self.local_sched_manager.deadline_sum;
    //     let local_sched_used = self.local_sched_manager.used_sum();
    //     let contracting = self.contracting;

    //     let capacity = max.checked_sub(local_sched_used + contracting).unwrap();
    //     let available_jobs = capacity.div_duration_f32(job_deadline) as usize;

    //     let max = std::cmp::min(
    //         available_jobs,
    //         self.contractor.semaphore.available_permits(),
    //     );

    //     tracing::debug!("capacity: {capacity:?}, available_jobs: {available_jobs}, max: {max}",);

    //     // let (worker_id, job_deadline) = self.worker_id_manager.get_default();
    //     // static COUNTER: AtomicUsize = AtomicUsize::new(1);

    //     for _ in 0..max {
    //         tokio::time::sleep(Duration::from_millis(10)).await;

    //         // edit
    //         //
    //         // let i = COUNTER.load(std::sync::atomic::Ordering::SeqCst);
    //         // let groupe = format!("test{i}");
    //         // let (worker_id, job_deadline) = self.worker_id_manager.get(&groupe).unwrap();
    //         // COUNTER.store(i % 6 + 1, std::sync::atomic::Ordering::SeqCst);
    //         //
    //         // /////

    //         self.add_contracting(job_deadline);
    //         self.schedule_contract("", &worker_id).await;
    //         // self.schedule_contract("default", &worker_id).await;
    //     }
    // }

    // fn add_contracting(&mut self, duration: Duration) {
    //     self.contracting += duration;
    // }

    // fn sub_contracting(&mut self, duration: Duration) {
    //     self.contracting = self.contracting.checked_sub(duration).unwrap();
    // }
}

impl GlobalSched {
    //     async fn blocking(&mut self) {
    // let (default_worker_id, default_job_deadline) = self.worker_id_manager.get_default();

    // self.contract_up_to_deadline(default_job_deadline, &default_worker_id)
    //     .await;

    // let controller = self.global_sched.clone();
    // let contract = tokio::spawn(async move {
    //     loop {
    //         controller.signal_local_action().await;
    //         tokio::time::sleep(Duration::from_millis(200)).await;
    //     }
    // });

    //     while let Some(command) = self.command_receiver.recv().await {
    //         match command {
    //             Command::Contracted { job, groupe: _ } => {
    //                 // self.local_sched_manager.view();
    //                 let local_sched = self.local_sched_manager.shortest();

    //                 local_sched.assign(job).await;
    //                 tracing::debug!("assigned job to LocalSched: {}", local_sched.id);
    //                 // self.sub_contracting(default_job_deadline);
    //             }
    //             // Command::NoJob => self.sub_contracting(default_job_deadline),
    //             // Command::LocalAction => {
    //             //     self.contract_up_to_deadline(default_job_deadline, &default_worker_id)
    //             //         .await
    //             // }
    //             Command::ShutdownReq => {
    //                 self.schedule_shutdown().await;
    //                 // contract.abort();
    //             }
    //             Command::ShutdownDone => {
    //                 self.local_sched_manager.signal_shutdown_req().await;
    //                 break;
    //             }
    //             _ => {}
    //         }
    //     }
    // }

    fn start_job_generator(&self) -> tokio::task::JoinHandle<()> {
        let controller = self.global_sched.clone();
        let code = self.code.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            // warmup
            let warmup_rps = Duration::from_secs_f32(1.0 / config.warmup.rps as f32);
            let mut warmup_ticker = tokio::time::interval(warmup_rps);
            let warmup_time = config.warmup.time;

            println!("warmup for {:?}", warmup_time);
            warmup_ticker.tick().await;
            let stop = tokio::time::Instant::now() + warmup_time;
            while tokio::select! {
                _ = warmup_ticker.tick() => true,
                _ = tokio::time::sleep_until(stop) => false,
            } {
                let job = gen_job((-1).to_string(), code.clone()).await;
                controller.enqueue_job(job).await;
            }

            // measurement
            let mut current_rps = config.measure.start_rps;
            let final_rps = config.measure.final_rps;
            let steps = config.measure.steps;
            let step_time = config.measure.step_time;
            let step_rps = if 1 < steps {
                (final_rps - config.measure.start_rps) / (steps - 1)
            } else {
                0
            };
            let mut count = 0;

            println!("measure for {:?}", step_time * steps);

            for step in 1..=steps {
                let interval = Duration::from_secs_f32(1.0 / current_rps as f32);
                let mut ticker = tokio::time::interval(interval);
                let stop = tokio::time::Instant::now() + step_time;

                println!(
                    "step {step}: current_rps: {current_rps}, interval: {:?}",
                    interval
                );

                ticker.tick().await;
                while tokio::select! {
                    _ = ticker.tick() => true,
                    _ = tokio::time::sleep_until(stop) => false,
                } {
                    let job = gen_job(count.to_string(), code.clone()).await;
                    controller.enqueue_job(job).await;

                    count += 1;
                }

                current_rps += step_rps;
            }
        })
    }

    async fn blocking(&mut self) {
        let job_generator = self.start_job_generator();

        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Command::Contracted { job } => {
                    // self.local_sched_manager.view();
                    // let mut has_wait = false;
                    let local_sched = loop {
                        if let Some(sched) = self.local_sched_manager.no_jobs() {
                            break sched;
                        }
                        // tracing::warn!("all LocalScheds are busy");
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    };

                    local_sched.assign(job).await;
                    tracing::debug!("assigned job to LocalSched: {}", local_sched.id);
                }
                Command::ShutdownReq => {
                    self.schedule_shutdown().await;
                    job_generator.abort();
                }
                Command::ShutdownDone => {
                    self.local_sched_manager.signal_shutdown_req().await;
                    break;
                }
                _ => {}
            }
        }
    }

    /// cooperative_pipeline
    ///
    ///
    ///
    ///
    async fn cooperative(&mut self) {
        // let (default_worker_id, default_job_deadline) = self.worker_id_manager.get_default();

        // self.contract_up_to_deadline(default_job_deadline, &default_worker_id)
        //     .await;

        // let contract = tokio::spawn(async move {
        //     loop {
        //         controller.signal_local_action().await;
        //         tokio::time::sleep(Duration::from_millis(200)).await;
        //     }
        // });
        let job_generator = self.start_job_generator();

        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Command::Contracted { job } => {
                    // self.local_sched_manager.view();
                    // let local_sched = self.local_sched_manager.shortest();
                    // edit
                    // match groupe.as_str() {
                    //     "default" => self.local_sched_manager.shortest().assign(job).await,
                    //     "test1" | "test2" | "test3" | "test6" => {
                    //         let local_sched = self.local_sched_manager.shortest_cpu();
                    //         local_sched.assign(job).await;
                    //         local_sched.increment_cpu_jobs();
                    //         tracing::debug!("assigned job to LocalSched: {}", local_sched.id);
                    //     }
                    //     "test4" | "test5" => {
                    //         let local_sched = self.local_sched_manager.shortest_gpu();
                    //         local_sched.assign(job).await;
                    //         local_sched.increment_gpu_jobs();
                    //         tracing::debug!("assigned job to LocalSched: {}", local_sched.id);
                    //     }
                    //     _ => {}
                    // };
                    // /////

                    // // local_sched.assign(job).await;
                    // tracing::debug!("assigned job to LocalSched: {}", local_sched.id);
                    // self.sub_contracting(default_job_deadline);
                    // self.local_sched_manager.view();
                    let mut local_sched = self.local_sched_manager.shortest();

                    // let mut has_wait = false;
                    while local_sched.is_overloaded() {
                        // has_wait = true;
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        local_sched = self.local_sched_manager.shortest();
                    }

                    // tracing::warn!("all LocalScheds are busy");

                    local_sched.assign(job).await;
                    tracing::debug!("assigned job to LocalSched: {}", local_sched.id);
                }
                // Command::NoJob => self.sub_contracting(default_job_deadline),
                // Command::LocalAction => {
                //     self.contract_up_to_deadline(default_job_deadline, &default_worker_id)
                //         .await
                // }
                Command::ShutdownReq => {
                    self.schedule_shutdown().await;
                    job_generator.abort();
                }
                Command::ShutdownDone => {
                    self.local_sched_manager.signal_shutdown_req().await;
                    break;
                }
                _ => {}
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
        let _ = self.command_sender.send(Command::Contracted { job }).await;
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
    Contracted { job: Job },
    NoJob,
    LocalAction,
    ShutdownReq,
    ShutdownDone,
}

// ACDSA
async fn gen_job(job_id: String, code: bytes::Bytes) -> Job {
    Job {
        id: job_id,
        status: JobStatus::Assigned,
        consumed: Duration::ZERO,
        remaining: Duration::from_secs(300),
        context: None,
        lambda: Lambda {
            code: Blob { data: code },
        },
        input: Blob {
            data: bytes::Bytes::default(),
        },

        contracted_at: Instant::now(),
    }
}
