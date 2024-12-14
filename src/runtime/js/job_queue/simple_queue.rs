use boa_engine::{
    job::{FutureJob, JobQueue, NativeJob},
    Context,
};
use std::{cell::RefCell, collections::VecDeque};

use crate::runtime::{js::host_defined::HostDefined as _, RuntimeRequest, RuntimeResponse};

#[derive(Default)]
pub struct SimpleJobQueue(RefCell<VecDeque<NativeJob>>);

impl SimpleJobQueue {
    /// Creates an empty `SimpleJobQueue`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl JobQueue for SimpleJobQueue {
    fn enqueue_promise_job(&self, job: NativeJob, _: &mut Context) {
        println!("enqueue_promise_job: enqueuing job");
        self.0.borrow_mut().push_back(job);
    }

    fn run_jobs(&self, context: &mut Context) {
        println!("run_jobs: running jobs: {:?}", self.0.borrow().len());

        // Yeah, I have no idea why Rust extends the lifetime of a `RefCell` that should be immediately
        // dropped after calling `pop_front`.
        let mut next_job = self.0.borrow_mut().pop_front();
        while let Some(job) = next_job {
            //
            if job.call(context).is_err() {
                self.0.borrow_mut().clear();
                return;
            };

            //
            if let Some(request) = RuntimeRequest::get_from_context(context.realm()) {
                println!("RuntimeRequest found !!!: {:?}", request);
                println!("return from run_jobs");

                return;
            }
            next_job = self.0.borrow_mut().pop_front();
        }

        println!("run_jobs: finished running jobs");
    }

    fn enqueue_future_job(&self, future: FutureJob, context: &mut Context) {
        println!("enqueue_future_job: enqueuing future job");

        // let host_defined = context.realm().host_defined();
        // host_defined.get_mut::<FutureJob>()

        let job = pollster::block_on(future);
        self.enqueue_promise_job(job, context);
    }
}
