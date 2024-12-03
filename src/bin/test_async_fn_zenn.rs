use std::{cell::RefCell, collections::VecDeque, future::Future, pin::Pin, rc::Rc};

use boa_engine::{
    job::{FutureJob, JobQueue, NativeJob},
    context::ContextBuilder, js_string, property::Attribute, Context, JsArgs, JsResult, JsValue, NativeFunction, Source
};
use boa_runtime::Console;

use tokio::task::JoinSet;

struct MyJobQueue {
    futures: RefCell<JoinSet<NativeJob>>,
    jobs: RefCell<VecDeque<NativeJob>>,
}

impl MyJobQueue {
    fn new() -> Self {
        Self {
            futures: RefCell::default(),
            jobs: RefCell::default(),
        }
    }
}

impl JobQueue for MyJobQueue {
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
    fn run_jobs_async<'a, 'ctx, 'fut>(&'a self, context: &'ctx mut Context) -> Pin<Box<dyn Future<Output = ()> + 'fut>>
    where
        'a: 'fut,
        'ctx: 'fut,
    {
        Box::pin(async {
            let local = tokio::task::LocalSet::new();
            local.run_until(async {
                while !(self.jobs.borrow().is_empty() && self.futures.borrow().is_empty()) {
                    context.run_jobs();

                    if let Some(res) = self.futures.borrow_mut().join_next().await {
                        context.enqueue_job(res.unwrap())
                    }
                }
            }).await;
        })
    }
}

fn sleep(_this: &JsValue, args: &[JsValue], context: &mut Context) -> impl Future<Output = JsResult<JsValue>> {
    let delay = args.get_or_undefined(0).to_u32(context).unwrap();
    async move {
        tokio::time::sleep(std::time::Duration::from_millis(u64::from(delay))).await;
        Ok(JsValue::undefined())
    }
}

#[tokio::main]
async fn main() {
    let js_code = r"
        (async () => {
            for (let sec of [1, 2, 3, 4, 5]) {
                await sleep(1000)
                console.log(sec)
            }
        })()
    ";

    let queue = MyJobQueue::new();

    let mut context = &mut ContextBuilder::new()
        .job_queue(Rc::new(queue))
        .build()
        .unwrap();

    let console = Console::init(&mut context);

    context
        .register_global_property(js_string!(Console::NAME), console, Attribute::all())
        .expect("the console object shouldn't exist yet");

    context
        .register_global_builtin_callable(js_string!("sleep"), 1, NativeFunction::from_async_fn(sleep))
        .expect("the sleep builtin shouldn't exist yet");

    let job = NativeJob::new(move |context| -> JsResult<JsValue> {
        context.eval(Source::from_bytes(js_code))
    });

    context.enqueue_job(job);
    context.run_jobs_async().await;
}
