use clap::Parser;
// use core::num;
use pleiades_worker::scheduler::global_sched;
use pleiades_worker::{updater, WorkerConfig};
// use std::collections::HashMap;
use std::io::prelude::*;
use std::sync::Mutex;
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

    let num_all_cores = config.num_executor_cores + config.num_general_cores;
    let num_executors = config.num_executors;
    let num_executor_cores = config.num_executor_cores;
    let num_tokio_workers = config.num_general_cores;
    // let hyperthreads_executor = config.hyperthreads_executor;

    // println!("num_host_cores: {num_host_cores}");
    println!("num_all_cores: {num_all_cores}");
    println!("num_executors: {num_executors}");
    println!("num_executor_cores: {num_executor_cores}");
    println!("num_tokio_workers: {num_tokio_workers}");

    let mut runtime_builder = tokio::runtime::Builder::new_multi_thread();
    let tids = Arc::new(Mutex::new(Vec::new()));
    let tids_clone = tids.clone();

    match config.affinity_mode.as_str() {
        "none" => {}
        "unfixed" => {
            runtime_builder
                .worker_threads(num_tokio_workers)
                .on_thread_start(move || {
                    static CORE_COUNT: AtomicUsize = AtomicUsize::new(0);
                    let count = CORE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                    // 再開時はスキップ
                    if count >= num_tokio_workers + num_executors {
                        // println!("skip");
                        return;
                    }

                    if count < num_tokio_workers {
                        let core_list = [8, 9, 20, 21];
                        // let core_list = [8, 9, 10, 11, 20, 21, 22, 23];
                        affinity::set_thread_affinity(core_list).unwrap();
                        println!("tokio worker is set to core {core_list:?}");
                    } else {
                        let core_list = (0..num_executor_cores).collect::<Vec<usize>>();
                        affinity::set_thread_affinity(&core_list).unwrap();
                        println!("executor is set to core {core_list:?}");

                        let mut tids = tids.lock().unwrap();
                        tids.push(get_tid());
                    }
                });
        }
        "fixed" => {
            runtime_builder
                .worker_threads(num_tokio_workers)
                .on_thread_start(move || {
                    static CORE_COUNT: AtomicUsize = AtomicUsize::new(0);
                    let count = CORE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                    // 再開時はスキップ
                    if count >= num_tokio_workers + num_executors {
                        return;
                    }

                    if count < num_tokio_workers {
                        let core_list = [8, 9, 20, 21];
                        // let core_list = [8, 9, 10, 11, 20, 21, 22, 23];
                        affinity::set_thread_affinity(core_list).unwrap();
                        println!("tokio worker is set to core {core_list:?}");
                    } else {
                        let core_id = (count - num_tokio_workers) % num_executor_cores;
                        core_affinity::set_for_current(core_affinity::CoreId { id: core_id });
                        println!("executor is set to core {core_id:?}");

                        let mut tids = tids.lock().unwrap();
                        tids.push(get_tid());
                    }
                });
        }
        mode => panic!("invalid affinity mode: {mode}"),
    }
    let runtime = runtime_builder.enable_all().build().unwrap();

    runtime.block_on(async move {
        worker(args, config, tids_clone).await.join_all().await;
        // tokio::time::sleep(Duration::from_secs(10)).await;
    });
    // worker(config).await;
}

async fn worker(_args: Arg, config: WorkerConfig, tids: Arc<Mutex<Vec<i32>>>) -> JoinSet<()> {
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
            config.clone(),
        );

        local_sched_manager_builder.insert(local_sched_controller);

        join_set.spawn_blocking(move || {
            executor.run();
        });

        join_set.spawn(async move {
            local_sched.run().await;
        });
    });

    let local_sched_manager = local_sched_manager_builder.build().unwrap();

    let (mut global_sched, global_sched_controller) = GlobalSched::new(
        // contractor_controller,
        local_sched_manager,
        // worker_id_manager,
        // notify_receiver,
        // policy,
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
    let timestamp = chrono::Local::now().format("%Y_%m%d_%H-%M-%S");
    let dir = PathBuf::from(format!("./metrics/{timestamp}"));
    std::fs::create_dir_all(&dir).unwrap();

    let (start_notify_sender, start_notify_receiver) = tokio::sync::watch::channel(());
    let (stop_notify_sender, stop_notify_receiver) = tokio::sync::watch::channel(());

    let cpu_usage = tokio::spawn(save_system_metrics(
        dir.clone(),
        // config.num_executor_cores,
        start_notify_receiver,
        stop_notify_receiver,
        // config.sys_metrics_freq,
        config.clone(),
        tids,
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
    //
    // /////

    join_set
}

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
}

async fn save_request_metrics(
    config: WorkerConfig,
    dir: PathBuf,
    global_sched_controller: global_sched::Controller,
    mut updater_controller: updater::Controller,
    start_notify_sender: tokio::sync::watch::Sender<()>,
    stop_notify_sender: tokio::sync::watch::Sender<()>,
) -> (Duration, u64, u64, u32) {
    let request_metrics = File::create(dir.join("request.csv")).unwrap();
    let mut request_metrics = BufWriter::new(request_metrics);
    request_metrics
        .write_all(b"id,timestamp(us),status,elapsed(us),consumed_cpu(us)\n")
        .unwrap();

    let rps_metrics = File::create(dir.join("rps.csv")).unwrap();
    let mut rps_metrics = BufWriter::new(rps_metrics);
    rps_metrics
        .write_all(b"timestamp(s),rps,latency_avg(us)\n")
        .unwrap();

    let mut first_instant = None;
    let mut last_instant = None;
    // let mut count = 0;
    let mut finished = 0;
    let mut canceled = 0;

    let mut current_instant = Instant::now();
    let mut current_rps = 0;
    let mut max_rps = 0;

    let mut total_elapsed = Duration::ZERO;

    //
    // let mut each_sum = [0u64; 6];
    // let mut runtime_sum = HashMap::<String, (u32, u64, u64)>::new();
    //
    let mut warmup_timer = tokio::time::interval(config.warmup.time);
    warmup_timer.tick().await;

    while let Some(n) = tokio::select! {
        _ = updater_controller.recv_metric() => Some(0),
        // _ = tokio::signal::ctrl_c() => Some(1),
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
        // _ = tokio::signal::ctrl_c() => None,
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

        if start < start_measurement {
            continue;
        }

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
        total_elapsed += elapsed;

        if 1 <= current_instant.elapsed().as_secs() {
            if max_rps < current_rps {
                max_rps = current_rps;
            }

            let latency = if 0 < current_rps {
                total_elapsed.as_micros() / current_rps as u128
            } else {
                0
            };
            current_instant = Instant::now();
            rps_metrics
                .write_all(
                    format!(
                        "{timestamp},{current_rps},{latency}\n",
                        timestamp = (current_instant - start_measurement).as_secs(),
                    )
                    .as_bytes(),
                )
                .unwrap();
            current_rps = 0;
            total_elapsed = Duration::ZERO;
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
        if 1000 <= count {
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

    // while let Some(_metric) = updater_controller.recv_metric().await  {}
    while tokio::select! {
        _ = updater_controller.recv_metric() => true,
        _ = tokio::time::sleep(Duration::from_millis(100)) => false,
    } {}

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
    // num_use_cpus: usize,
    mut start_notifier: tokio::sync::watch::Receiver<()>,
    mut stop_notifier: tokio::sync::watch::Receiver<()>,
    // freq: Duration,
    config: WorkerConfig,
    tids: Arc<Mutex<Vec<i32>>>,
) {
    let file_name = dir.join("system.csv");
    let file = File::create(&file_name).unwrap();
    let mut writer = BufWriter::new(file);

    writer
        .write_all(b"timestamp(s),cpu_usage(%),context_switch(sum),voluntary,nonvoluntary\n")
        .unwrap();

    let mut counter = 0;
    let pid = sysinfo::get_current_pid().unwrap();
    let mut system = sysinfo::System::new_all();
    let kind = sysinfo::ProcessRefreshKind::nothing().with_memory();
    let pids = [pid];
    let processes_to_update = sysinfo::ProcessesToUpdate::Some(&pids);

    // let proc_file = File::open("/proc/self/status").unwrap();
    // let mut proc_reader = std::io::BufReader::new(proc_file);

    let (mut prev_voluntary, mut prev_nonvoluntary) = get_ctx_switch(&tids.lock().unwrap());

    start_notifier.changed().await.unwrap();
    let mut ticker = tokio::time::interval(config.sys_metrics_freq);
    while {
        tokio::select! {
            _ = stop_notifier.changed() => false,
            _ = ticker.tick() => true,
        }
    } {
        // let start = Instant::now();
        /////////////////////////////////////////////////////////////////
        // CPU usage
        /////////////////////////////////////////////////////////////////
        system.refresh_cpu_usage();
        let cpu_list = system.cpus();

        let cpu_usage = cpu_list[0..config.num_executor_cores]
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .sum::<f32>()
            / config.num_executor_cores as f32;

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

        let (voluntary, nonvoluntary) = get_ctx_switch(&tids.lock().unwrap());
        // let context_switch = voluntary + nonvoluntary;

        let dif_voluntary = voluntary - prev_voluntary;
        let dif_nonvoluntary = nonvoluntary - prev_nonvoluntary;
        let dif_context_switch = dif_voluntary + dif_nonvoluntary;

        prev_voluntary = voluntary;
        prev_nonvoluntary = nonvoluntary;

        /////////////////////////////////////////////////////////////////
        // finalize
        /////////////////////////////////////////////////////////////////

        let timestamp = counter * config.sys_metrics_freq.as_secs();
        writer
            .write_all(
                format!("{timestamp},{cpu_usage},{dif_context_switch},{dif_voluntary},{dif_nonvoluntary}\n"
                , cpu_usage = cpu_usage as u8)
                    .as_bytes(),
            )
            .unwrap();
        // writer.write_all(b"\n").unwrap();

        counter += 1;
        if counter == 500 {
            counter = 0;
            writer.flush().unwrap();
        }

        // let elapsed = start.elapsed();
        // println!("sys metrics: {elapsed:?}");
    }

    writer.flush().unwrap();

    // tokio::time::sleep(Duration::from_secs(20)).await;
    // stop_notify.send(()).unwrap();

    // println!("cpu usage is saved to {file_name}");
}

fn get_tid() -> libc::pid_t {
    unsafe { libc::syscall(libc::SYS_gettid) as libc::pid_t }
}

fn read_context_switch(tid: i32) -> (u64, u64) {
    let proc_file = File::open(format!("/proc/self/task/{}/status", tid)).unwrap();
    let proc_reader = std::io::BufReader::new(proc_file);

    let mut voluntary = 0;
    let mut nonvoluntary = 0;

    for line in proc_reader.lines() {
        let line = line.unwrap();
        if let Some(val) = line.strip_prefix("voluntary_ctxt_switches:") {
            voluntary = val.trim().parse().unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("nonvoluntary_ctxt_switches:") {
            nonvoluntary = val.trim().parse().unwrap_or(0);
            break;
        }
    }

    (voluntary, nonvoluntary)
}

fn get_ctx_switch(tids: &Vec<i32>) -> (u64, u64) {
    let mut voluntary = 0;
    let mut nonvoluntary = 0;

    for tid in tids {
        let (v, n) = read_context_switch(*tid);
        voluntary += v;
        nonvoluntary += n;
    }

    // println!("voluntary: {voluntary}, nonvoluntary: {nonvoluntary}");

    (voluntary, nonvoluntary)
}
