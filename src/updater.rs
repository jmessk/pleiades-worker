use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::mpsc;

use crate::{
    data_manager,
    pleiades_type::{Job, JobStatus},
};

/// Contractor
///
///
///
///
pub struct Updater {
    client: Arc<pleiades_api::Client>,

    /// data manager api
    ///
    data_manager_controller: data_manager::Controller,

    /// interface to access this component
    ///
    // command_sender: mpsc::Sender<Command>,
    command_receiver: mpsc::Receiver<Command>,

    /// number of jobs currently being contracted
    ///
    task_counter: Arc<AtomicUsize>,
}

impl Updater {
    /// new
    ///
    pub fn new(
        client: Arc<pleiades_api::Client>,
        data_manager_controller: data_manager::Controller,
    ) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(64);

        let updater = Self {
            client,
            data_manager_controller,
            command_receiver,
            task_counter: Arc::new(AtomicUsize::new(0)),
        };

        let controller = Controller { command_sender };

        (updater, controller)
    }

    /// api
    ///
    // pub fn api(&self) -> Api {
    //     Api {
    //         command_sender: self.command_sender.clone(),
    //     }
    // }

    /// run
    ///
    pub async fn run(&mut self) {
        while let Some(request) = self.command_receiver.recv().await {
            // clone fields to bind
            let client = self.client.clone();
            let data_manager_api = self.data_manager_controller.clone();
            let task_counter = self.task_counter.clone();

            // new task
            tokio::spawn(async move {
                task_counter.fetch_add(1, Ordering::Relaxed);

                match request {
                    Command::FinishJob(request) => {
                        Self::task_update_job(client, data_manager_api, request).await
                    }
                }

                task_counter.fetch_sub(1, Ordering::Relaxed);
            });
        }

        self.wait_for_shutdown().await;
    }

    /// task
    ///
    async fn task_update_job(
        client: Arc<pleiades_api::Client>,
        data_manager_api: data_manager::Controller,
        request: update::Request,
    ) {
        match request.job.status {
            JobStatus::Finished(output) => {
                let output = output.unwrap_or(bytes::Bytes::new());

                let post_handle = data_manager_api.post_blob(output).await;
                let post_response = post_handle.recv().await;

                let update_request = pleiades_api::api::job::update::Request::builder()
                    .job_id(request.job.id)
                    .data_id(post_response.blob.id)
                    .status("finished")
                    .build();

                let _update_response = client
                    .call_api(&update_request)
                    .await
                    .expect("no error handling: update");
            }
            JobStatus::Cancelled => {
                let update_request = pleiades_api::api::job::update::Request::builder()
                    .job_id(request.job.id)
                    .data_id("0")
                    .status("cancelled")
                    .build();

                let _update_response = client
                    .call_api(&update_request)
                    .await
                    .expect("no error handling: update");
            }
            _ => {}
        };
    }

    /// wait_for_shutdown
    ///
    pub async fn wait_for_shutdown(&self) {
        println!("Updater is shutting down");

        while self.task_counter.load(Ordering::Relaxed) > 0 {
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
pub struct Controller {
    command_sender: mpsc::Sender<Command>,
}

impl Controller {
    /// get_blob
    ///
    pub async fn update_job(&self, job: Job) {
        let request = Command::FinishJob(update::Request { job });
        self.command_sender.send(request).await.unwrap();
    }

    pub fn update_job_nowait(&self, job: Job) {
        let request = Command::FinishJob(update::Request { job });
        self.command_sender.blocking_send(request).unwrap();
    }
}

pub enum Command {
    FinishJob(update::Request),
    // PostBlob(cannel::Request),
}

pub mod update {
    use super::*;

    pub struct Request {
        pub job: Job,
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
    use crate::{contractor::Contractor, data_manager::DataManager, fetcher::Fetcher};

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
        let client =
            Arc::new(pleiades_api::Client::try_new("http://192.168.1.47/api/v0.5/").unwrap());

        let (mut fetcher, fetcher_api) = Fetcher::new(client.clone());
        let (mut data_manager, data_manager_api) = DataManager::new(fetcher_api);
        let (mut contractor, api) = Contractor::new(client.clone(), data_manager_api.clone(), 16);
        let (mut updater, _updater_api) = Updater::new(client.clone(), data_manager_api);

        tokio::spawn(async move {
            fetcher.run().await;
        });
        tokio::spawn(async move { data_manager.run().await });
        tokio::spawn(async move {
            contractor.run().await;
        });
        tokio::spawn(async move {
            updater.run().await;
        });

        let requester_client = client.clone();
        let requester = tokio::spawn(async move { generate_job(&requester_client).await });

        let worker_id = register_worker(&client).await;
        let handle = api.try_contract(worker_id).await.unwrap();

        let response = handle.recv().await;

        println!("{:?}", response);
        assert!(response.contracted.is_some());

        let job_id = response.contracted.unwrap().id;
        finish_job(&client, &job_id).await;

        let output = requester.await.unwrap();

        assert_eq!(output, bytes::Bytes::from("test output"));
    }
}
