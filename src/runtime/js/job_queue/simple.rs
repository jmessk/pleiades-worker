use std::{cell::RefCell, collections::VecDeque};

use boa_engine::{
    job::{FutureJob, JobQueue, NativeJob},
    Context,
};

#[derive(Default)]
pub struct SimpleJobQueue(RefCell<VecDeque<NativeJob>>);

// impl Debug for SimpleJobQueue {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_tuple("SimpleQueue").field(&"..").finish()
//     }
// }

impl SimpleJobQueue {
    /// Creates an empty `SimpleJobQueue`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl JobQueue for SimpleJobQueue {
    fn enqueue_promise_job(&self, job: NativeJob, _: &mut Context) {
        self.0.borrow_mut().push_back(job);
    }

    fn run_jobs(&self, context: &mut Context) {
        // Yeah, I have no idea why Rust extends the lifetime of a `RefCell` that should be immediately
        // dropped after calling `pop_front`.
        let mut next_job = self.0.borrow_mut().pop_front();
        while let Some(job) = next_job {
            if job.call(context).is_err() {
                self.0.borrow_mut().clear();
                return;
            };
            next_job = self.0.borrow_mut().pop_front();
        }
    }

    fn enqueue_future_job(&self, future: FutureJob, context: &mut Context) {
        // let job = pollster::block_on(future);
        // self.enqueue_promise_job(job, context);
    }
}
