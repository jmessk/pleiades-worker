use std::{cell::RefCell, collections::VecDeque, future::Future, pin::Pin};
use tokio::task::JoinSet;

use boa_engine::{
    job::{FutureJob, JobQueue, NativeJob},
    Context,
};

#[derive(Default)]
pub struct TokioJobQueue {
    futures: RefCell<JoinSet<NativeJob>>,
    jobs: RefCell<VecDeque<NativeJob>>,
}

impl TokioJobQueue {
    pub fn new() -> Self {
        Self {
            jobs: RefCell::default(),
            futures: RefCell::default(),
        }
    }
}

impl JobQueue for TokioJobQueue {
    fn enqueue_promise_job(&self, job: NativeJob, _context: &mut Context) {
        self.jobs.borrow_mut().push_back(job);
    }

    fn enqueue_future_job(&self, future: FutureJob, _context: &mut Context) {
        self.futures.borrow_mut().spawn_local(future);
    }

    fn run_jobs(&self, context: &mut Context) {
        while let Some(job) = self.jobs.borrow_mut().pop_front() {
            if job.call(context).is_err() {
                self.jobs.borrow_mut().clear();
                return;
            };
        }
    }

    fn run_jobs_async<'a, 'ctx, 'fut>(
        &'a self,
        context: &'ctx mut Context,
    ) -> Pin<Box<dyn Future<Output = ()> + 'fut>>
    where
        'a: 'fut,
        'ctx: 'fut,
    {
        Box::pin(async {
            let local = tokio::task::LocalSet::new();
            local
                .run_until(async {
                    while !(self.jobs.borrow().is_empty() && self.futures.borrow().is_empty()) {
                        context.run_jobs();

                        if let Some(res) = self.futures.borrow_mut().join_next().await {
                            context.enqueue_job(res.unwrap())
                        }
                    }
                })
                .await;
        })
    }
}
