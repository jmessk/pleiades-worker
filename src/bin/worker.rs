use std::{
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use clap::Parser;
use core_affinity::CoreId;
use pleiades_worker::{
    executor::Executor, scheduler::Policy, Contractor, DataManager, ExecutorManager, Fetcher,
    PendingManager, Scheduler, Updater, WorkerIdManager,
};
use tokio::task::JoinSet;

static CPU_INCREMENT: OnceLock<Arc<Mutex<usize>>> = OnceLock::new();

// #[tokio::main(flavor = "current_thread")]
// #[tokio::main(flavor = "multi_thread")]
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
    // print with format
    println!("config: {config:#?}");
    //
    // /////

    // Initialize tracing
    //
    tracing_subscriber::fmt()
        // .with_env_filter(tracing_subscriber::EnvFilter::new("pleiades_worker=info"))
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    //
    // /////

    // Initialize tokio runtime and set core affinity
    //
    let cpu_list = core_affinity::get_core_ids().unwrap();
    CPU_INCREMENT.get_or_init(|| Arc::new(Mutex::new(0)));

    println!("cores: {}", cpu_list.len());

    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(cpu_list.len() - config.num_executors)
        .max_blocking_threads(config.num_executors)
        .on_thread_start(|| {
            let mut cpu_id_lock = CPU_INCREMENT.get().unwrap().lock().unwrap();
            let cpu_id = *cpu_id_lock;
            *cpu_id_lock += 1;
            println!("cpu_id: {cpu_id}");
            core_affinity::set_for_current(CoreId { id: cpu_id });
        })
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(worker(config));
}

async fn worker(config: WorkerConfig) {
    let pleiades_url = std::env::var("PLEIADES_URL").unwrap();
    let client = Arc::new(pleiades_api::Client::try_new(&pleiades_url).unwrap());

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

    // Initialize executor
    //
    let mut executor_manager_builder = ExecutorManager::builder();

    for i in 0..config.num_executors {
        let (mut executor, executor_controller) = Executor::new(i);
        executor_manager_builder.insert(executor_controller, config.exec_deadline);

        join_set.spawn_blocking(move || executor.run());
    }

    let executor_manager = executor_manager_builder.build().unwrap();
    //
    // /////

    let worker_id_manager = WorkerIdManager::new(client).await;

    println!("worker_id_manager");
    let (mut scheduler, scheduler_controller) = Scheduler::new(
        worker_id_manager,
        contractor_controller,
        updater_controller,
        pending_manager_controller,
        executor_manager,
    );

    join_set.spawn(async move {
        scheduler
            .run(match config.policy.as_str() {
                "fast_contract" => Policy::FastContract,
                "blocking_pipeline" => Policy::BlockingPipeline(config.job_deadline),
                "cooperative_pipeline" => Policy::CooperativePipeline(config.job_deadline),
                _ => panic!("unknown policy"),
            })
            .await;
    });

    tokio::signal::ctrl_c().await.unwrap();
    scheduler_controller.signal_shutdown_req().await;

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
