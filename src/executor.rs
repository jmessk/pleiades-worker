use cpu_time::ThreadTime;
use std::ops::AddAssign;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::pleiades_type::{Job, JobStatus};
use crate::runtime::{JsRuntime, Runtime as _, RuntimeContext};

/// Executor
///
///
///
///
pub struct Executor {
    /// interface to access this component
    ///
    command_receiver: mpsc::Receiver<Command>,

    /// id_num
    ///
    id: usize,

    /// max_queueing_time
    ///
    _max_queueing_time: Arc<Mutex<Duration>>,

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
    pub fn new(id: usize) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(64);

        let max_queueing_time = Arc::new(Mutex::new(Duration::ZERO));

        let data_manager = Self {
            command_receiver,
            id,
            _max_queueing_time: max_queueing_time.clone(),
            runtime: JsRuntime::init(),
        };

        let controller = Controller {
            command_sender,
            max_queueing_time,
            id,
        };

        (data_manager, controller)
    }

    // / set_controller
    // /
    // /
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
        tracing::info!("Executor {}: running", self.id);

        while let Some(command) = self.command_receiver.blocking_recv() {
            tracing::debug!("Executor {}: start execute", self.id);
            println!("Executor {}: start execute", self.id);

            match command {
                Command::Register(request) => self.task_execute_job(request),
            }
        }

        tracing::info!("Executor {}: shutdown", self.id);
    }

    fn task_execute_job(&mut self, request: register::Request) {
        let mut job = request.job;

        match job.status {
            JobStatus::Assigned => match self.runtime.create_context(&job.lambda, &job.input) {
                Ok(context) => {
                    self.runtime.set_context(context);
                }
                Err(e) => {
                    tracing::warn!("Executor {}: {}", self.id, e);
                    job.cancel();

                    request
                        .response_sender
                        .send(register::Response { job })
                        .unwrap();

                    return;
                }
            },
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

        // execution
        //
        //
        let job_status = {
            // start measuring time
            let start = ThreadTime::now();

            // execute job with runtime
            let job_status = self.runtime.execute().unwrap();

            // stop measuring time
            let elapsed = start.elapsed();

            job.sub_rem_time(elapsed);

            job_status
        };

        let job = match job_status {
            JobStatus::Finished(_) => Job {
                status: job_status,
                context: None,
                ..job
            },
            JobStatus::Pending(_) if !job.is_timeout() => {
                let context = match self.runtime.get_context() {
                    Some(context) => context,
                    None => unreachable!(),
                };

                Job {
                    status: job_status,
                    context: Some(RuntimeContext::JavaScript(context)),
                    ..job
                }
            }
            _ => {
                job.cancel();
                job
            }
        };

        request
            .response_sender
            .send(register::Response { job })
            .unwrap();

        tracing::trace!("Executor {}: finish execute", self.id);
    }

    fn sub_queueing_time(&self, duration: Duration) {
        let mut max_queueing_time = self._max_queueing_time.lock().unwrap();
        if *max_queueing_time < duration {
            print!(
                "max_queueing_time: {:?}, duration: {:?}",
                max_queueing_time, duration
            );
        }
        *max_queueing_time = max_queueing_time.checked_sub(duration).unwrap();
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
    pub id: usize,
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

    pub async fn enqueue(&self, job: Job) -> register::Handle {
        let rem_time = job.rem_time;

        {
            self.max_queueing_time.lock().unwrap().add_assign(rem_time);
        }

        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
        let request = Command::Register(register::Request {
            response_sender,
            job,
        });
        self.command_sender.send(request).await.unwrap();

        register::Handle {
            response_receiver,
            max_queueing_time: self.max_queueing_time.clone(),
            prev_rem_time: rem_time,
        }
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
    use std::ops::SubAssign as _;

    use tokio::sync::oneshot;

    use super::*;

    pub struct Handle {
        pub(super) response_receiver: oneshot::Receiver<Response>,
        pub(super) prev_rem_time: Duration,
        pub(super) max_queueing_time: Arc<Mutex<Duration>>,
    }

    impl Handle {
        pub async fn recv(self) -> Response {
            let response = self.response_receiver.await.unwrap();

            {
                self.max_queueing_time
                    .lock()
                    .unwrap()
                    .sub_assign(self.prev_rem_time);
            }

            response
        }
    }

    pub struct Request {
        pub response_sender: oneshot::Sender<Response>,
        pub job: Job,
    }

    #[derive(Debug)]
    pub struct Response {
        pub job: Job,
    }
}
