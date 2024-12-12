use bytes::Bytes;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio::sync::{mpsc, oneshot};

use crate::fetcher;
use crate::pleiades_type::Blob;

/// Contractor
///
///
///
///
pub struct DataManager {
    fetcher_api: fetcher::Api,

    /// interface to access this component
    ///
    // command_sender: mpsc::Sender<Command>,
    command_receiver: mpsc::Receiver<Command>,

    /// number of jobs currently being contracted
    ///
    task_counter: Arc<AtomicUsize>,
}

impl DataManager {
    /// new
    ///
    ///
    ///
    ///
    pub fn new(fetcher_api: fetcher::Api) -> (Self, Api) {
        let (command_sender, command_receiver) = mpsc::channel(64);

        let data_manager = Self {
            fetcher_api,
            // command_sender: command_sender.clone(),
            command_receiver,
            task_counter: Arc::new(AtomicUsize::new(0)),
        };

        let api = Api { command_sender };

        (data_manager, api)
    }

    /// api
    ///
    ///
    ///
    ///
    // pub fn api(&self) -> Api {
    //     Api {
    //         command_sender: self.command_sender.clone(),
    //     }
    // }

    /// run
    ///
    ///
    ///
    ///
    pub async fn run(&mut self) {
        while let Some(request) = self.command_receiver.recv().await {
            let fetcher_api = self.fetcher_api.clone();
            let task_counter = self.task_counter.clone();

            tokio::spawn(async move {
                task_counter.fetch_add(1, Ordering::Relaxed);

                match request {
                    Command::GetBlob(request) => Self::task_get_blob(fetcher_api, request).await,
                    Command::PostBlob(request) => Self::task_post_blob(fetcher_api, request).await,
                }

                task_counter.fetch_sub(1, Ordering::Relaxed);
            });
        }

        self.wait_for_shutdown().await;
    }

    /// task
    ///
    ///
    ///
    ///
    async fn task_get_blob(fetcher_api: fetcher::Api, request: get_blob::Request) {
        let download_handle = fetcher_api.download_blob(request.blob_id).await;
        let blob = download_handle.recv().await.blob;

        request
            .response_sender
            .send(get_blob::Response { blob })
            .expect("fetcher");
    }

    /// task
    ///
    ///
    ///
    ///
    async fn task_post_blob(fetcher_api: fetcher::Api, request: post_blob::Request) {
        let upload_handle = fetcher_api.upload_blob(request.data.clone()).await;
        let blob = upload_handle.recv().await.blob;

        request
            .response_sender
            .send(post_blob::Response {
                blob: Blob {
                    id: blob.id,
                    data: request.data,
                },
            })
            .expect("data_manager");
    }

    /// wait_for_shutdown
    ///
    ///
    ///
    ///
    pub async fn wait_for_shutdown(&self) {
        println!("Data Manager is shutting down");

        while self.task_counter.load(Ordering::Relaxed) > 0 {
            print!(".");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        println!("Data Manager is shut down");
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
    ///
    ///
    ///
    pub async fn post_blob(&self, data: impl Into<Bytes>) -> post_blob::Handle {
        let (response_sender, response_receiver) = oneshot::channel();

        let request = Command::PostBlob(post_blob::Request {
            response_sender,
            data: data.into(),
        });

        self.command_sender.send(request).await.unwrap();

        post_blob::Handle { response_receiver }
    }
}

/// Command
///
///
///
///
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
        pub blob: Blob,
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

pub mod post_blob {
    use super::*;

    pub struct Request {
        pub response_sender: oneshot::Sender<Response>,
        pub data: Bytes,
    }

    #[derive(Debug)]
    pub struct Response {
        pub blob: Blob,
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
    use super::*;

    use bytes::Bytes;

    #[tokio::test]
    async fn test_fetcher() {
        let client =
            Arc::new(pleiades_api::Client::try_new("http://master.local/api/v0.5/").unwrap());

        // fetcher
        let (mut fetcher, fetcher_api) = fetcher::Fetcher::new(client);

        tokio::spawn(async move {
            fetcher.run().await;
        });

        // data manager

        let (mut data_manager, api) = DataManager::new(fetcher_api);

        tokio::spawn(async move {
            data_manager.run().await;
        });

        let data = Bytes::from("hello world");

        let mut handle = api.post_blob(data.clone()).await;

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let response = handle.recv_nowait().unwrap();

        let mut handle = api.get_blob(response.blob.id).await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let response = handle.recv_nowait().unwrap();

        assert_eq!(response.blob.data, data);
    }
}
