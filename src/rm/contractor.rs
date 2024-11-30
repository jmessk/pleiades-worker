use std::sync::{atomic::AtomicU32, Arc};
use tokio::sync::{mpsc, oneshot};

use crate::types::JobMetadata;

/// Contractor
///
///
///
///
pub struct Contractor {
    client: Arc<pleiades_api::Client>,

    /// interface to access this component
    ///
    request_sender: mpsc::Sender<Request>,
    request_receiver: mpsc::Receiver<Request>,

    /// number of jobs currently being contracted
    ///
    task_counter: Arc<AtomicU32>,
}

impl Contractor {
    /// new
    ///
    pub fn new(client: Arc<pleiades_api::Client>) -> Self {
        let (request_sender, request_receiver) = mpsc::channel(64);

        Self {
            client,
            request_sender,
            request_receiver,
            task_counter: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn api(&self) -> Api {
        Api {
            num_contracting: self.task_counter.clone(),
            request_sender: self.request_sender.clone(),
        }
    }

    /// run
    ///
    pub async fn run(&mut self) {
        while let Some(request) = self.request_receiver.recv().await {
            let client = self.client.clone();
            let task_counter = self.task_counter.clone();

            tokio::spawn(async move {
                task_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Self::task(client, request).await;
                task_counter.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            });
        }

        self.wait_for_shutdown().await;
    }

    /// contract_task
    ///
    async fn task(client: Arc<pleiades_api::Client>, request: Request) {
        // Fetch a new job
        //
        let contract_request = pleiades_api::api::worker::contract::Request::builder()
            .worker_id(request.runtime)
            .timeout(10)
            .build();

        let contract_response = client.call_api(&contract_request).await;

        let maybe_job = contract_response.expect("no error handling: contract");
        // let maybe_job = match contract_response {
        //     Ok(response) => response,
        //     Err(err) => {
        //         request
        //             .response_sender
        //             .send(Err(err.to_string()))
        //             .await
        //             .expect("contractor cannot send response error");
        //         return;
        //     }
        // };

        let job_id = match maybe_job.job_id {
            Some(job_id) => job_id,
            None => {
                request.response_sender.send(Response { contracted: None }).expect("contractor");
                return;
            }
        };

        // Fetch job metadata
        //
        let info_request = pleiades_api::api::job::info::Request::builder()
            .job_id(job_id)
            .build();

        let info_response = client.call_api(&info_request).await;

        let info = info_response.expect("no error handling: job info");
        // let info = match info_response {
        //     Ok(response) => response,
        //     Err(err) => {
        //         request.response_sender
        //             .send(Err(err.to_string()))
        //             .await
        //             .expect("contractor cannot send info");
        //         return;
        //     }
        // };

        let job_metadata = JobMetadata {
            id: info.job_id,
            lambda_id: info.lambda.lambda_id,
            input_id: info.input.data_id,
        };

        request
            .response_sender
            .send(Response {
                contracted: Some(job_metadata),
            })
            .expect("contractor");
    }

    /// wait_for_shutdown
    ///
    pub async fn wait_for_shutdown(&self) {
        println!("Contractor is shutting down");

        while self.task_counter.load(std::sync::atomic::Ordering::Relaxed) > 0 {
            print!(".");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        println!("Contractor is shut down");
    }
}

/// Api
///
///
///
///
pub struct Api {
    num_contracting: Arc<AtomicU32>,
    request_sender: mpsc::Sender<Request>,
}

impl Api {
    /// contract
    ///
    pub async fn try_contract(&self, runtime: String) -> Handler {
        // self.request_sender.send(runtime).await.unwrap();
        let (response_sender, response_receiver) = oneshot::channel();

        let request = Request {
            runtime,
            response_sender,
        };

        self.request_sender.send(request).await.unwrap();

        Handler { response_receiver }
    }

    /// num_contracting
    ///
    pub fn num_contracting(&self) -> u32 {
        self.num_contracting
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

pub struct Request {
    runtime: String,
    response_sender: oneshot::Sender<Response>,
}

#[derive(Debug)]
pub struct Response {
    pub contracted: Option<JobMetadata>,
}

pub struct Handler {
    response_receiver: oneshot::Receiver<Response>,
}

impl Handler {
    pub async fn recv_nowait(&mut self) -> Option<Response> {
        self.response_receiver.try_recv().ok()
    }
}
