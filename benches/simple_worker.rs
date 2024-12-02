// use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::sync::mpsc;

use pleiades_worker::{
    contractor, data_manager, fetcher,
    types::{self, JobMetadata},
    updater,
};

#[tokio::main]
async fn main() {
    let client = pleiades_api::Client::new("http://master.local/api/v0.5/").unwrap();
    let client = Arc::new(client);

    // components
    let (mut contractor, contractor_api) = contractor::Contractor::new(client.clone());
    let (mut fetcher, fetcher_api) = fetcher::Fetcher::new(client.clone());
    let (mut data_manager, data_manager_api) = data_manager::DataManager::new(fetcher_api);
    let (mut updater, updater_api) = updater::Updater::new(client.clone());

    // run
    tokio::spawn(async move {
        contractor.run().await;
    });

    tokio::spawn(async move {
        fetcher.run().await;
    });

    tokio::spawn(async move {
        data_manager.run().await;
    });

    tokio::spawn(async move {
        updater.run().await;
    });

    let worker_id = register(&client).await;

    let (job_sender, job_receiver) = mpsc::channel(64);

    tokio::spawn(job_contractor(contractor_api, worker_id, job_sender));
    tokio::spawn(job_spawner(data_manager_api, updater_api, job_receiver));
}

async fn register(client: &Arc<pleiades_api::Client>) -> String {
    let request = pleiades_api::api::worker::register::Request::builder()
        .runtimes(&["pleiades-example"])
        .build();

    let response = client.call_api(&request).await.unwrap();

    response.worker_id
}

async fn job_contractor(
    contractor_api: contractor::Api,
    worker_id: String,
    job_sender: mpsc::Sender<types::JobMetadata>,
) {
    for _ in 0..8 {
        let contractor_api = contractor_api.clone();
        let worker_id = worker_id.clone();
        let job_sender = job_sender.clone();

        tokio::spawn(async move {
            let worker_id = worker_id;

            loop {
                let handle = contractor_api.contract(worker_id.clone()).await;
                let job = handle.recv().await;

                if let Some(job) = job.contracted {
                    job_sender.send(job).await.unwrap();
                }
            }
        });
    }
}

async fn job_spawner(
    data_manager_api: data_manager::Api,
    updater_api: updater::Api,
    mut job_receiver: mpsc::Receiver<JobMetadata>,
) {
    while let Some(job) = job_receiver.recv().await {
        let data_manager_api = data_manager_api.clone();
        let updater_api = updater_api.clone();

        tokio::spawn(async move {
            let _input = data_manager_api.get_blob(job.input_id).await;

            let output_id = data_manager_api
                .post_blob("output")
                .await
                .recv()
                .await
                .blob_id;

            let _update_handle = updater_api.finish_job(job.id, output_id).await;
        });
    }
}
