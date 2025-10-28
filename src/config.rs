use duration_str::deserialize_duration;
use std::{path::PathBuf, time::Duration};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct WorkerConfig {
    // num_contractors: usize,
    pub num_executors: usize,
    pub num_executor_cores: usize,
    pub num_general_cores: usize,
    pub hyperthreads_executor: bool,

    pub affinity_mode: String,
    pub policy: String,
    // #[serde(deserialize_with = "deserialize_duration")]
    // exec_deadline: Duration,
    // #[serde(deserialize_with = "deserialize_duration")]
    // job_deadline: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub sys_metrics_freq: Duration,
    pub warmup: ConfigWarmup,
    pub measure: ConfigMeasure,

    // pub script_path: PathBuf,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ConfigWarmup {
    #[serde(deserialize_with = "deserialize_duration")]
    pub time: Duration,
    pub rps: u32,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ConfigMeasure {
    pub start_rps: u32,
    pub final_rps: u32,
    pub steps: u32,
    #[serde(deserialize_with = "deserialize_duration")]
    pub step_time: Duration,
}

impl WorkerConfig {
    pub fn from_path<P: AsRef<std::path::Path>>(path: P) -> Self {
        let file = std::fs::File::open(path).unwrap();
        let reader = std::io::BufReader::new(file);

        serde_yaml::from_reader(reader).unwrap()
    }
}
