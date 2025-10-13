use clap::Parser;
// use core::num;
use pleiades_worker::scheduler::global_sched;
use pleiades_worker::{updater, WorkerConfig};
// use std::collections::HashMap;
use std::io::prelude::*;
use std::time::Instant;
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
    DataManager, Fetcher, PendingManager, Updater,
};

#[derive(Debug, clap::Parser)]
struct Arg {
    #[clap(long = "config")]
    config_path: Option<String>,
    // #[clap(long = "num_iteration", short = 'n')]
    // num_iteration: Option<usize>,
}

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
        Some(ref path) => WorkerConfig::from_path(path),
        None => unreachable!("config file is required"),
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
    // let num_host_cores = affinity::get_core_num();
    let num_all_cores = config.num_executor_cores + config.num_general_cores;
    let num_executors = config.num_executors;
    let num_executor_cores = config.num_executor_cores;
    let num_tokio_workers = config.num_general_cores;

    // println!("num_host_cores: {num_host_cores}");
    println!("num_all_cores: {num_all_cores}");
    println!("num_executors: {num_executors}");
    println!("num_executor_cores: {num_executor_cores}");
    println!("num_tokio_workers: {num_tokio_workers}");

    let mut runtime_builder = tokio::runtime::Builder::new_multi_thread();

    match config.affinity_mode.as_str() {
        "none" => {}
        "unfixed" => {
            runtime_builder
                .worker_threads(num_tokio_workers)
                .on_thread_start(move || {
                    static CORE_COUNT: AtomicUsize = AtomicUsize::new(0);
                    let count = CORE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                    // if count < num_tokio_workers {
                    //     // 後半のコアに割り当て
                    //     let last = num_executor_cores + num_tokio_workers;
                    //     let list = (num_tokio_workers..last).collect::<Vec<usize>>();

                    //     affinity::set_thread_affinity(&list).unwrap();
                    //     println!("tokio worker is set to core {list:?}");
                    // } else if count < num_tokio_workers + num_executors {
                    //     let list = (0..num_executor_cores).collect::<Vec<usize>>();

                    //     affinity::set_thread_affinity(&list).unwrap();
                    //     println!("executor is set to core {list:?}");
                    // }
                    // if count < num_executors {
                    //     let core_list = (0..num_executor_cores).collect::<Vec<usize>>();

                    if count < num_tokio_workers {
                        let last = num_executor_cores + num_tokio_workers;
                        let core_list = (num_executor_cores..last).collect::<Vec<usize>>();

                        affinity::set_thread_affinity(&core_list).unwrap();
                        println!("tokio worker is set to core {core_list:?}");
                    } else {
                        let core_list = (0..num_executor_cores).collect::<Vec<usize>>();

                        affinity::set_thread_affinity(&core_list).unwrap();
                        println!("executor is set to core {core_list:?}");
                    }
                });
        }
        "fixed" => {
            runtime_builder
                .worker_threads(num_tokio_workers)
                .on_thread_start(move || {
                    static CORE_COUNT: AtomicUsize = AtomicUsize::new(0);
                    let count = CORE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                    // if count < num_executors {
                    //     let core_id = count % num_executor_cores;
                    //     core_affinity::set_for_current(core_affinity::CoreId { id: core_id });
                    //     println!("executor is set to core {core_id:?}");
                    // } else {
                    //     // 後半のコアに割り当て
                    //     let last = num_executor_cores + num_tokio_workers;
                    //     let core_list = (num_executor_cores..last).collect::<Vec<usize>>();

                    //     affinity::set_thread_affinity(&core_list).unwrap();
                    //     println!("tokio worker is set to core {core_list:?}");
                    // }

                    if count < num_tokio_workers {
                        let last = num_executor_cores + num_tokio_workers;
                        let core_list = (num_executor_cores..last).collect::<Vec<usize>>();

                        affinity::set_thread_affinity(&core_list).unwrap();
                        println!("tokio worker is set to core {core_list:?}");
                    } else {
                        let core_id = (count - num_tokio_workers) % num_executor_cores;
                        core_affinity::set_for_current(core_affinity::CoreId { id: core_id });
                        println!("executor is set to core {core_id:?}");
                    }
                });
        }
        mode => panic!("invalid affinity mode: {mode}"),
    }
    let runtime = runtime_builder.enable_all().build().unwrap();

    runtime.block_on(async move {
        worker(args, config).await.join_all().await;
        // tokio::time::sleep(Duration::from_secs(10)).await;
    });
    // worker(config).await;
}

async fn worker(_args: Arg, config: WorkerConfig) -> JoinSet<()> {
    // let pleiades_url = std::env::var("PLEIADES_URL").unwrap();
    let client = Arc::new(pleiades_api::Client::try_new("http://example.com/").unwrap());
    // println!("{:?}", client.ping().await.unwrap());

    // Read script
    //
    let data = std::fs::read(&config.script_path).unwrap();
    let code = bytes::Bytes::from(data);
    //
    // ///////

    // Initialize components
    //
    let (mut fetcher, fetcher_controller) = Fetcher::new(client.clone());
    let (mut data_manager, data_manager_controller) = DataManager::new(fetcher_controller);
    // let (mut contractor, contractor_controller) = Contractor::new(
    //     client.clone(),
    //     data_manager_controller.clone(),
    //     config.num_contractors,
    //     config.job_deadline,
    // );
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
    // join_set.spawn(async move {
    //     contractor.run().await;
    // });
    join_set.spawn(async move {
        updater.run().await;
    });
    join_set.spawn(async move {
        pending_manager.run().await;
    });
    //
    // /////

    // tokio::time::sleep(Duration::from_secs(10)).await;

    // Initialize LocalSched and Executor
    //
    let mut local_sched_manager_builder = LocalSchedManager::builder();
    // let (notify_sender, notify_receiver) = tokio::sync::watch::channel(());

    (0..config.num_executors).for_each(|id| {
        let (mut executor, executor_controller) = Executor::new(id);
        let (mut local_sched, local_sched_controller) = LocalSched::new(
            id,
            executor_controller,
            updater_controller.clone(),
            pending_manager_controller.clone(),
            // notify_sender.clone(),
        );

        local_sched_manager_builder.insert(local_sched_controller);

        join_set.spawn_blocking(move || {
            executor.run();
        });

        let policy = match config.policy.as_str() {
            "cooperative" => local_sched::Policy::Cooperative,
            // "blocking" => local_sched::Policy::Blocking,
            _ => panic!("invalid policy"),
        };
        join_set.spawn(async move {
            local_sched.run(policy).await;
        });
    });

    let local_sched_manager = local_sched_manager_builder.build().unwrap();
    //
    // /////

    // let mut worker_id_manager = WorkerIdManager::new(client, config.job_deadline).await;
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

    // Initialize GlobalSched
    //
    let policy = match config.policy.as_str() {
        "cooperative" => local_sched::Policy::Cooperative,
        // "blocking" => local_sched::Policy::Blocking,
        _ => panic!("invalid policy"),
    };

    let (mut global_sched, global_sched_controller) = GlobalSched::new(
        // contractor_controller,
        local_sched_manager,
        // worker_id_manager,
        // notify_receiver,
        policy,
        code,
        config.clone(),
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
    // if let Some(_num_iteration) = args.num_iteration {
    let timestamp = chrono::Local::now().format("%Y_%m%d_%H-%M-%S");
    let dir = PathBuf::from(format!("./metrics/{timestamp}"));
    std::fs::create_dir_all(&dir).unwrap();

    let (start_notify_sender, start_notify_receiver) = tokio::sync::watch::channel(());
    let (stop_notify_sender, stop_notify_receiver) = tokio::sync::watch::channel(());

    let cpu_usage = tokio::spawn(save_system_metrics(
        dir.clone(),
        config.num_executor_cores,
        start_notify_receiver,
        stop_notify_receiver,
        config.sys_metrics_freq,
    ));

    let summary = save_request_metrics(
        config.clone(),
        dir.clone(),
        global_sched_controller.clone(),
        updater_controller,
        start_notify_sender,
        stop_notify_sender,
    )
    .await;

    cpu_usage.await.unwrap();
    save_summary(dir.clone(), config, summary).await;

    println!("metrics are saved to {}", dir.to_str().unwrap());
    // } else {
    //     tokio::signal::ctrl_c().await.unwrap();
    //     global_sched_controller.signal_shutdown_req().await;
    // }
    //
    // /////

    join_set
}

// impl Default for WorkerConfig {
//     fn default() -> Self {
//         Self {
//             // num_contractors: 1,
//             num_executors: 1,
//             num_cpus: 1,
//             affinity_mode: 0,
//             policy: "cooperative".to_string(),
//             // exec_deadline: Duration::from_millis(300),
//             // job_deadline: Duration::from_millis(100),
//             cpu_usage_freq: Duration::from_secs(1),
//         }
//     }
// }

async fn save_summary(
    dir: PathBuf,
    config: WorkerConfig,
    (elapsed, finished, cancelled, max_rps): (Duration, u64, u64, u32),
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
max_rps: {max_rps}
",
                elapsed = elapsed.as_millis(),
                sum = finished + cancelled,
            )
            .as_bytes(),
        )
        .unwrap();

    writer.flush().unwrap();

    // writer.write_all("elapsed_avr:\n".as_bytes()).unwrap();
    // runtime_elapsed_arv.iter().for_each(|(runtime, avr)| {
    //     writer
    //         .write_all(format!("  {runtime}: {avr}\n").as_bytes())
    //         .unwrap();
    // });

    //     writer.write_all("runtime:\n".as_bytes()).unwrap();
    //     runtime_sum
    //         .iter()
    //         .for_each(|(runtime, (count, elapsed, consumed))| {
    //             writer
    //                 .write_all(
    //                     format!(
    //                         r"
    //   {runtime}:
    //     num: {count}
    //     elapsed_avr: {elapsed}
    //     consumed_avr: {consumed}",
    //                         count = count,
    //                         elapsed = *elapsed as f64 / *count as f64,
    //                         consumed = *consumed as f64 / *count as f64,
    //                     )
    //                     .as_bytes(),
    //                 )
    //                 .unwrap();
    //         });
}

async fn save_request_metrics(
    config: WorkerConfig,
    dir: PathBuf,
    global_sched_controller: global_sched::Controller,
    mut updater_controller: updater::Controller,
    start_notify_sender: tokio::sync::watch::Sender<()>,
    stop_notify_sender: tokio::sync::watch::Sender<()>,
) -> (Duration, u64, u64, u32) {
    let request_metrics = File::create(dir.join("request_metrics.csv")).unwrap();
    let mut request_metrics = BufWriter::new(request_metrics);
    request_metrics
        .write_all(b"id,timestamp(us),status,elapsed(us),consumed_cpu(us)\n")
        .unwrap();

    let rps_metrics = File::create(dir.join("rps.csv")).unwrap();
    let mut rps_metrics = BufWriter::new(rps_metrics);
    rps_metrics.write_all(b"timestamp(s),rps\n").unwrap();

    let mut first_instant = None;
    let mut last_instant = None;
    // let mut count = 0;
    let mut finished = 0;
    let mut canceled = 0;

    let mut current_instant = Instant::now();
    let mut current_rps = 0;
    let mut max_rps = 0;

    //
    // let mut each_sum = [0u64; 6];
    // let mut runtime_sum = HashMap::<String, (u32, u64, u64)>::new();
    //
    let mut warmup_timer = tokio::time::interval(config.warmup.time);
    warmup_timer.tick().await;

    while let Some(n) = tokio::select! {
        _ = updater_controller.recv_metric() => Some(0),
        _ = tokio::signal::ctrl_c() => Some(1),
        _ = warmup_timer.tick() => None,
    } {
        match n {
            0 => {}
            1 => {
                global_sched_controller.signal_shutdown_req().await;
                stop_notify_sender.send(()).unwrap();

                return (Duration::ZERO, 0, 0, 0);
            }
            _ => unreachable!(),
        }
    }

    println!("start measurement");
    start_notify_sender.send(()).unwrap();

    let measure_time = config.measure.step_time * config.measure.steps;
    let mut measure_timer = tokio::time::interval(measure_time);
    measure_timer.tick().await;
    // let start_instant = std::time::Instant::now();

    let start_measurement = Instant::now();
    let mut count = 0;
    while let Some(metric) = tokio::select! {
        metric = updater_controller.recv_metric() => metric,
        _ = tokio::signal::ctrl_c() => None,
        _ = measure_timer.tick() => None,
    } {
        let Metric {
            id,
            // runtime,
            status,
            start,
            end,
            elapsed,
            consumed_cpu,
        } = metric;

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

        current_rps += 1;
        if 1 <= current_instant.elapsed().as_secs() {
            if max_rps < current_rps {
                max_rps = current_rps;
            }
            current_instant = Instant::now();
            rps_metrics
                .write_all(
                    format!(
                        "{timestamp},{current_rps}\n",
                        timestamp = (current_instant - start_measurement).as_secs(),
                    )
                    .as_bytes(),
                )
                .unwrap();
            current_rps = 0;
        }

        //
        // let runt = runtime.split('_').collect::<Vec<&str>>()[0];
        // let index = runt.chars().last().unwrap().to_digit(10).unwrap() as usize - 1;
        // each_sum[index] += elapsed.as_millis() as u64;

        // runtime_sum
        //     .entry(runtime.clone())
        //     .and_modify(|(count, elapsed_sum, consumed_sum)| {
        //         *count += 1;
        //         *elapsed_sum += elapsed.as_millis() as u64;
        //         *consumed_sum += consumed.as_millis() as u64;
        //     })
        //     .or_insert((1, elapsed.as_millis() as u64, consumed.as_millis() as u64));
        //

        request_metrics
            .write_all(
                format!(
                    "{id},{timestamp},{status},{elapsed},{consumed_cpu}\n",
                    timestamp = (end - start_measurement).as_micros(),
                    elapsed = elapsed.as_micros(),
                    consumed_cpu = consumed_cpu.as_micros(),
                )
                .as_bytes(),
            )
            .unwrap();

        count += 1;
        if 100 <= count {
            count = 0;
            request_metrics.flush().unwrap();
            rps_metrics.flush().unwrap();
        }
    }

    request_metrics.flush().unwrap();

    stop_notify_sender.send(()).unwrap();
    global_sched_controller.signal_shutdown_req().await;

    let elapsed = last_instant
        .map(|last| last - first_instant.unwrap())
        .unwrap_or(Duration::ZERO);

    (
        elapsed, finished, canceled,
        max_rps,
        // runtime_elapsed_avr,
        // runtime_consumed_avr,
        // runtime_sum,
    )
}

async fn save_system_metrics(
    dir: PathBuf,
    num_use_cpus: usize,
    mut start_notifier: tokio::sync::watch::Receiver<()>,
    mut stop_notifier: tokio::sync::watch::Receiver<()>,
    freq: Duration,
) {
    let file_name = dir.join("system_metrics.csv");
    let file = File::create(&file_name).unwrap();
    let mut writer = BufWriter::new(file);

    writer
        .write_all(b"timestamp(s),cpu_usage(%),memory(bytes),context_switch\n")
        .unwrap();

    let mut counter = 0;
    let pid = sysinfo::get_current_pid().unwrap();
    let mut system = sysinfo::System::new_all();
    let kind = sysinfo::ProcessRefreshKind::nothing().with_memory();
    let pids = [pid];
    let processes_to_update = sysinfo::ProcessesToUpdate::Some(&pids);

    start_notifier.changed().await.unwrap();
    let mut ticker = tokio::time::interval(freq);

    while {
        tokio::select! {
            _ = stop_notifier.changed() => false,
            _ = ticker.tick() => true,
        }
    } {
        /////////////////////////////////////////////////////////////////
        // CPU usage
        /////////////////////////////////////////////////////////////////
        system.refresh_cpu_usage();
        let cpu_list = system.cpus();

        let cpu_usage = cpu_list[0..num_use_cpus]
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .sum::<f32>()
            / num_use_cpus as f32;

        // writer
        //     .write_all(format!(",{cpu_usage}").as_bytes())
        //     .unwrap();

        /////////////////////////////////////////////////////////////////
        // Memory usage
        /////////////////////////////////////////////////////////////////
        // tokio::task::yield_now().await;

        // let start = Instant::now();
        // system.refresh_processes_specifics(processes_to_update, false, kind);
        // let process = system.process(pid).unwrap();
        // let used_memory = process.memory();

        // println!("{:?}", start.elapsed());
        // writer
        //     .write_all(format!(",{used_memory}").as_bytes())
        //     .unwrap();

        /////////////////////////////////////////////////////////////////
        // Context switch
        /////////////////////////////////////////////////////////////////

        /////////////////////////////////////////////////////////////////
        // finalize
        /////////////////////////////////////////////////////////////////

        let timestamp = counter * freq.as_secs();
        writer
            .write_all(format!("{timestamp},{cpu_usage}\n", cpu_usage = cpu_usage as u8).as_bytes())
            .unwrap();
        // writer.write_all(b"\n").unwrap();

        counter += 1;
        if counter == 500 {
            counter = 0;
            writer.flush().unwrap();
        }
    }

    writer.flush().unwrap();

    // tokio::time::sleep(Duration::from_secs(20)).await;
    // stop_notify.send(()).unwrap();

    // println!("cpu usage is saved to {file_name}");
}
