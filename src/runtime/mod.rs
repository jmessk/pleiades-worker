use std::fmt::Debug;

use crate::types::{Job, JobStatus};

pub mod js;
pub mod python;

pub trait Runtime {
    fn init() -> Self;
    fn process(&mut self, job: Job) -> Job;
}

pub trait Context {
    fn init(job: &Job) -> Self;
}
