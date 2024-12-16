use boa_engine::{
    job::{FutureJob, JobQueue, NativeJob},
    Context,
};
use std::{cell::RefCell, collections::VecDeque};

use crate::runtime::{js::host_defined::HostDefined as _, RuntimeRequest};

#[derive(Default)]
pub struct SyncJobQueue(RefCell<VecDeque<NativeJob>>);

impl SyncJobQueue {
    /// Creates an empty `SimpleJobQueue`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl JobQueue for SyncJobQueue {
    fn enqueue_promise_job(&self, job: NativeJob, _: &mut Context) {
        // println!("enqueue_promise_job: enqueuing job");
        self.0.borrow_mut().push_back(job);
    }

    fn run_jobs(&self, context: &mut Context) {
        println!("run_jobs: running jobs: {:?}", self.0.borrow().len());

        // Yeah, I have no idea why Rust extends the lifetime of a `RefCell` that should be immediately
        // dropped after calling `pop_front`.
        let mut next_job = self.0.borrow_mut().pop_front();
        while let Some(job) = next_job {
            println!("run_jobs: running job loop");
            //
            if job.call(context).is_err() {
                self.0.borrow_mut().clear();
                return;
            };

            //
            if RuntimeRequest::exists_in_context(context.realm()) {
                println!("run_jobs: RuntimeRequest found !!!. return from run_jobs");
                return;
            }

            next_job = self.0.borrow_mut().pop_front();
        }

        println!("run_jobs: finished running jobs");
    }

    fn enqueue_future_job(&self, _future: FutureJob, _context: &mut Context) {
        println!("enqueue_future_job: enqueuing future job");
        println!("enqueue_future_job: unreachable code");
    }
}
