use bytes::Bytes;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio::sync::mpsc;

use crate::pleiades_type::{Job, JobStatus};
use crate::runtime::{blob, gpu, http, RuntimeRequest, RuntimeResponse};
use crate::{data_manager, scheduler};

/// PendingManager
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
    scheduler_controller: scheduler::Controller,

    /// data_manager_controller
    ///
    data_manager_controller: data_manager::Controller,

    /// http_client
    ///
    http_client: reqwest::Client,
}

impl PendingManager {
    /// new
    ///
    ///
    ///
    ///
    pub fn new(
        scheduler_controller: scheduler::Controller,
        data_manager_controller: data_manager::Controller,
    ) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(64);

        let data_manager = Self {
            command_receiver,
            task_counter: Arc::new(AtomicUsize::new(0)),
            scheduler_controller,
            data_manager_controller,
            http_client: reqwest::Client::new(),
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
        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Command::Register(request) => {
                    let mut job = request.job;

                    // extract request
                    //
                    let runtime_request = if let JobStatus::Pending(request) = job.status {
                        request
                    } else {
                        println!("unreachable");
                        continue;
                    };
                    //
                    // /////

                    self.task_counter.fetch_add(1, Ordering::Relaxed);

                    // clone to move
                    let task_counter = self.task_counter.clone();
                    let scheduler_controller = self.scheduler_controller.clone();

                    match runtime_request {
                        // blob
                        //
                        //
                        RuntimeRequest::Blob(blob::Request::Get(blob_id)) => {
                            job.status = JobStatus::Resolving;

                            let handle = self.data_manager_controller.get_blob(blob_id).await;

                            tokio::spawn(async move {
                                Self::task_blob_get(scheduler_controller, handle, job).await;
                                task_counter.fetch_sub(1, Ordering::Relaxed);
                            });
                        }
                        RuntimeRequest::Blob(blob::Request::Post(data)) => {
                            job.status = JobStatus::Resolving;

                            let handle = self.data_manager_controller.post_blob(data).await;

                            tokio::spawn(async move {
                                Self::task_blob_post(scheduler_controller, handle, job).await;
                                task_counter.fetch_sub(1, Ordering::Relaxed);
                            });
                        }
                        //
                        //
                        // /////

                        // gpu
                        //
                        //
                        RuntimeRequest::Gpu(gpu::Request { data }) => {
                            job.status = JobStatus::Resolving;

                            tokio::spawn(async move {
                                task_counter.fetch_sub(1, Ordering::Relaxed);
                            });
                        }
                        //
                        //
                        // /////

                        // http
                        //
                        //
                        RuntimeRequest::Http(http::Request::Get(url)) => {
                            job.status = JobStatus::Resolving;

                            let client = self.http_client.clone();

                            tokio::spawn(async move {
                                Self::task_http_get(scheduler_controller, client, url.clone(), job)
                                    .await;

                                task_counter.fetch_sub(1, Ordering::Relaxed);
                            });
                        }
                        RuntimeRequest::Http(http::Request::Post { url, body }) => {
                            job.status = JobStatus::Resolving;

                            let client = self.http_client.clone();

                            tokio::spawn(async move {
                                Self::task_http_post(
                                    scheduler_controller,
                                    client,
                                    url.clone(),
                                    body,
                                    job,
                                )
                                .await;

                                task_counter.fetch_sub(1, Ordering::Relaxed);
                            });
                        } //
                          //
                          // /////
                    }
                }
            }
        }

        self.wait_for_shutdown().await;
    }

    /// task_blob_get
    ///
    ///
    async fn task_blob_get(
        scheduler_controller: scheduler::Controller,
        handle: data_manager::get_blob::Handle,
        mut job: Job,
    ) {
        let response = handle.recv().await;
        job.status = JobStatus::Ready(RuntimeResponse::Blob(blob::Response::Get(response.blob)));

        scheduler_controller.enqueue_global(job).await;
    }

    /// task_blob_post
    ///
    ///
    async fn task_blob_post(
        scheduler_controller: scheduler::Controller,
        handle: data_manager::post_blob::Handle,
        mut job: Job,
    ) {
        let response = handle.recv().await;
        job.status = JobStatus::Ready(RuntimeResponse::Blob(blob::Response::Post(response.blob)));

        scheduler_controller.enqueue_global(job).await;
    }

    /// task_http_get
    ///
    ///
    async fn task_http_get(
        scheduler_controller: scheduler::Controller,
        client: reqwest::Client,
        url: String,
        mut job: Job,
    ) {
        let response = client.get(url).send().await.ok();

        match response {
            Some(response) => {
                let body = response.bytes().await.unwrap();
                job.status =
                    JobStatus::Ready(RuntimeResponse::Http(http::Response::Get(Some(body))));
            }
            None => {
                println!("http get failed");
                job.status = JobStatus::Ready(RuntimeResponse::Http(http::Response::Get(None)));
            }
        }

        scheduler_controller.enqueue_global(job).await;
    }

    /// task_http_post
    ///
    ///
    async fn task_http_post(
        scheduler_controller: scheduler::Controller,
        client: reqwest::Client,
        url: String,
        body: Bytes,
        mut job: Job,
    ) {
        let response = client.post(url).body(body).send().await.ok();

        match response {
            Some(response) => {
                let body = response.bytes().await.unwrap();
                job.status =
                    JobStatus::Ready(RuntimeResponse::Http(http::Response::Post(Some(body))));
            }
            None => {
                println!("http post failed");
                job.status = JobStatus::Ready(RuntimeResponse::Http(http::Response::Post(None)));
            }
        }

        scheduler_controller.enqueue_global(job).await;
    }

    /// wait_for_shutdown
    ///
    pub async fn wait_for_shutdown(&self) {
        println!("Pending Manager is shutting down");

        while self.task_counter.load(Ordering::Relaxed) > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        println!("Pending Manager is shut down");
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
    /// register
    ///
    ///
    ///
    ///
    pub async fn register(&self, job: Job) {
        let request = Command::Register(register::Request { job });
        self.command_sender.send(request).await.unwrap();
    }

    /// register_nowait
    ///
    ///
    ///
    ///
    pub fn register_nowait(&self, job: Job) {
        let request = Command::Register(register::Request { job });
        self.command_sender.blocking_send(request).unwrap();
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
