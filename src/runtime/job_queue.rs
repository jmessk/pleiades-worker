use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    future::Future,
    pin::Pin,
    rc::Rc,
    time::{Duration, Instant},
};

use boa_engine::{
    context::ContextBuilder,
    job::{FutureJob, JobQueue, NativeJob},
    js_string,
    native_function::NativeFunction,
    property::Attribute,
    Context, JsArgs, JsResult, JsValue, Source,
};

use tokio::task::JoinSet;

pub struct TokioJobQueue {
    jobs: RefCell<VecDeque<NativeJob>>,
    futures: RefCell<JoinSet<NativeJob>>,
}

impl TokioJobQueue {
    pub fn new() -> Self {
        Self {
            futures: RefCell::default(),
            jobs: RefCell::default(),
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
        let mut next_job = self.jobs.borrow_mut().pop_front();
        while let Some(job) = next_job {
            if job.call(context).is_err() {
                self.jobs.borrow_mut().clear();
                return;
            };
            next_job = self.jobs.borrow_mut().pop_front();
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
                    // ジョブとFutureが無くなるまでループする
                    while !(self.jobs.borrow().is_empty() && self.futures.borrow().is_empty()) {
                        // 溜まっているジョブを実行
                        context.run_jobs();

                        // Futureの完了を1つ待って終わったら結果(NativeJob)をキューに追加
                        if let Some(res) = self.futures.borrow_mut().join_next().await {
                            context.enqueue_job(res.unwrap())
                        }
                    }
                })
                .await;
        })
    }
}
