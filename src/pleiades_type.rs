use bytes::Bytes;
use std::time::Duration;

use crate::runtime::{RuntimeContext, RuntimeRequest, RuntimeResponse};

/// Blob
///
///
///
///
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

/// Lambda
///
///
///
///
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

/// Job
///
///
///
///
#[derive(Debug, Default)]
pub struct Job {
    /// status of the job
    ///
    pub status: JobStatus,

    /// remaining time that the job can consume
    ///
    pub remaining_time: Duration,

    /// context of the job
    /// runtimes need to implement their own context
    ///
    pub context: Option<RuntimeContext>,

    /// id of the job
    ///
    pub id: String,

    /// lambda of the job
    ///
    pub lambda: Lambda,

    /// input of the job
    ///
    pub input: Blob,
}

impl Job {
    /// set_remaining_time
    /// 
    /// 
    pub fn sub_remaining_time(&mut self, duration: Duration) {
        self.remaining_time -= duration;
    }

    /// is_timeout
    /// 
    /// 
    pub fn is_timeout(&self) -> bool {
        self.remaining_time <= Duration::from_secs(0)
    }

    /// cancel
    /// 
    /// 
    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
    }
}

/// JobStatus
///
///
///
///
#[derive(Debug, Default)]
pub enum JobStatus {
    /// Assigned
    /// 
    /// Job will be this status when it is assigned to worker
    #[default]
    Assigned,

    /// Running
    /// 
    /// Status of the job when it is running
    Running,

    /// Finished
    /// 
    /// Job will be this status when it is returned from Pending Manager
    Ready(RuntimeResponse),

    /// Pending
    /// 
    /// Job will be this status between Executor and Pending Manager
    Pending(RuntimeRequest),

    /// Finished
    Finished(Option<Bytes>),

    /// Runtime Request is preceding
    Resolving,

    /// Cancelled
    Cancelled,
}
