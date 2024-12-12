use bytes::Bytes;
use std::time::{Duration, Instant};

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
    pub time_counter: Duration,
    pub deadline: Duration,
    pub id: String,
    pub lambda: Lambda,
    pub input: Blob,
}

impl Job {
    pub fn start_timer(&mut self) -> Instant {
        Instant::now()
    }

    pub fn stop_timer_and_check_timeout(&mut self, start: Instant) {
        self.time_counter += start.elapsed();
    }

    pub fn is_timeout(&self) -> bool {
        self.time_counter >= self.deadline
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
    Ready {
        context: RuntimeContext,
        response: RuntimeResponse,
    },
    Waiting {
        context: RuntimeContext,
        request: RuntimeRequest,
    },
    Finished(Bytes),
    Cancelled,
}
