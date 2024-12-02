use bytes::Bytes;
use std::sync::{atomic::AtomicU32, Arc};
use tokio::sync::{mpsc, oneshot};

/// Contractor
///
///
///
///
pub struct Fetcher {
    client: Arc<pleiades_api::Client>,

    /// interface to access this component
    ///
    command_sender: mpsc::Sender<Command>,
    command_receiver: mpsc::Receiver<Command>,

    /// number of jobs currently being contracted
    ///
    task_counter: Arc<AtomicU32>,
}

impl Fetcher {
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

    pub fn api(&self) -> Api {
        Api {
            command_sender: self.command_sender.clone(),
        }
    }

    /// run
    ///
    pub async fn run(&mut self) {
        while let Some(request) = self.command_receiver.recv().await {
            let client = self.client.clone();
            let task_counter = self.task_counter.clone();

            tokio::spawn(async move {
                task_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                match request {
                    Command::DownloadBlob(request) => Self::task_download_blob(client, request).await,
                    Command::UploadBlob(request) => Self::task_upload_blob(client, request).await,
                }

                task_counter.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            });
        }

        self.wait_for_shutdown().await;
    }

    /// task
    ///
    async fn task_download_blob(client: Arc<pleiades_api::Client>, request: download_blob::Request) {
        let download_request = pleiades_api::api::data::download::Request::builder()
            .data_id(request.blob_id)
            .build();

        let download_response = client.call_api(&download_request).await;

        let download_response = download_response.expect("no error handling: download blob");
        // don't check error handling

        request
            .response_sender
            .send(download_blob::Response {
                data: download_response.data,
            })
            .expect("fetcher");
    }

    /// task
    ///
    async fn task_upload_blob(client: Arc<pleiades_api::Client>, request: upload_blob::Request) {
        let upload_request = pleiades_api::api::data::upload::Request::builder()
            .data(request.data)
            .build();

        let upload_response = client.call_api(&upload_request).await;

        let upload_response = upload_response.expect("no error handling: upload blob");

        request
            .response_sender
            .send(upload_blob::Response {
                blob_id: upload_response.data_id,
            })
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
        pub data: Bytes,
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
        pub blob_id: String,
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
        let client = Arc::new(pleiades_api::Client::new("http://master.local/api/v0.5/").unwrap());
        let mut fetcher = Fetcher::new(client);
        let api = fetcher.api();

        tokio::spawn(async move {
            fetcher.run().await;
        });

        let data = Bytes::from("hello world");

        let handle = api.upload_blob(data.clone()).await;
        let response = handle.recv().await;

        let handle = api.download_blob(response.blob_id).await;
        let response = handle.recv().await;

        assert_eq!(response.data, data);
    }
}
