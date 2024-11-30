use std::sync::{atomic::AtomicU32, Arc};
use tokio::sync::{mpsc, oneshot};

use crate::types::JobMetadata;

///
/// Contractor
///
///
///
pub struct Contractor {
    client: Arc<pleiades_api::Client>,
    request_receiver: mpsc::Receiver<String>,
    response_sender: mpsc::Sender<Result<Option<JobMetadata>, String>>,
    num_contracting: Arc<AtomicU32>,
}

impl Contractor {
    ///
    /// new
    ///
    pub fn new(client: Arc<pleiades_api::Client>) -> (Self, Api) {
        let (request_sender, request_receiver) = mpsc::channel(100);
        let (response_sender, response_receiver) = mpsc::channel(100);

        let contracting_counter = Arc::new(AtomicU32::new(0));

        let contractor = Self {
            client,
            request_receiver,
            response_sender,
            num_contracting: contracting_counter.clone(),
        };

        let api = Api {
            request_sender,
            response_receiver,
            num_contracting: contracting_counter,
        };

        (contractor, api)
    }

    ///
    /// run
    ///
    pub async fn run(&mut self) {
        while let Some(runtime) = self.request_receiver.recv().await {
            let client = self.client.clone();
            let response_sender = self.response_sender.clone();

            self.num_contracting
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            let contract_task = tokio::spawn(Self::contract_task(
                client,
                runtime,
                response_sender,
                self.num_contracting.clone(),
            ));
        }
    }

    ///
    /// contract_task
    ///
    async fn contract_task(
        client: Arc<pleiades_api::Client>,
        runtime: String,
        response_sender: mpsc::Sender<Result<Option<JobMetadata>, String>>,
        contracting_counter: Arc<AtomicU32>,
    ) {
        //
        // Fetch a new job
        //
        let contract_request = pleiades_api::api::worker::contract::Request::builder()
            .worker_id(runtime)
            .timeout(10)
            .build();

        let contract_response = client.call_api(&contract_request).await;

        contracting_counter.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);

        let maybe_job = match contract_response {
            Ok(response) => response,
            Err(err) => {
                response_sender
                    .send(Err(err.to_string()))
                    .await
                    .expect("contractor cannot send response error");
                return;
            }
        };

        let job_id = match maybe_job.job_id {
            Some(job_id) => job_id,
            None => {
                response_sender
                    .send(Ok(None))
                    .await
                    .expect("contractor cannot send no job response");
                return;
            }
        };

        //
        // Fetch job metadata
        //
        let info_request = pleiades_api::api::job::info::Request::builder()
            .job_id(job_id)
            .build();

        let info_response = client.call_api(&info_request).await;

        let info = match info_response {
            Ok(response) => response,
            Err(err) => {
                response_sender
                    .send(Err(err.to_string()))
                    .await
                    .expect("contractor cannot send info");
                return;
            }
        };

        let job_metadata = JobMetadata {
            id: info.job_id,
            lambda_id: info.lambda.lambda_id,
            input_id: info.input.data_id,
        };

        response_sender
            .send(Ok(Some(job_metadata)))
            .await
            .expect("contractor cannot send job metadata");
    }
}

///
/// Api
///
///
///
pub struct Api {
    request_sender: mpsc::Sender<String>,
    response_receiver: mpsc::Receiver<Result<Option<JobMetadata>, String>>,
    num_contracting: Arc<AtomicU32>,
}

impl Api {
    ///
    /// contract
    ///
    pub async fn try_contract(&self, runtime: String) {
        self.request_sender.send(runtime).await.unwrap();
    }

    ///
    /// receive
    ///
    pub async fn receive(&mut self) -> Result<Option<JobMetadata>, String> {
        self.response_receiver.recv().await.unwrap()
    }

    ///
    /// num_contracting
    ///
    pub fn num_contracting(&self) -> u32 {
        self.num_contracting
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

pub struct Request {
    runtime: String,
    response_sender: oneshot::Sender<Option<JobMetadata>>,
}

pub struct Handler {
    response_receiver: oneshot::Receiver<Option<JobMetadata>>
}
