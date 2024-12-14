use bytes::Bytes;
use std::time::Duration;

use crate::runtime::{RuntimeContext, RuntimeRequest, RuntimeResponse};

#[derive(Debug)]
pub struct BlobMetadata {
    pub id: String,
}

#[derive(Debug)]
pub struct LambdaMetadata {
    pub id: String,
    pub code: BlobMetadata,
}

#[derive(Debug)]
pub struct JobMetadata {
    pub id: String,
    pub lambda_id: String,
    pub input_id: String,
}

/////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Blob {
    pub id: String,
    pub data: Bytes,
}

impl Default for Blob {
    fn default() -> Self {
        Self {
            id: "default".into(),
            data: "default".into(),
        }
    }
}

#[derive(Debug)]
pub struct Lambda {
    pub id: String,
    pub runtime: String,
    pub code: Blob,
}

impl Default for Lambda {
    fn default() -> Self {
        Self {
            id: "default".into(),
            runtime: "default".into(),
            code: Blob::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Job {
    pub status: JobStatus,
    pub remaining_time: Duration,
    pub context: Option<RuntimeContext>,
    pub id: String,
    pub lambda: Lambda,
    pub input: Blob,
}

impl Job {
    pub fn sub_remaining_time(&mut self, duration: Duration) {
        self.remaining_time -= duration;
    }

    pub fn is_timeout(&self) -> bool {
        self.remaining_time <= Duration::from_secs(0)
    }

    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
    }
}

#[derive(Debug, Default)]
pub enum JobStatus {
    #[default]
    Assigned,
    Running,
    Ready(RuntimeResponse),
    Pending(RuntimeRequest),
    Finished(Option<Bytes>),
    Cancelled,
    Temporal,
}
