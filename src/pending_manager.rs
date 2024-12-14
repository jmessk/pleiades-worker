use bytes::Bytes;
use cpu_time::ThreadTime;
use std::ops::{AddAssign, Sub};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

use crate::pleiades_type::{Job, JobStatus};
use crate::runtime::{blob, gpu, http, RuntimeRequest};
use crate::{data_manager, scheduler};

/// Contractor
///
///
///
///
pub struct PendingManager {
    /// interface to access this component
    ///
    command_receiver: mpsc::Receiver<Command>,

    /// number of tasks currently being processed
    ///
    task_counter: Arc<AtomicUsize>,

    /// scheduler_controller
    ///
    scheduler_controller: Arc<()>,

    /// data_manager_controller
    ///
    data_manager_controller: data_manager::Controller,
}

impl PendingManager {
    /// new
    ///
    ///
    ///
    ///
    pub fn new(data_manager_controller: data_manager::Controller) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(64);

        let data_manager = Self {
            command_receiver,
            task_counter: Arc::new(AtomicUsize::new(0)),
            scheduler_controller: Arc::new(()),
            data_manager_controller,
        };

        let controller = Controller { command_sender };

        (data_manager, controller)
    }

    /// run
    ///
    ///
    ///
    ///
    pub async fn run(&mut self) {
        while let Some(request) = self.command_receiver.recv().await {
            match request {
                Command::Register(request) => {
                    // extract request
                    //
                    let request = if let JobStatus::Pending {
                        context: _,
                        request,
                    } = request.job.status
                    {
                        request
                    } else {
                        println!("unreachable");
                        continue;
                    };
                    //
                    // /////

                    self.task_counter.fetch_add(1, Ordering::Relaxed);

                    let task_counter = self.task_counter.clone();
                    let scheduler_controller = self.scheduler_controller.clone();

                    match request {
                        // blob
                        RuntimeRequest::Blob(blob::Request::Get(blob_id)) => {
                            let handle = self.data_manager_controller.get_blob(blob_id).await;
                            tokio::spawn(async move {
                                task_counter.fetch_sub(1, Ordering::Relaxed);
                            });
                        }
                        RuntimeRequest::Blob(blob::Request::Post(data)) => {
                            tokio::spawn(async move {
                                task_counter.fetch_sub(1, Ordering::Relaxed);
                            });
                        }

                        // gpu
                        RuntimeRequest::Gpu(gpu::Request { data }) => {
                            tokio::spawn(async move {
                                task_counter.fetch_sub(1, Ordering::Relaxed);
                            });
                        }

                        // http
                        RuntimeRequest::Http(http::Request::Get(url)) => {
                            tokio::spawn(async move {
                                task_counter.fetch_sub(1, Ordering::Relaxed);
                            });
                        }
                        RuntimeRequest::Http(http::Request::Post(url)) => {
                            tokio::spawn(async move {
                                task_counter.fetch_sub(1, Ordering::Relaxed);
                            });
                        }
                    }
                }
            }
        }
    }

    async fn task_blob_get(
        scheduler_controller: Arc<()>,
        handle: data_manager::get_blob::Handle,
        job: Job,
    ) {
        let response = handle.recv().await;

        match response.blob {
            Some(blob) => {
                let old_status = job.status;
                job.status = JobStatus::Ready { context: old_status.0, response: () }
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
}

impl Controller {
    /// get_blob
    ///
    ///
    ///
    ///
    pub async fn register(&self, job: Job) {
        let request = Command::Register(register::Request { job });
        self.command_sender.send(request).await.unwrap();
    }
}

/// Command
///
///
///
///
pub enum Command {
    Register(register::Request),
}

pub mod register {
    use super::*;

    pub struct Request {
        pub job: Job,
    }
}
