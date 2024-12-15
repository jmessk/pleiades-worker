use bytes::Bytes;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::{mpsc, oneshot};

use crate::pleiades_type::Blob;

/// Contractor
///
///
///
///
pub struct Fetcher {
    client: Arc<pleiades_api::Client>,

    /// interface to access this component
    ///
    command_receiver: mpsc::Receiver<Command>,

    /// number of jobs currently being contracted
    ///
    task_counter: Arc<AtomicUsize>,
}

impl Fetcher {
    /// new
    ///
    pub fn new(client: Arc<pleiades_api::Client>) -> (Self, Api) {
        let (command_sender, command_receiver) = mpsc::channel(64);

        let fetcher = Self {
            client,
            command_receiver,
            task_counter: Arc::new(AtomicUsize::new(0)),
        };

        let api = Api { command_sender };

        (fetcher, api)
    }

    /// run
    ///
    pub async fn run(&mut self) {
        while let Some(command) = self.command_receiver.recv().await {
            let client = self.client.clone();
            let task_counter = self.task_counter.clone();

            tokio::spawn(async move {
                task_counter.fetch_add(1, Ordering::Relaxed);

                match command {
                    Command::DownloadBlob(request) => {
                        Self::task_download_blob(client, request).await
                    }
                    Command::UploadBlob(request) => Self::task_upload_blob(client, request).await,
                }

                task_counter.fetch_sub(1, Ordering::Relaxed);
            });
        }

        self.wait_for_shutdown().await;
    }

    /// task
    ///
    async fn task_download_blob(
        client: Arc<pleiades_api::Client>,
        request: download_blob::Request,
    ) {
        let download_request = pleiades_api::api::data::download::Request::builder()
            .data_id(&request.blob_id)
            .build();

        let download_response = client.call_api(&download_request).await;

        match download_response {
            Ok(response) => {
                request
                    .response_sender
                    .send(download_blob::Response {
                        blob: Some(Blob {
                            id: request.blob_id,
                            data: response.data,
                        }),
                    })
                    .expect("fetcher");
            }
            Err(_) => {
                request
                    .response_sender
                    .send(download_blob::Response { blob: None })
                    .expect("fetcher");
            }
        }
    }

    /// task
    ///
    async fn task_upload_blob(client: Arc<pleiades_api::Client>, request: upload_blob::Request) {
        let upload_request = pleiades_api::api::data::upload::Request::builder()
            .data(request.data.clone())
            .build();

        let upload_response = client.call_api(&upload_request).await;

        let upload_response = upload_response.expect("no error handling: upload blob");

        request
            .response_sender
            .send(upload_blob::Response {
                blob: Blob {
                    id: upload_response.data_id,
                    data: request.data,
                },
            })
            .expect("fetcher");
    }

    /// wait_for_shutdown
    ///
    pub async fn wait_for_shutdown(&self) {
        println!("Fetcher is shutting down");

        while self.task_counter.load(Ordering::Relaxed) > 0 {
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
#[derive(Clone)]
pub struct Api {
    command_sender: mpsc::Sender<Command>,
}

impl Api {
    /// get_blob
    ///
    pub async fn download_blob(&self, blob_id: String) -> download_blob::Handle {
        let (response_sender, response_receiver) = oneshot::channel();

        let request = Command::DownloadBlob(download_blob::Request {
            response_sender,
            blob_id,
        });

        self.command_sender.send(request).await.unwrap();

        download_blob::Handle { response_receiver }
    }

    /// post_blob
    ///
    pub async fn upload_blob(&self, data: Bytes) -> upload_blob::Handle {
        let (response_sender, response_receiver) = oneshot::channel();

        let request = Command::UploadBlob(upload_blob::Request {
            response_sender,
            data,
        });

        self.command_sender.send(request).await.unwrap();

        upload_blob::Handle { response_receiver }
    }
}

pub enum Command {
    DownloadBlob(download_blob::Request),
    UploadBlob(upload_blob::Request),
}

// #[derive(Debug)]
// pub enum Response {
//     GetBlob { blob: Bytes },
//     PostBlob { blob_id: String },
// }

pub mod download_blob {
    use super::*;

    pub struct Request {
        pub response_sender: oneshot::Sender<Response>,
        pub blob_id: String,
    }

    #[derive(Debug)]
    pub struct Response {
        pub blob: Option<Blob>,
    }

    pub struct Handle {
        pub response_receiver: oneshot::Receiver<Response>,
    }

    impl Handle {
        pub async fn recv(self) -> Response {
            self.response_receiver.await.unwrap()
        }

        pub async fn recv_nowait(&mut self) -> Option<Response> {
            self.response_receiver.try_recv().ok()
        }
    }
}

pub mod upload_blob {
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
        let (mut fetcher, api) = Fetcher::new(client);

        tokio::spawn(async move {
            fetcher.run().await;
        });

        let data = Bytes::from("hello world");

        let handle = api.upload_blob(data.clone()).await;
        let response = handle.recv().await;

        let handle = api.download_blob(response.blob.id).await;
        let response = handle.recv().await;

        assert_eq!(response.blob.unwrap().data, data);
    }
}
