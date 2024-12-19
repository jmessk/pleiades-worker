use cpu_time::ThreadTime;
use std::ops::{AddAssign, SubAssign};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::pleiades_type::{Job, JobStatus};
use crate::runtime::{JsRuntime, Runtime as _, RuntimeContext};
use crate::scheduler;

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
    // scheduler_controller: Option<scheduler::Controller>,

    /// pending_manager
    ///
    // pending_manager_controller: Option<pending_manager::Controller>,

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
    pub fn new(// pending_manager_controller: pending_manager::Controller,
        // scheduler_controller: scheduler::Controller,
    ) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(64);

        let max_queueing_time = Arc::new(Mutex::new(Duration::from_secs(0)));

        let data_manager = Self {
            command_receiver,
            // scheduler_controller: None,
            // pending_manager_controller: None,
            max_queueing_time: max_queueing_time.clone(),
            runtime: JsRuntime::init(),
        };

        let controller = Controller {
            command_sender,
            max_queueing_time,
        };

        (data_manager, controller)
    }

    /// set_controller
    ///
    ///
    // pub fn set_controller(
    //     &mut self,
    //     scheduler_controller: scheduler::Controller,
    //     pending_manager_controller: pending_manager::Controller,
    // ) {
    //     self.scheduler_controller = Some(scheduler_controller);
    //     self.pending_manager_controller = Some(pending_manager_controller);
    // }

    /// run
    ///
    ///
    ///
    ///
    pub fn run(&mut self) {
        while let Some(command) = self.command_receiver.blocking_recv() {
            match command {
                Command::Register(request) => self.task_execute_job(request),
            }
        }
    }

    fn task_execute_job(&mut self, request: register::Request) {
        // let scheduler_controller = self
        //     .scheduler_controller
        //     .as_ref()
        //     .expect("SchedulerController is not set");

        // let pending_manager_controller = self
        //     .pending_manager_controller
        //     .as_ref()
        //     .expect("PendingManagerController is not set");

        let mut job = request.job;

        match job.status {
            JobStatus::Assigned => {
                if self
                    .runtime
                    .create_context(&job.lambda, &job.input)
                    .inspect_err(|e| println!("Executor: {}", e))
                    .is_err()
                {
                    println!("Executor: Job is cancelled");

                    job.cancel();
                    // scheduler_controller.enqueue_nowait(job);
                    request
                        .response_sender
                        .send(register::Response { job })
                        .unwrap();

                    return;
                }
            }
            JobStatus::Ready(response) => {
                let context = match job.context {
                    Some(RuntimeContext::JavaScript(context)) => context,
                    _ => unreachable!(),
                };

                self.runtime.set_context(context);
                self.runtime.set_runtime_response(response).unwrap();
            }
            _ => unreachable!(),
        }

        let mut job = Job {
            status: JobStatus::Running,
            context: None,
            ..job
        };

        let job_remaining_time = job.remaining_time;

        // execution
        //
        //
        let job_status = {
            // start measuring time
            let start = ThreadTime::now();

            // execute job with runtime
            let job_status = self.runtime.execute().unwrap();

            // stop measuring time
            job.sub_remaining_time(start.elapsed());

            job_status
        };

        {
            // update max_queueing_time
            self.max_queueing_time
                .lock()
                .unwrap()
                .sub_assign(job_remaining_time);
        }

        let context = match self.runtime.get_context() {
            Some(context) => context,
            None => unreachable!(),
        };

        let job = Job {
            status: job_status,
            context: Some(RuntimeContext::JavaScript(context)),
            ..job
        };

        request
            .response_sender
            .send(register::Response { job })
            .unwrap();

        // self.scheduler_controller
        //     .as_mut()
        //     .unwrap()
        //     .enqueue_nowait(job);

        // match job_status {
        //     JobStatus::Finished(output) => {
        //         let job = Job {
        //             status: JobStatus::Finished(output),
        //             context: Some(RuntimeContext::JavaScript(context)),
        //             ..job
        //         };

        //         scheduler_controller.enqueue_nowait(job);
        //     }
        //     JobStatus::Pending(request) if !job.is_timeout() => {
        //         let job = Job {
        //             status: JobStatus::Pending(request),
        //             context: Some(RuntimeContext::JavaScript(context)),
        //             ..job
        //         };

        //         pending_manager_controller.enqueue_nowait(job);
        //     }
        //     _ => {
        //         job.cancel();
        //         scheduler_controller.enqueue_nowait(job);
        //     }
        // }

        // let job_remaining_time = request.job.remaining_time;

        // let mut processed_job = {
        //     // start measuring time
        //     let start = ThreadTime::now();

        //     // execute job with runtime
        //     let mut processed_job = self.runtime.execute(request.job);

        //     // stop measuring time
        //     processed_job.sub_remaining_time(start.elapsed());

        //     processed_job
        // };

        // {
        //     // update max_queueing_time
        //     self.max_queueing_time
        //         .lock()
        //         .unwrap()
        //         .sub_assign(job_remaining_time);
        // }

        // match processed_job.status {
        //     JobStatus::Finished(_) | JobStatus::Cancelled => {
        //         // self.updater_controller.update_job_nowait(processed_job);
        //         self.scheduler_controller.enqueue_nowait(processed_job);
        //     }
        //     JobStatus::Pending(_) => {
        //         if processed_job.is_timeout() {
        //             println!("Job is timeout");
        //             processed_job.cancel();
        //             // self.updater_controller.update_job_nowait(processed_job);
        //             self.scheduler_controller.enqueue_nowait(processed_job);
        //         } else {
        //             self.pending_manager_controller
        //                 .enqueue_nowait(processed_job);
        //         }
        //     }
        //     _ => {
        //         processed_job.cancel();
        //         // self.updater_controller.update_job_nowait(processed_job);
        //         self.scheduler_controller.enqueue_nowait(processed_job);
        //         unreachable!();
        //     }
        // }
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
    /// get_max_queueing_time
    ///
    ///
    pub fn max_queueing_time(&self) -> Duration {
        *self.max_queueing_time.lock().unwrap()
    }

    // /// get_blob
    // ///
    // ///
    // pub async fn enqueue(&self, job: Job) {
    //     {
    //         self.max_queueing_time
    //             .lock()
    //             .unwrap()
    //             .add_assign(job.remaining_time);
    //     }

    //     let request = Command::Enqueue(enqueue::Request { job });
    //     self.command_sender.send(request).await.unwrap();
    // }

    pub async fn register(&self, job: Job) -> register::Handle {
        {
            self.max_queueing_time
                .lock()
                .unwrap()
                .add_assign(job.remaining_time);
        }

        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();

        let request = Command::Register(register::Request {
            response_sender,
            job,
        });

        self.command_sender.send(request).await.unwrap();

        register::Handle { response_receiver }
    }
}

/// Command
///
///
///
///
pub enum Command {
    // Enqueue(enqueue::Request),
    Register(register::Request),
}

// pub mod enqueue {

//     use super::*;

//     pub struct Request {
//         pub job: Job,
//     }
// }

pub mod register {
    use super::*;

    pub struct Handle {
        pub response_receiver: tokio::sync::oneshot::Receiver<Response>,
    }

    pub struct Request {
        pub response_sender: tokio::sync::oneshot::Sender<Response>,
        pub job: Job,
    }

    #[derive(Debug)]
    pub struct Response {
        pub job: Job,
    }
}
