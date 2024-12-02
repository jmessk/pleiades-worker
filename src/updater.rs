use std::sync::{atomic::AtomicU32, Arc};
use tokio::sync::{mpsc, oneshot};

/// Contractor
///
///
///
///
pub struct Updater {
    client: Arc<pleiades_api::Client>,

    /// interface to access this component
    ///
    command_sender: mpsc::Sender<Command>,
    command_receiver: mpsc::Receiver<Command>,

    /// number of jobs currently being contracted
    ///
    task_counter: Arc<AtomicU32>,
}

impl Updater {
    /// new
    ///
    pub fn new(client: Arc<pleiades_api::Client>) -> Self {
        let (request_sender, request_receiver) = mpsc::channel(64);

        Self {
            client,
            command_sender: request_sender,
            command_receiver: request_receiver,
            task_counter: Arc::new(AtomicU32::new(0)),
        }
    }

    /// api
    ///
    pub fn api(&self) -> Api {
        Api {
            command_sender: self.command_sender.clone(),
        }
    }

    /// run
    ///
    pub async fn run(&mut self) {
        while let Some(request) = self.command_receiver.recv().await {
            // clone fields to bind
            let client = self.client.clone();
            let task_counter = self.task_counter.clone();

            // new task
            tokio::spawn(async move {
                task_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                match request {
                    Command::FinishJob(request) => Self::task_finish_job(client, request).await,
                    // Command::PostBlob(request) => Self::task_post_blob(client, request).await,
                }

                task_counter.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            });
        }

        self.wait_for_shutdown().await;
    }

    /// task
    ///
    async fn task_finish_job(client: Arc<pleiades_api::Client>, request: finish::Request) {
        let update_request = pleiades_api::api::job::update::Request::builder()
            .job_id(request.job_id)
            .data_id(request.output_id)
            .status("finished")
            .build();

        let update_request = client.call_api(&update_request).await;

        let _update_request = update_request.expect("no error handling: update job");
        // don't check error handling

        request
            .response_sender
            .send(finish::Response {})
            .expect("updater");
    }

    /// wait_for_shutdown
    ///
    pub async fn wait_for_shutdown(&self) {
        println!("Updater is shutting down");

        while self.task_counter.load(std::sync::atomic::Ordering::Relaxed) > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        println!("Updater is shut down");
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
    pub async fn finish_job(&self, job_id: String, output_id: String) -> finish::Handle {
        let (response_sender, response_receiver) = oneshot::channel();

        let request = Command::FinishJob(finish::Request {
            response_sender,
            job_id,
            output_id,
        });

        self.command_sender.send(request).await.unwrap();

        finish::Handle { response_receiver }
    }
}

pub enum Command {
    FinishJob(finish::Request),
    // PostBlob(cannel::Request),
}

pub mod finish {
    use super::*;

    pub struct Request {
        pub response_sender: oneshot::Sender<Response>,
        pub job_id: String,
        pub output_id: String,
    }

    #[derive(Debug)]
    pub struct Response {}

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

// pub mod cannel {
//     use super::*;

//     pub struct Request {
//         pub response_sender: oneshot::Sender<Response>,
//         pub data: Bytes,
//     }

//     #[derive(Debug)]
//     pub struct Response {
//         pub blob_id: String,
//     }

//     pub struct Handler {
//         pub response_receiver: oneshot::Receiver<Response>,
//     }

//     impl Handler {
//         pub async fn recv_nowait(&mut self) -> Option<Response> {
//             self.response_receiver.try_recv().ok()
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contractor::Contractor;

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
    async fn test_contract() {
        let client = Arc::new(pleiades_api::Client::new("http://master.local/api/v0.5/").unwrap());
        let mut contractor = Contractor::new(client.clone());
        let api = contractor.api();

        let mut updater = Updater::new(client.clone());
        let updater_api = updater.api();

        tokio::spawn(async move {
            contractor.run().await;
        });

        tokio::spawn(async move {
            updater.run().await;
        });

        let requester_client = client.clone();
        let requester = tokio::spawn(async move { generate_job(&requester_client).await });

        let worker_id = register_worker(&client).await;
        let mut handle = api.contract(worker_id).await;

        let response = handle.recv_nowait().unwrap();

        println!("{:?}", response);
        assert!(response.contracted.is_some());

        let job_id = response.contracted.unwrap().id;
        finish_job(&client, &job_id).await;

        let output = requester.await.unwrap();

        assert_eq!(output, bytes::Bytes::from("test output"));
    }
}
