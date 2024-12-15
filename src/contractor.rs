use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot};

use crate::{
    data_manager,
    pleiades_type::{Job, JobStatus, Lambda},
};

/// Contractor
///
///
///
///
pub struct Contractor {
    client: Arc<pleiades_api::Client>,

    /// data manager api
    ///
    data_manager_controller: data_manager::Controller,

    /// interface to access this component
    ///
    command_receiver: mpsc::Receiver<Command>,

    /// semaphore
    ///
    semaphore: Arc<tokio::sync::Semaphore>,

    /// max_concurrency
    ///
    max_concurrency: usize,
}

impl Contractor {
    /// new
    ///
    pub fn new(
        client: Arc<pleiades_api::Client>,
        data_manager_controller: data_manager::Controller,
        max_concurrency: usize,
    ) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(8);

        let contractor = Self {
            client,
            data_manager_controller,
            command_receiver,
            semaphore: Arc::new(tokio::sync::Semaphore::new(max_concurrency)),
            max_concurrency,
        };

        let controller = Controller { command_sender };

        (contractor, controller)
    }

    /// run
    ///
    ///
    ///
    ///
    pub async fn run(&mut self) {
        while let Some(command) = self.command_receiver.recv().await {
            let client = self.client.clone();
            let data_manager_controller = self.data_manager_controller.clone();

            if self.semaphore.available_permits() == 0 {
                println!("Contractor: busy");
            }

            let permit = self.semaphore.clone().acquire_owned().await.unwrap();

            tokio::spawn(async move {
                match command {
                    Command::Contract(request) => {
                        Self::task_contract(client, data_manager_controller, request).await
                    }
                }
                let _permit = permit;
            });
        }

        self.wait_for_shutdown().await;
    }

    /// contract_task
    ///
    ///
    ///
    ///
    async fn task_contract(
        client: Arc<pleiades_api::Client>,
        data_manager_controller: data_manager::Controller,
        request: contract::Request,
    ) {
        // Fetch a new job
        //
        let contract_request = pleiades_api::api::worker::contract::Request::builder()
            .worker_id(request.worker_id)
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
                request
                    .response_sender
                    .send(contract::Response { contracted: None })
                    .expect("contractor");
                return;
            }
        };

        // fetch job metadata
        //
        let info_request = pleiades_api::api::job::info::Request::builder()
            .job_id(&job_id)
            .build();

        let info_response = client.call_api(&info_request).await;

        let job_info = info_response.expect("no error handling: job info");
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

        // download input and lambda code
        //
        let get_input_handle = data_manager_controller
            .get_blob(job_info.input.data_id)
            .await;
        let get_code_handle = data_manager_controller
            .get_blob(job_info.lambda.data_id)
            .await;
        //
        let input = get_input_handle.recv().await.blob;
        let code = get_code_handle.recv().await.blob;
        //
        // ////

        let job = Job {
            id: job_id,
            status: JobStatus::Assigned,
            remaining_time: Duration::from_secs(0),
            context: None,
            lambda: Lambda {
                id: job_info.lambda.lambda_id,
                runtime: job_info.lambda.runtime,
                code: code.expect("contractor: code"),
            },
            input: input.expect("contractor: input"),
        };

        request
            .response_sender
            .send(contract::Response {
                contracted: Some(job),
            })
            .expect("contractor");
    }

    /// wait_for_shutdown
    ///
    ///
    ///
    ///
    pub async fn wait_for_shutdown(&self) {
        println!("Contractor is shutting down");

        while self.semaphore.available_permits() != self.max_concurrency {
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
#[derive(Clone)]
pub struct Controller {
    command_sender: mpsc::Sender<Command>,
}

impl Controller {
    /// contract
    ///
    pub async fn try_contract(&self, worker_id: String) -> anyhow::Result<contract::Handle> {
        let (response_sender, response_receiver) = oneshot::channel();

        let request = Command::Contract(contract::Request {
            worker_id,
            response_sender,
        });

        self.command_sender.send(request).await?;

        Ok(contract::Handle { response_receiver })
    }

    // /// num_contracting
    // ///
    // pub fn num_contracting(&self) -> u32 {
    //     self.num_contracting
    //         .load(std::sync::atomic::Ordering::Relaxed)
    // }
}

pub enum Command {
    Contract(contract::Request),
}

pub mod contract {
    use super::*;

    pub struct Request {
        pub worker_id: String,
        pub response_sender: oneshot::Sender<Response>,
    }

    #[derive(Debug)]
    pub struct Response {
        pub contracted: Option<Job>,
    }

    pub struct Handle {
        pub response_receiver: oneshot::Receiver<Response>,
    }

    impl Handle {
        pub async fn recv(self) -> Response {
            self.response_receiver.await.unwrap()
        }

        pub fn recv_nowait(&mut self) -> Option<Response> {
            self.response_receiver.try_recv().ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fetcher;

    use super::*;

    async fn register_worker(client: &Arc<pleiades_api::Client>) -> String {
        let request = pleiades_api::api::worker::register::Request::builder()
            .runtimes(&["pleiades+test"])
            .build();

        let response = client.call_api(&request).await;

        response
            .expect("no error handling: register worker")
            .worker_id
    }

    async fn generate_job(client: &Arc<pleiades_api::Client>) -> bytes::Bytes {
        use pleiades_api::api;

        let code_blob = {
            let request = api::data::upload::Request::builder()
                .data(r#"console.log("hello, world!");"#)
                .build();
            client.call_api(&request).await.unwrap()
        };

        // lambda
        let lambda = {
            let request = api::lambda::create::Request::builder()
                .data_id(code_blob.data_id)
                .runtime("pleiades+test")
                .build();
            client.call_api(&request).await.unwrap()
        };

        // input blob
        let input = {
            let request = api::data::upload::Request::builder()
                .data("test input")
                .build();
            client.call_api(&request).await.unwrap()
        };

        // create job
        let create_job = {
            let request = api::job::create::Request::builder()
                .lambda_id(lambda.lambda_id)
                .data_id(input.data_id)
                .build();
            client.call_api(&request).await.unwrap()
        };

        // wait for finish
        let job_info = {
            let request = api::job::info::Request::builder()
                .job_id(create_job.job_id)
                .except("Finished")
                .timeout(10)
                .build();
            client.call_api(&request).await.unwrap()
        };

        // download output
        let output = {
            let request = api::data::download::Request::builder()
                .data_id(job_info.output.unwrap().data_id)
                .build();
            client.call_api(&request).await.unwrap()
        };

        output.data
    }

    async fn finish_job(client: &Arc<pleiades_api::Client>, job_id: &str) {
        use pleiades_api::api;

        let output = {
            let request = api::data::upload::Request::builder()
                .data("test output")
                .build();
            client.call_api(&request).await.unwrap()
        };

        let _update = {
            let request = api::job::update::Request::builder()
                .job_id(job_id)
                .data_id(output.data_id)
                .status("finished")
                .build();
            client.call_api(&request).await.unwrap()
        };
    }

    #[tokio::test]
    async fn test_ping() {
        let client = pleiades_api::Client::try_new("http://192.168.1.47/api/v0.5/").unwrap();
        println!("{:?}", client.ping().await.unwrap());
    }

    #[tokio::test]
    async fn test_contract() {
        // let client = Arc::new(pleiades_api::Client::new("http://master.local/api/v0.5/").unwrap());
        let client =
            Arc::new(pleiades_api::Client::try_new("http://192.168.1.47/api/v0.5/").unwrap());

        let (mut _fetcher, fetcher_api) = fetcher::Fetcher::new(client.clone());
        let (mut _data_manager, data_manager_controller) =
            data_manager::DataManager::new(fetcher_api);
        let (mut contractor, contractor_api) =
            Contractor::new(client.clone(), data_manager_controller, 16);

        tokio::spawn(async move {
            contractor.run().await;
        });

        let requester_client = client.clone();
        let requester = tokio::spawn(async move { generate_job(&requester_client).await });

        let worker_id = register_worker(&client).await;
        let handle = contractor_api.try_contract(worker_id).await.unwrap();

        let response = handle.response_receiver.await.unwrap();

        println!("{:?}", response);
        assert!(response.contracted.is_some());

        let job_id = response.contracted.unwrap().id;
        finish_job(&client, &job_id).await;

        let output = requester.await.unwrap();

        assert_eq!(output, bytes::Bytes::from("test output"));
    }
}
