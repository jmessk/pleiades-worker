use bytes::Bytes;
use std::sync::{atomic::AtomicU32, Arc};
use tokio::sync::{mpsc, oneshot};

use crate::rm::fetcher;

/// Contractor
///
///
///
///
pub struct DataManager {
    fetcher_api: fetcher::Api,

    /// interface to access this component
    ///
    command_sender: mpsc::Sender<Command>,
    command_receiver: mpsc::Receiver<Command>,

    /// number of jobs currently being contracted
    ///
    task_counter: Arc<AtomicU32>,
}

impl DataManager {
    /// new
    ///
    pub fn new(fetcher_api: fetcher::Api) -> Self {
        let (request_sender, request_receiver) = mpsc::channel(64);

        Self {
            fetcher_api,
            command_sender: request_sender,
            command_receiver: request_receiver,
            task_counter: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn api(&self) -> Api {
        Api {
            command_sender: self.command_sender.clone(),
        }
    }

    /// run
    ///
    pub async fn run(&mut self) {
        while let Some(request) = self.command_receiver.recv().await {
            let fetcher_api = self.fetcher_api.clone();
            let task_counter = self.task_counter.clone();

            tokio::spawn(async move {
                task_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                match request {
                    Command::GetBlob(request) => Self::task_get_blob(fetcher_api, request).await,
                    Command::PostBlob(request) => Self::task_post_blob(fetcher_api, request).await,
                }

                task_counter.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            });
        }

        self.wait_for_shutdown().await;
    }

    /// task
    ///
    async fn task_get_blob(fetcher_api: fetcher::Api, request: get_blob::Request) {
        let download_handle = fetcher_api.download_blob(request.blob_id).await;
        let data = download_handle.recv().await.data;

        request
            .response_sender
            .send(get_blob::Response { data })
            .expect("fetcher");
    }

    /// task
    ///
    async fn task_post_blob(fetcher_api: fetcher::Api, request: post_blob::Request) {
        let upload_handle = fetcher_api.upload_blob(request.data).await;
        let blob_id = upload_handle.recv().await.blob_id;

        request
            .response_sender
            .send(post_blob::Response { blob_id })
            .expect("fetcher");
    }

    /// wait_for_shutdown
    ///
    pub async fn wait_for_shutdown(&self) {
        println!("Fetcher is shutting down");

        while self.task_counter.load(std::sync::atomic::Ordering::Relaxed) > 0 {
            print!(".");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        println!("Fetcher is shut down");
    }
}

/// Api
///
///
///
///
pub struct Api {
    command_sender: mpsc::Sender<Command>,
}

impl Api {
    /// get_blob
    ///
    pub async fn get_blob(&self, blob_id: String) -> get_blob::Handle {
        let (response_sender, response_receiver) = oneshot::channel();

        let request = Command::GetBlob(get_blob::Request {
            response_sender,
            blob_id,
        });

        self.command_sender.send(request).await.unwrap();

        get_blob::Handle { response_receiver }
    }

    /// post_blob
    ///
    pub async fn post_blob(&self, data: Bytes) -> post_blob::Handle {
        let (response_sender, response_receiver) = oneshot::channel();

        let request = Command::PostBlob(post_blob::Request {
            response_sender,
            data,
        });

        self.command_sender.send(request).await.unwrap();

        post_blob::Handle { response_receiver }
    }
}

pub enum Command {
    GetBlob(get_blob::Request),
    PostBlob(post_blob::Request),
}

// #[derive(Debug)]
// pub enum Response {
//     GetBlob { blob: Bytes },
//     PostBlob { blob_id: String },
// }

pub mod get_blob {
    use super::*;

    pub struct Request {
        pub response_sender: oneshot::Sender<Response>,
        pub blob_id: String,
    }

    #[derive(Debug)]
    pub struct Response {
        pub data: Bytes,
    }

    pub struct Handle {
        pub response_receiver: oneshot::Receiver<Response>,
    }

    impl Handle {
        pub async fn recv_nowait(&mut self) -> Option<Response> {
            self.response_receiver.try_recv().ok()
        }
    }
}

pub mod post_blob {
    use super::*;

    pub struct Request {
        pub response_sender: oneshot::Sender<Response>,
        pub data: Bytes,
    }

    #[derive(Debug)]
    pub struct Response {
        pub blob_id: String,
    }

    pub struct Handle {
        pub response_receiver: oneshot::Receiver<Response>,
    }

    impl Handle {
        pub async fn recv_nowait(&mut self) -> Option<Response> {
            self.response_receiver.try_recv().ok()
        }
    }
}
