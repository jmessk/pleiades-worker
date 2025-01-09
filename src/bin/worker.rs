use clap::Parser;
use pleiades_worker::{
    executor::Executor,
    helper::LocalSchedManager,
    scheduler::{local_sched, GlobalSched, LocalSched},
    Contractor, DataManager, Fetcher, PendingManager, Updater, WorkerIdManager,
};
use std::{
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};
use tokio::task::JoinSet;
// use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt};

// #[tokio::main(flavor = "multi_thread")]
// async fn main() {
fn main() {
    // Load .env file
    //
    dotenvy::dotenv().unwrap();
    //
    // /////

    // Load worker configuration
    //
    let args = Arg::parse();
    let config = match args.config_path {
        Some(path) => WorkerConfig::from_path(path),
        None => WorkerConfig::default(),
    };

    println!("config: {config:#?}");
    //
    // /////

    // Initialize tracing
    //
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    // tracing_subscriber::registry()
    //     .with(tracing_subscriber::EnvFilter::from_default_env())
    //     .with(tracing_subscriber::fmt::layer())
    //     .with(console_subscriber::spawn())
    //     .try_init()
    //     .unwrap();

    // console_subscriber::init();
    //
    // /////

    let cpu_list = core_affinity::get_core_ids().unwrap();
    let num_cores = cpu_list.len();
    let num_tokio_workers = num_cores - config.num_executors;
    let num_executors = config.num_executors;

    println!("num_cores: {num_cores}");
    println!("num_tokio_workers: {num_tokio_workers}");
    println!("num_executors: {num_executors}");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(num_tokio_workers)
        // .max_blocking_threads(num_executors)
        .on_thread_start(move || {
            static CORE_COUNT: AtomicUsize = AtomicUsize::new(0);
            let count = CORE_COUNT.load(std::sync::atomic::Ordering::SeqCst);
            let id = num_cores
                - CORE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst) % num_cores
                - 1;

            // println!("count: {count}, id: {id}");

            if count < num_cores {
                core_affinity::set_for_current(core_affinity::CoreId { id });
                println!("thread is set to core {}", id);
            }
        })
        .build()
        .unwrap();

    runtime.block_on(worker(config));
    // worker(config).await;
}

async fn worker(config: WorkerConfig) {
    let pleiades_url = std::env::var("PLEIADES_URL").unwrap();
    let client = Arc::new(pleiades_api::Client::try_new(&pleiades_url).unwrap());
    println!("{:?}", client.ping().await.unwrap());

    // Initialize components
    //
    let (mut fetcher, fetcher_controller) = Fetcher::new(client.clone());
    let (mut data_manager, data_manager_controller) = DataManager::new(fetcher_controller);
    let (mut contractor, contractor_controller) = Contractor::new(
        client.clone(),
        data_manager_controller.clone(),
        config.num_contractors,
        config.job_deadline,
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

    // Initialize LocalSched and Executor
    //
    let mut local_sched_manager_builder = LocalSchedManager::builder();
    let (notify_sender, notify_receiver) = tokio::sync::watch::channel(());

    (0..config.num_executors).for_each(|id| {
        let (mut executor, executor_controller) = Executor::new(id);
        let (mut local_sched, local_sched_controller) = LocalSched::new(
            id,
            executor_controller,
            updater_controller.clone(),
            pending_manager_controller.clone(),
            notify_sender.clone(),
        );

        local_sched_manager_builder.insert(local_sched_controller, config.exec_deadline);

        join_set.spawn_blocking(move || {
            executor.run();
        });

        let policy = config.policy.clone();
        join_set.spawn(async move {
            local_sched
                .run(match policy.as_str() {
                    "cooperative" => local_sched::Policy::Cooperative,
                    "blocking" => local_sched::Policy::Blocking,
                    _ => panic!("invalid policy"),
                })
                .await;
        });
    });

    let local_sched_manager = local_sched_manager_builder.build().unwrap();
    //
    // /////

    // Initialize GlobalSched
    //
    let worker_id_manager = WorkerIdManager::new(client, config.job_deadline).await;

    let (mut global_sched, global_sched_controller) = GlobalSched::new(
        contractor_controller,
        local_sched_manager,
        worker_id_manager,
        notify_receiver,
    );

    join_set.spawn(async move {
        global_sched.run().await;
    });

    tokio::signal::ctrl_c().await.unwrap();
    global_sched_controller.signal_shutdown_req().await;

    drop(updater_controller);
    drop(pending_manager_controller);

    join_set.join_all().await;
}

#[derive(Debug, clap::Parser)]
struct Arg {
    #[clap(long = "config")]
    config_path: Option<String>,
}

use duration_str::deserialize_duration;

#[derive(Debug, serde::Deserialize)]
struct WorkerConfig {
    num_contractors: usize,
    num_executors: usize,
    policy: String,
    #[serde(deserialize_with = "deserialize_duration")]
    exec_deadline: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    job_deadline: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            num_contractors: 1,
            num_executors: 1,
            policy: "cooperative_pipeline".to_string(),
            exec_deadline: Duration::from_millis(300),
            job_deadline: Duration::from_millis(100),
        }
    }
}

impl WorkerConfig {
    fn from_path<P: AsRef<std::path::Path>>(path: P) -> Self {
        let file = std::fs::File::open(path).unwrap();
        let reader = std::io::BufReader::new(file);

        serde_yaml::from_reader(reader).unwrap()
    }
}
