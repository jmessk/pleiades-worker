use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use bytes::Bytes;
use pleiades_worker::{
    contractor,
    pleiades_type::{Job, JobStatus},
    updater, Contractor, DataManager, Fetcher, Updater,
};
use tokio::{sync::mpsc, task::JoinSet};

#[tokio::main]
async fn main() {
    // Load .env file
    //
    dotenvy::dotenv().expect(".env file not found");
    let pleiades_url = std::env::var("PLEIADES_URL").expect("PLEIADES_URL must be set");
    //
    // /////

    // Initialize tracing
    //
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("pleiades_worker=debug"))
        .init();
    //
    // /////

    let client = Arc::new(pleiades_api::Client::try_new(&pleiades_url).unwrap());

    let (mut fetcher, fetcher_controller) = Fetcher::new(client.clone());
    let (mut data_manager, data_manager_controller) = DataManager::new(fetcher_controller);
    let (mut contractor, contractor_controller) = Contractor::new(
        client.clone(),
        data_manager_controller.clone(),
        8,
        Duration::from_millis(100),
    );
    let (mut updater, updater_controller) = Updater::new(client.clone(), data_manager_controller, "");

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
        loop_contractor(contractor_controller, worker_id, job_sender).await;
    });

    let updater_loop = tokio::spawn(async move {
        updater_loop(updater_controller, job_receiver).await;
    });

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

    for i in 0..8 {
        let contractor_api = contractor_api.clone();
        let worker_id = worker_id.clone();
        let job_sender = job_sender.clone();

        join_set.spawn(async move {
            // while let Some(job) = contractor_api
            if let Some(job) = contractor_api
                .try_contract(worker_id.clone())
                .await
                .unwrap()
                .recv()
                .await
                .contracted
            {
                println!("contracotr {}: contracted", i);
                job_sender.send((job, Instant::now())).await.unwrap();
            }
        });

        tokio::time::sleep(Duration::from_secs(1)).await;

        // let handle = contractor_api
        //     .try_contract(worker_id.clone())
        //     .await
        //     .unwrap();

        // let job_sender = job_sender.clone();

        // join_set.spawn(async move {
        //     match handle.recv().await.contracted {
        //         Some(job) => {
        //             println!("contractor {}: contracted", i);
        //             job_sender.send((job, Instant::now())).await.unwrap();
        //         }
        //         None => {
        //             println!("contractor {}: no job", i);
        //         }
        //     }
        //     // println!("{:?}", handle.recv().await);
        // });
    }

    join_set.join_all().await;
}

async fn updater_loop(
    updater_api: updater::Controller,
    mut job_receiver: mpsc::Receiver<(Job, Instant)>,
) {
    while let Some((mut job, instant)) = job_receiver.recv().await {
        job.status = JobStatus::Finished(Some(Bytes::from("output")));

        updater_api.update_job(job).await;
        println!("job finished in {:?}", instant.elapsed());
    }
}
