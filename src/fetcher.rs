use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Semaphore};

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
    max_concurrency: usize,
    semaphore: Arc<Semaphore>,
}

impl Fetcher {
    /// new
    ///
    pub fn new(client: Arc<pleiades_api::Client>) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(128);

        let fetcher = Self {
            client,
            command_receiver,
            max_concurrency: 64,
            semaphore: Arc::new(Semaphore::new(64)),
        };

        let api = Controller { command_sender };

        (fetcher, api)
    }

    /// run
    ///
    pub async fn run(&mut self) {
        tracing::info!("running");

        while let Some(command) = self.command_receiver.recv().await {
            let client = self.client.clone();
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();

            tokio::spawn(async move {
                match command {
                    Command::DownloadBlob(request) => {
                        Self::task_download_blob(client, request).await
                    }
                    Command::UploadBlob(request) => Self::task_upload_blob(client, request).await,
                }

                drop(permit);
            });
        }

        self.wait_for_shutdown().await;
    }

    async fn wait_for_shutdown(&self) {
        tracing::debug!(
            "shutting down {} tasks",
            self.max_concurrency - self.semaphore.available_permits()
        );

        let _ = self
            .semaphore
            .acquire_many(self.max_concurrency as u32)
            .await;

        tracing::info!("shutdown");
    }

    /// task
    ///
    async fn task_download_blob(
        client: Arc<pleiades_api::Client>,
        request: download_blob::Request,
    ) {
        tracing::trace!("downloading blob: {}", request.blob_id);

        let download_request = pleiades_api::api::data::download::Request::builder()
            .data_id(&request.blob_id)
            .build();

        let download_response = client.call_api(&download_request).await;

        match download_response {
            Ok(response) => {
                tracing::debug!("downloaded blob: {}", request.blob_id);

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
                tracing::debug!("blob not found: {}", request.blob_id);

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
    pub async fn download_blob(&self, blob_id: String) -> download_blob::Handle {
        let (response_sender, response_receiver) = oneshot::channel();

        let request = Command::DownloadBlob(download_blob::Request {
            response_sender,
            blob_id,
        });

        if self.command_sender.capacity() == 0 {
            tracing::warn!("command queue is full");
        }

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

        if self.command_sender.capacity() == 0 {
            tracing::warn!("command queue is full");
        }

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
