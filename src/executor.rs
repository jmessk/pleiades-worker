use bytes::Bytes;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio::sync::{mpsc, oneshot};

use crate::pleiades_type::{Job, JobStatus};
use crate::runtime::{JsRuntime, Runtime as _};
use crate::updater;

/// Contractor
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
    ///
    updater_api: updater::Api,

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
    pub fn new(updater_api: updater::Api) -> (Self, Api) {
        let (command_sender, command_receiver) = mpsc::channel(64);

        let data_manager = Self {
            command_receiver,
            updater_api,
            runtime: JsRuntime::init(),
        };

        let api = Api { command_sender };

        (data_manager, api)
    }

    /// run
    ///
    ///
    ///
    ///
    pub fn run(&mut self) {
        while let Some(request) = self.command_receiver.blocking_recv() {
            match request {
                Command::Enqueue(request) => self.task_process_job(request),
            }
        }
    }

    fn task_process_job(&mut self, request: enqueue::Request) {
        let job = request.job;

        // execute job with runtime
        let processed_job = self.runtime.process(job);

        match processed_job.status {
            JobStatus::Finished(_) | JobStatus::Cancelled => {
                self.updater_api.update_job_nowait(processed_job);
            }
            JobStatus::Waiting { context: _, request } => {
                




                todo!()
            }
            _ => {}
        }
    }
}

/// Api
///
///
///
///
#[derive(Clone)]
pub struct Api {
    command_sender: mpsc::Sender<Command>,
}

impl Api {
    /// get_blob
    ///
    ///
    ///
    ///
    pub async fn enqueue(&self, job: Job) {
        let request = Command::Enqueue(enqueue::Request { job });
        self.command_sender.send(request).await.unwrap();
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
    use super::*;

    pub struct Request {
        pub job: Job,
    }
}
