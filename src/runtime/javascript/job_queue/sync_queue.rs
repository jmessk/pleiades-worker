use boa_engine::{
    job::{FutureJob, JobQueue, NativeJob},
    Context, Source,
};
use std::{cell::RefCell, collections::VecDeque};

use crate::runtime::{javascript::host_defined::HostDefined as _, RuntimeRequest};

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
        self.0.borrow_mut().push_back(job);
    }

    fn run_jobs(&self, context: &mut Context) {
        // Yeah, I have no idea why Rust extends the lifetime of a `RefCell` that should be immediately
        // dropped after calling `pop_front`.
        let mut next_job = self.0.borrow_mut().pop_front();
        while let Some(job) = next_job {
            //
            //
            if job.call(context).is_err() {
                self.0.borrow_mut().clear();
                return;
            };

            //
            if RuntimeRequest::exists_in(context.realm()) {
                // context
                //     .eval(Source::from_bytes("$boa.gc.collect()"))
                //     .unwrap();
                // boa_gc::force_collect();
                // println!("run_jobs: RuntimeRequest found. return from run_jobs");
                tracing::trace!("RuntimeRequest found. return from run_jobs");
                return;
            }

            next_job = self.0.borrow_mut().pop_front();
        }

        tracing::trace!("job queue is empty");
    }

    fn enqueue_future_job(&self, _future: FutureJob, _context: &mut Context) {
        unimplemented!("FutureJob is not supported in SyncJobQueue");
    }
}
