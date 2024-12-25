use std::{sync::Arc, time::Duration};

use pleiades_worker::{
    executor::Executor, scheduler::Policy, Contractor, DataManager, ExecutorManager, Fetcher,
    PendingManager, Scheduler, Updater, WorkerIdManager,
};
use tokio::task::JoinSet;

const NUM_CONTRACTORS: usize = 8;
const NUM_EXECUTORS: usize = 4;

#[tokio::main]
async fn main() {
    // Load .env file
    //
    dotenvy::dotenv().unwrap();
    let pleiades_url = std::env::var("PLEIADES_URL").unwrap();
    //
    // /////

    // Initialize tracing
    //
    tracing_subscriber::fmt()
        // .with_env_filter(tracing_subscriber::EnvFilter::new("pleiades_worker=info"))
        .with_env_filter(tracing_subscriber::EnvFilter::new("pleiades_worker=debug"))
        // .with_env_filter(tracing_subscriber::EnvFilter::new("pleiades_worker=trace"))
        .init();
    //
    // /////

    // Initialize components
    //
    let client = Arc::new(pleiades_api::Client::try_new(&pleiades_url).unwrap());

    let (mut fetcher, fetcher_controller) = Fetcher::new(client.clone());
    let (mut data_manager, data_manager_controller) = DataManager::new(fetcher_controller);
    let (mut contractor, contractor_controller) = Contractor::new(
        client.clone(),
        data_manager_controller.clone(),
        NUM_CONTRACTORS,
    );
    let (mut updater, updater_controller) =
        Updater::new(client.clone(), data_manager_controller.clone());
    let (mut pending_manager, pending_manager_controller) =
        PendingManager::new(data_manager_controller);
    //
    // /////

    // Start components
    //
    let mut join_set = JoinSet::new();

    join_set.spawn(async move {
        fetcher.run().await;
    });
    join_set.spawn(async move {
        data_manager.run().await;
    });
    join_set.spawn(async move {
        contractor.run().await;
    });
    join_set.spawn(async move {
        updater.run().await;
    });
    join_set.spawn(async move {
        pending_manager.run().await;
    });
    //
    // /////

    // Initialize executor
    //
    let mut executor_manager_builder = ExecutorManager::builder();

    for i in 0..NUM_EXECUTORS {
        let (mut executor, executor_controller) = Executor::new(i);
        executor_manager_builder.insert(executor_controller, Duration::from_millis(300));

        join_set.spawn_blocking(move || executor.run());
    }

    let executor_manager = executor_manager_builder.build().unwrap();
    //
    // /////

    let worker_id_manager = WorkerIdManager::new(client).await;

    let (mut scheduler, scheduler_controller) = Scheduler::new(
        worker_id_manager,
        contractor_controller,
        updater_controller,
        pending_manager_controller,
        executor_manager,
    );

    join_set.spawn(async move {
        scheduler.run(Policy::BlockingPipeline).await;
    });

    tokio::signal::ctrl_c().await.unwrap();
    scheduler_controller.signal_shutdown_req().await;

    join_set.join_all().await;
}
