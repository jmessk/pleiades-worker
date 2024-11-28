use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

use crate::types::Job;

///
///
///
///
///
///
///
///
pub struct Contractor {
    client: Arc<pleiades_api::Client>,

    /// Contractor API to fetch jobs
    ///
    ///
    request_receiver: mpsc::Receiver<(String, oneshot::Sender<Job>)>,
}

impl Contractor {
    pub fn new(client: Arc<pleiades_api::Client>) -> (Self, ContractorApi) {
        let (request_sender, request_receiver) = mpsc::channel(100);

        let contractor = Self {
            client,
            request_receiver,
        };

        (contractor, ContractorApi { request_sender })
    }

    pub async fn run(&mut self) {}
}

///
///
///
///
///
///
///
///
pub struct ContractorApi {
    request_sender: mpsc::Sender<(String, oneshot::Sender<Job>)>,
}

impl ContractorApi {
    pub async fn contract(&self, runtime: String) -> oneshot::Receiver<Job> {
        let (sender, receiver) = oneshot::channel();

        self.request_sender.send((runtime, sender)).await.unwrap();

        receiver
    }
}
