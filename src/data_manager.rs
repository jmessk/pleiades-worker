use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Semaphore};

use crate::fetcher;
use crate::pleiades_type::Blob;

/// DataManager
///
///
///
///
pub struct DataManager {
    fetcher_controller: fetcher::Controller,

    /// interface to access this component
    ///
    command_receiver: mpsc::Receiver<Command>,

    /// number of jobs currently being contracted
    ///
    max_concurrency: usize,
    semaphore: Arc<Semaphore>,
}

impl DataManager {
    /// new
    ///
    ///
    ///
    ///
    pub fn new(fetcher_controller: fetcher::Controller) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(64);

        let data_manager = Self {
            fetcher_controller,
            command_receiver,
            max_concurrency: 64,
            semaphore: Arc::new(Semaphore::new(64)),
        };

        let controller = Controller { command_sender };

        (data_manager, controller)
    }

    /// run
    ///
    ///
    ///
    ///
    pub async fn run(&mut self) {
        tracing::info!("running");

        while let Some(command) = self.command_receiver.recv().await {
            let fetcher_controller = self.fetcher_controller.clone();
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();

            tokio::spawn(async move {
                match command {
                    Command::GetBlob(request) => {
                        Self::task_get_blob(fetcher_controller, request).await
                    }
                    Command::PostBlob(request) => {
                        Self::task_post_blob(fetcher_controller, request).await
                    }
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
    ///
    ///
    ///
    async fn task_get_blob(fetcher_controller: fetcher::Controller, request: get_blob::Request) {
        tracing::trace!("getting blob");

        let download_handle = fetcher_controller.download_blob(request.blob_id).await;
        let response = download_handle.recv().await;

        request
            .response_sender
            .send(get_blob::Response {
                blob: response.blob,
            })
            .expect("fetcher");

        tracing::debug!("got blob");
    }

    /// task
    ///
    ///
    ///
    ///
    async fn task_post_blob(fetcher_controller: fetcher::Controller, request: post_blob::Request) {
        tracing::trace!("posting blob");

        let upload_handle = fetcher_controller.upload_blob(request.data.clone()).await;
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

        tracing::debug!("posted blob");
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
    ///
    ///
    ///
    pub async fn get_blob(&self, blob_id: String) -> get_blob::Handle {
        let (response_sender, response_receiver) = oneshot::channel();

        let request = Command::GetBlob(get_blob::Request {
            response_sender,
            blob_id,
        });

        if self.command_sender.capacity() == 0 {
            tracing::warn!("command queue is full");
        }

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

        if self.command_sender.capacity() == 0 {
            tracing::warn!("command queue is full");
        }

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

pub mod get_blob {
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
        let (mut fetcher, fetcher_controller) = fetcher::Fetcher::new(client);

        tokio::spawn(async move {
            fetcher.run().await;
        });

        // data manager

        let (mut data_manager, api) = DataManager::new(fetcher_controller);

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

        assert_eq!(response.blob.unwrap().data, data);
    }
}
