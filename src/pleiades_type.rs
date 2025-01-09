use bytes::Bytes;
use std::time::{Duration, Instant};

use crate::runtime::{RuntimeContext, RuntimeRequest, RuntimeResponse};

/// Blob
///
///
///
///
#[derive(Debug, PartialEq, Eq)]
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
#[derive(Debug, PartialEq, Eq)]
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
#[derive(Debug)]
pub struct Job {
    pub status: JobStatus,

    pub consumed: Duration,
    pub remaining: Duration,
    pub contracted_at: Instant,
    pub context: Option<RuntimeContext>,

    pub id: String,
    pub lambda: Box<Lambda>,
    pub input: Box<Blob>,
}

impl Default for Job {
    fn default() -> Self {
        Self {
            status: JobStatus::Assigned,
            consumed: Duration::ZERO,
            remaining: Duration::ZERO,
            context: None,

            id: "default".into(),
            lambda: Box::new(Lambda::default()),
            input: Box::new(Blob::default()),

            contracted_at: Instant::now(),
        }
    }
}

impl Job {
    /// set_remaining_time
    ///
    ///
    pub fn sub_remaining(&mut self, duration: Duration) {
        self.remaining = self
            .remaining
            .checked_sub(duration)
            .unwrap_or(Duration::ZERO);
    }

    pub fn add_consumed(&mut self, duration: Duration) {
        self.consumed = self.consumed.checked_add(duration).unwrap();
    }

    /// is_timeout
    ///
    ///
    pub fn is_timeout(&self) -> bool {
        self.remaining == Duration::ZERO
    }

    /// cancel
    ///
    ///
    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
        tracing::warn!("Job {} is cancelled", self.id);
    }

    pub fn is_cancelled(&self) -> bool {
        self.status == JobStatus::Cancelled
    }
}

/// JobStatus
///
///
///
///
#[derive(Debug, Default, PartialEq, Eq)]
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
