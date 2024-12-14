use std::{sync::Arc, time::Instant};

use bytes::Bytes;
use pleiades_worker::{
    contractor,
    pleiades_type::{Job, JobStatus},
    updater, Contractor, DataManager, Fetcher, Updater,
};
use tokio::{sync::mpsc, task::JoinSet};

#[tokio::main]
async fn main() {
    let client =
        Arc::new(pleiades_api::Client::try_new("http://pleiades.local/api/v0.5/").unwrap());

    let (mut fetcher, fetcher_api) = Fetcher::new(client.clone());
    let (mut data_manager, data_manager_api) = DataManager::new(fetcher_api);
    let (mut contractor, contractor_api) =
        Contractor::new(client.clone(), data_manager_api.clone(), 8);
    let (mut updater, updater_api) = Updater::new(client.clone(), data_manager_api);

    let fetcher = tokio::spawn(async move {
        fetcher.run().await;
    });
    let data_manager = tokio::spawn(async move {
        data_manager.run().await;
    });
    let contractor = tokio::spawn(async move {
        contractor.run().await;
    });
    let updater = tokio::spawn(async move {
        updater.run().await;
    });

    let worker_id = register_worker(&client).await;
    let (job_sender, job_receiver) = mpsc::channel(64);
    tokio::spawn(async move {
        loop_contractor(contractor_api, worker_id, job_sender).await;
    });

    let updater_loop = tokio::spawn(async move {
        updater_loop(updater_api, job_receiver).await;
    });

    // job_generator(client).await;

    updater_loop.await.unwrap();

    fetcher.await.unwrap();
    data_manager.await.unwrap();
    contractor.await.unwrap();
    updater.await.unwrap();
}

async fn register_worker(client: &Arc<pleiades_api::Client>) -> String {
    let request = pleiades_api::api::worker::register::Request::builder()
        .runtimes(&["pleiades+example"])
        .build();

    let response = client.call_api(&request).await.unwrap();

    response.worker_id
}

async fn loop_contractor(
    contractor_api: contractor::Controller,
    worker_id: String,
    job_sender: mpsc::Sender<(Job, Instant)>,
) {
    let mut join_set = JoinSet::new();

    for _ in 0..8 {
        let contractor_api = contractor_api.clone();
        let worker_id = worker_id.clone();
        let job_sender = job_sender.clone();

        join_set.spawn(async move {
            while let Some(job) = contractor_api
                .try_contract(worker_id.clone())
                .await
                .unwrap()
                .recv()
                .await
                .contracted
            {
                job_sender.send((job, Instant::now())).await.unwrap();
            }
        });
    }

    join_set.join_all().await;
}

async fn updater_loop(updater_api: updater::Controller, mut job_receiver: mpsc::Receiver<(Job, Instant)>) {
    while let Some((mut job, instant)) = job_receiver.recv().await {
        job.status = JobStatus::Finished(Some(Bytes::from("output")));

        let handle = updater_api.update_job(job).await;

        tokio::spawn(async move {
            handle.recv().await;
            println!("job finished in {:?}", instant.elapsed());
        });
    }
}

async fn job_generator(client: Arc<pleiades_api::Client>) {
    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(50));

    for _ in 0..300 {
        let client = client.clone();

        tokio::spawn(async move {
            client.generate_test_job().await.unwrap();
        });

        ticker.tick().await;
    }
}
