use cpu_time::ThreadTime;
use std::ops::{AddAssign, SubAssign};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::pleiades_type::{Job, JobStatus};
use crate::runtime::{JsRuntime, Runtime as _};
use crate::{pending_manager, scheduler};

/// Executor
///
///
///
///
pub struct Executor {
    /// interface to access this component
    ///
    command_receiver: mpsc::Receiver<Command>,

    /// updater
    ///
    // updater_controller: updater::Controller,
    scheduler_controller: scheduler::Controller,

    /// pending_manager
    ///
    pending_manager_controller: pending_manager::Controller,

    /// max_queueing_time
    ///
    max_queueing_time: Arc<Mutex<Duration>>,

    /// runtime
    ///
    /// Todo: replace with Runtime Trait
    runtime: JsRuntime,
}

impl Executor {
    /// new
    ///
    ///
    ///
    ///
    pub fn new(
        // updater_controller: updater::Controller,
        pending_manager_controller: pending_manager::Controller,
        scheduler_controller: scheduler::Controller,
    ) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(64);

        let max_queueing_time = Arc::new(Mutex::new(Duration::from_secs(0)));

        let data_manager = Self {
            command_receiver,
            // updater_controller,
            scheduler_controller,
            pending_manager_controller,
            max_queueing_time: max_queueing_time.clone(),
            runtime: JsRuntime::init(),
        };

        let controller = Controller {
            command_sender,
            max_queueing_time,
        };

        (data_manager, controller)
    }

    /// run
    ///
    ///
    ///
    ///
    pub fn run(&mut self) {
        while let Some(command) = self.command_receiver.blocking_recv() {
            match command {
                Command::Enqueue(request) => self.task_process_job(request),
            }
        }
    }

    fn task_process_job(&mut self, request: enqueue::Request) {
        let job_remaining_time = request.job.remaining_time;

        let mut processed_job = {
            // start measuring time
            let start = ThreadTime::now();

            // execute job with runtime
            let mut processed_job = self.runtime.process(request.job);

            // stop measuring time
            processed_job.sub_remaining_time(start.elapsed());

            processed_job
        };

        {
            // update max_queueing_time
            self.max_queueing_time
                .lock()
                .unwrap()
                .sub_assign(job_remaining_time);
        }

        match processed_job.status {
            JobStatus::Finished(_) | JobStatus::Cancelled => {
                // self.updater_controller.update_job_nowait(processed_job);
                self.scheduler_controller.enqueue_nowait(processed_job);
            }
            JobStatus::Pending(_) => {
                if processed_job.is_timeout() {
                    println!("Job is timeout");
                    processed_job.cancel();
                    // self.updater_controller.update_job_nowait(processed_job);
                    self.scheduler_controller.enqueue_nowait(processed_job);
                } else {
                    self.pending_manager_controller
                        .enqueue_nowait(processed_job);
                }
            }
            _ => {
                processed_job.cancel();
                // self.updater_controller.update_job_nowait(processed_job);
                self.scheduler_controller.enqueue_nowait(processed_job);
                unreachable!();
            }
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
    max_queueing_time: Arc<Mutex<Duration>>,
}

impl Controller {
    /// get_blob
    ///
    ///
    pub async fn enqueue(&self, job: Job) -> enqueue::Handle {
        {
            self.max_queueing_time
                .lock()
                .unwrap()
                .add_assign(job.remaining_time);
        }

        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();

        let request = Command::Enqueue(enqueue::Request {
            job,
            response_sender,
        });

        self.command_sender.send(request).await.unwrap();

        enqueue::Handle { response_receiver }
    }

    /// get_max_queueing_time
    ///
    ///
    pub fn max_queueing_time(&self) -> Duration {
        *self.max_queueing_time.lock().unwrap()
    }
}

/// Command
///
///
///
///
pub enum Command {
    Enqueue(enqueue::Request),
}

pub mod enqueue {
    use tokio::sync::oneshot;

    use super::*;

    pub struct Handle {
        pub response_receiver: oneshot::Receiver<Response>,
    }

    pub struct Request {
        pub response_sender: oneshot::Sender<Response>,
        pub job: Job,
    }

    pub struct Response {
        pub job: Job,
    }
}
