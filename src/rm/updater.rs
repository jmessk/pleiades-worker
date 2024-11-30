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
            request_sender: self.command_sender.clone(),
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
pub struct Api {
    request_sender: mpsc::Sender<Command>,
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

        self.request_sender.send(request).await.unwrap();

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
        pub async fn recv_nowait(&mut self) -> Option<Response> {
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
