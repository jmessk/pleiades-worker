use clap::Parser;
use pleiades_worker::scheduler::global_sched;
use pleiades_worker::updater;
use std::collections::HashMap;
use std::io::prelude::*;
use std::{
    fs::File,
    io::BufWriter,
    path::PathBuf,
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};
use tokio::task::JoinSet;

use pleiades_worker::{
    executor::Executor,
    helper::LocalSchedManager,
    metric::Metric,
    scheduler::{local_sched, GlobalSched, LocalSched},
    Contractor, DataManager, Fetcher, PendingManager, Updater, WorkerIdManager,
};

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
        Some(ref path) => WorkerConfig::from_path(path),
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

    // let cpu_list = core_affinity::get_core_ids().unwrap();
    // let num_cores = cpu_list.len();
    let num_cores = affinity::get_core_num();
    let num_tokio_workers = num_cores - config.num_cpus;
    let num_executors = config.num_executors;
    let num_use_cores = config.num_cpus;

    println!("num_cores: {num_cores}");
    println!("num_tokio_workers: {num_tokio_workers}");
    println!("num_executors: {num_executors}");
    println!("num_cpus: {num_use_cores}");

    let mut runtime_builder = tokio::runtime::Builder::new_multi_thread();

    match config.affinity_mode {
        0 => {}
        1 => {
            runtime_builder
                .worker_threads(num_tokio_workers)
                .on_thread_start(move || {
                    static CORE_COUNT: AtomicUsize = AtomicUsize::new(0);
                    let count = CORE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                    if count < num_tokio_workers {
                        let first = num_cores - num_tokio_workers;
                        let list = (first..num_cores).collect::<Vec<usize>>();

                        affinity::set_thread_affinity(&list).unwrap();
                        println!("tokio worker is set to core {list:?}");
                    } else if count < num_tokio_workers + num_executors {
                        let list = (0..num_use_cores).collect::<Vec<usize>>();

                        affinity::set_thread_affinity(&list).unwrap();
                        println!("executor is set to core {list:?}");
                    }
                });
        }
        2 => {
            runtime_builder
                .worker_threads(num_tokio_workers)
                .on_thread_start(move || {
                    static CORE_COUNT: AtomicUsize = AtomicUsize::new(0);
                    let count = CORE_COUNT.load(std::sync::atomic::Ordering::SeqCst);
                    // println!("count: {count}");
                    if count < num_tokio_workers {
                        let id = num_cores
                            - CORE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                            - 1;
                        core_affinity::set_for_current(core_affinity::CoreId { id });
                        println!("tokio worker is set to core {}", id);
                    } else if count < num_tokio_workers + num_executors {
                        let id = (CORE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                            - num_tokio_workers)
                            % num_use_cores;
                        core_affinity::set_for_current(core_affinity::CoreId { id });
                        println!("executor is set to core {}", id);
                    }
                });
        }
        _ => panic!("invalid affinity"),
    }

    runtime_builder
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            worker(args, config).await.join_all().await;
        });
}

async fn worker(args: Arg, config: WorkerConfig) -> JoinSet<()> {
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
        Updater::new(client.clone(), data_manager_controller.clone(), true);
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

    let mut worker_id_manager = WorkerIdManager::new(client, config.job_deadline).await;
    // worker_id_manager
    //     .insert(
    //         "default",
    //         &[
    //             "pleiades+example",
    //             "js+compress",
    //             "js+resize",
    //             "js+fib",
    //             "js+gpu",
    //             "js+counter",
    //         ],
    //         config.job_deadline,
    //     )
    //     .await;
    worker_id_manager
        .insert("default", &["pleiades+example"], config.job_deadline)
        .await;

    // Initialize GlobalSched
    //
    let (mut global_sched, global_sched_controller) = GlobalSched::new(
        contractor_controller,
        local_sched_manager,
        worker_id_manager,
        notify_receiver,
        local_sched::Policy::Blocking,
    );

    join_set.spawn(async move {
        global_sched.run().await;
    });
    //
    // /////

    // metrics
    //
    // let (stop_notify_sender, mut stop_notify_receiver) = tokio::sync::watch::channel(());
    // join_set.spawn(save_cpu_usage(
    //     config.num_cpus,
    //     stop_notify_sender,
    //     global_sched_controller,
    // ));
    if let Some(num_iteration) = args.num_iteration {
        let timestamp = chrono::Local::now().format("%Y-%m%d-%H%M%S");
        let dir = PathBuf::from(format!("./metrics/{timestamp}"));
        std::fs::create_dir_all(&dir).unwrap();

        let (stop_notify_sender, stop_notify_receiver) = tokio::sync::watch::channel(());

        let cpu_usage = tokio::spawn(save_cpu_usage(
            dir.clone(),
            config.num_cpus,
            stop_notify_receiver,
            config.cpu_usage_freq,
        ));

        let summary = save_metrics(
            dir.clone(),
            global_sched_controller.clone(),
            updater_controller,
            num_iteration,
            config.clone(),
        )
        .await;

        stop_notify_sender.send(()).unwrap();
        cpu_usage.await.unwrap();
        save_summary(dir.clone(), config, summary).await;

        println!("metrics are saved to {}", dir.to_str().unwrap());
    } else {
        tokio::signal::ctrl_c().await.unwrap();
        global_sched_controller.signal_shutdown_req().await;
    }
    //
    // /////

    join_set
}

#[derive(Debug, clap::Parser)]
struct Arg {
    #[clap(long = "config")]
    config_path: Option<String>,

    #[clap(long = "num_iteration", short = 'n')]
    num_iteration: Option<usize>,
}

use duration_str::deserialize_duration;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct WorkerConfig {
    num_contractors: usize,
    num_executors: usize,
    num_cpus: usize,
    affinity_mode: usize,
    policy: String,
    #[serde(deserialize_with = "deserialize_duration")]
    exec_deadline: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    job_deadline: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    cpu_usage_freq: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            num_contractors: 1,
            num_executors: 1,
            num_cpus: 1,
            affinity_mode: 0,
            policy: "cooperative".to_string(),
            exec_deadline: Duration::from_millis(300),
            job_deadline: Duration::from_millis(100),
            cpu_usage_freq: Duration::from_secs(1),
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

async fn save_summary(
    dir: PathBuf,
    config: WorkerConfig,
    (elapsed, finished, cancelled, each_arv): (Duration, u64, u64, HashMap<String, f64>),
) {
    let file = File::create(dir.join("summary.yml")).unwrap();
    let mut writer = BufWriter::new(file);

    writer
        .write_all(serde_yaml::to_string(&config).unwrap().as_bytes())
        .unwrap();
    writer
        .write_all(
            format!(
                r"---
elapsed: {elapsed}
num_jobs: {sum}
finished: {finished}
cancelled: {cancelled}
",
                elapsed = elapsed.as_millis(),
                sum = finished + cancelled,
            )
            .as_bytes(),
        )
        .unwrap();

    writer.write_all("runtime_avr:\n".as_bytes()).unwrap();
    each_arv.iter().for_each(|(runtime, avr)| {
        writer
            .write_all(format!("  {runtime}: {avr}\n").as_bytes())
            .unwrap();
    });
    // each_arv.iter().enumerate().for_each(|(i, &x)| {
    //     let i = i + 1;
    //     writer
    //         .write_all(format!("  - test{i}: {x}\n").as_bytes())
    //         .unwrap();
    // });
}

async fn save_metrics(
    dir: PathBuf,
    global_sched_controller: global_sched::Controller,
    mut updater_controller: updater::Controller,
    num_iteration: usize,
    config: WorkerConfig,
) -> (Duration, u64, u64, HashMap<String, f64>) {
    let file = File::create(dir.join("metrics.csv")).unwrap();
    let mut file = BufWriter::new(file);

    file.write_all(b"id,runtime,status,elapsed,consumed\n")
        .unwrap();

    let mut first_instant = None;
    let mut last_instant = None;
    let mut count = 0;
    let mut finished = 0;
    let mut canceled = 0;

    //
    // let mut each_sum = [0u64; 6];
    let mut runtime_sum = HashMap::<String, (u32, u64)>::new();
    //

    // let mut last2 = None;

    // while let Some(metric) = updater_controller.recv_metric().await {
    while let Some(metric) = tokio::select! {
        metric = updater_controller.recv_metric() => metric,
        _ = tokio::signal::ctrl_c() => None,
    } {
        let Metric {
            id,
            runtime,
            status,
            start,
            end,
            elapsed,
            consumed,
        } = metric;

        let diff = end - last_instant.unwrap_or(start);

        // if count < config.num_cpus {
        //     count += 1;
        //     continue;
        // }

        if status == "Finished" {
            finished += 1;
        } else if status == "Canceled" {
            canceled += 1;
        }

        if first_instant.is_none() {
            first_instant = Some(start);
        } else if let Some(first) = first_instant {
            if start < first {
                first_instant = Some(start);
            }
        }

        // last_instant = Some(end);
        if last_instant.is_none() {
            last_instant = Some(end);
        } else if let Some(last) = last_instant {
            if last < end {
                last_instant = Some(end);
            }
        }

        //
        // let runt = runtime.split('_').collect::<Vec<&str>>()[0];
        // let index = runt.chars().last().unwrap().to_digit(10).unwrap() as usize - 1;
        // each_sum[index] += elapsed.as_millis() as u64;

        runtime_sum
            .entry(runtime.clone())
            .and_modify(|(count, sum)| {
                *count += 1;
                *sum += elapsed.as_millis() as u64;
            })
            .or_insert((1, elapsed.as_millis() as u64));
        //

        file.write_all(
            format!(
                "{id},{runtime},{status},{elapsed},{consumed},{diff}\n",
                elapsed = elapsed.as_millis(),
                consumed = consumed.as_millis(),
                diff = diff.as_millis(),
            )
            .as_bytes(),
        )
        .unwrap();
        file.flush().unwrap();

        count += 1;
        // if num_iteration - config.num_cpus == count {
        //     println!(
        //         "diff: {}, count: {}",
        //         num_iteration - config.num_cpus,
        //         count
        //     );
        //     last2 = Some(end);
        // }

        if num_iteration == count {
            // let diff: Duration = end - last2.unwrap();
            // println!("\n\n\ndiff: {}\n\n\n", diff.as_millis());
            break;
        }
    }

    global_sched_controller.signal_shutdown_req().await;
    let elapsed = last_instant
        .map(|last| last - first_instant.unwrap())
        .unwrap_or(Duration::ZERO);

    let runtime_avr = runtime_sum
        .into_iter()
        .map(|(runtime, (count, sum))| (runtime, sum as f64 / (count as f64)))
        .collect::<HashMap<String, f64>>();
    //

    (elapsed, finished, canceled, runtime_avr)
}

async fn save_cpu_usage(
    dir: PathBuf,
    num_use_cpus: usize,
    mut stop_notifier: tokio::sync::watch::Receiver<()>,
    freq: Duration,
) {
    let file_name = dir.join("cpu_usage.csv");
    let file = File::create(&file_name).unwrap();
    let mut writer = BufWriter::new(file);

    writer.write_all(b"timestamp, usage\n").unwrap();
    // (0..num_use_cpus).for_each(|i| {
    //     writer.write_all(format!(",core_{i}").as_bytes()).unwrap();
    // });
    // writer.write_all(b"\n").unwrap();

    let mut system = sysinfo::System::new_all();
    let mut ticker = tokio::time::interval(freq);
    let mut counter = 0u64;

    while {
        tokio::select! {
            _ = stop_notifier.changed() => false,
            _ = ticker.tick() => true,
        }
    } {
        system.refresh_cpu_usage();
        let cpu_list = system.cpus();
        // let mut sum = 0;

        writer.write_all(format!("{counter}").as_bytes()).unwrap();
        let usage = cpu_list[0..num_use_cpus]
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .sum::<f32>()
            / num_use_cpus as f32;

        writer.write_all(format!(",{usage}").as_bytes()).unwrap();

        writer.write_all(b"\n").unwrap();
        writer.flush().unwrap();

        // if 10 < counter && sum < 1 {
        //     tracing::info!("stop cpu usage monitoring: counter={counter}");
        //     break;
        // }

        counter += 1;
    }

    // tokio::time::sleep(Duration::from_secs(20)).await;
    // stop_notify.send(()).unwrap();

    // println!("cpu usage is saved to {file_name}");
}
