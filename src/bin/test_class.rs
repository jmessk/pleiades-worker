use boa_engine::{
    builtins::promise::PromiseState,
    class::{Class, ClassBuilder},
    context::ContextBuilder,
    job::{FutureJob, JobQueue, NativeJob},
    js_string,
    module::ModuleLoader,
    property::Attribute,
    Context, JsArgs, JsData, JsNativeError, JsResult, JsString, JsValue, Module, NativeFunction,
    Source,
};
use boa_gc::{empty_trace, Finalize, Trace};
use boa_runtime::Console;
use std::{cell::RefCell, collections::VecDeque, future::Future, pin::Pin, rc::Rc, sync::Arc};
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
        // println!("enqueue_promise_job: {:?}", job);
        self.jobs.borrow_mut().push_back(job);
    }

    fn enqueue_future_job(&self, future: FutureJob, _context: &mut Context) {
        println!("enqueue_future_job");
        self.futures.borrow_mut().spawn_local(future);
    }

    fn run_jobs(&self, context: &mut Context) {
        let mut next_job = self.jobs.borrow_mut().pop_front();
        while let Some(job) = next_job {
            if job.call(context).is_err() {
                self.jobs.borrow_mut().clear();
                println!("run_jobs");
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
                    while !(self.jobs.borrow().is_empty() && self.futures.borrow().is_empty()) {
                        context.run_jobs();

                        if let Some(res) = self.futures.borrow_mut().join_next().await {
                            println!("run_jobs_async: {:?}", res);
                            context.enqueue_job(res.unwrap())
                        }
                    }
                })
                .await;
        })
    }
}

#[derive(Debug, Default)]
pub struct CustomModuleLoader;

impl ModuleLoader for CustomModuleLoader {
    fn load_imported_module(
        &self,
        _referrer: boa_engine::module::Referrer,
        specifier: JsString,
        finish_load: Box<dyn FnOnce(JsResult<Module>, &mut Context)>,
        context: &mut Context,
    ) {
        match specifier.to_std_string_escaped().as_str() {
            "pleiades" => {
                let js_code = r#"
                    const blob = new Blob();

                    export { blob };
                "#;

                let module = Module::parse(Source::from_bytes(js_code), None, context);

                finish_load(module, context);
            }
            _ => finish_load(
                Err(JsNativeError::typ()
                    .with_message("module import error!")
                    .into()),
                context,
            ),
        };
    }
}

#[tokio::main]
async fn main() {
    let client = pleiades_api::Client::try_new("https://mecrm.omochilab.jp/api/v0.5/ping").unwrap();
    let client = Arc::new(client);

    let (mut fetcher, api) = pleiades_worker::fetcher::Fetcher::new(client);
    tokio::spawn(async move { fetcher.run().await });

    // let js_code = r"
    //     async function hello() {
    //         console.log('hello');
    //     }

    //     console.log('start');
    //     // let s1 = await sleep(1000);

    //     new Promise((resolve) => {
    //         console.log('start promise 1');
    //         resolve();
    //     }).then(() => {
    //         console.log('end promise 1');
    //     });

    //     await hello();

    //     // Promise.all([s1, s2, s3]).then(() => {
    //     //     console.log('all done');
    //     // });

    //     console.log('end');
    // ";

    let js_code = r#"
        // async function fetch() {
        //     console.log('start fetch');
        //     await sleep(1000);
        //     console.log('end fetch');
        // }

        import { blob } from "pleiades";

        console.log(blob.get());
        "#;

    let entry_point = r#"
        await fetch();
    "#;

    let context = &mut create_context(api);

    let module = Module::parse(Source::from_bytes(js_code), None, context).unwrap();
    let promise = module.load_link_evaluate(context);

    // context.eval(Source::from_bytes(entry_point)).unwrap();
    // context.run_jobs_async().await;
    context.run_jobs();

    if let PromiseState::Rejected(value) = promise.state() {
        eprintln!("Uncaught {}", value.display())
    }
}

fn create_context(api: pleiades_worker::fetcher::Api) -> Context {
    let queue = MyJobQueue::new();

    // use boa_engine::job::SimpleJobQueue;
    // let queue = SimpleJobQueue::new();

    let mut context = ContextBuilder::new()
        .job_queue(Rc::new(queue))
        .module_loader(Rc::new(CustomModuleLoader {}))
        .build()
        .unwrap();

    let console = Console::init(&mut context);

    context
        .register_global_property(js_string!(Console::NAME), console, Attribute::all())
        .expect("the console object shouldn't exist yet");

    context
        .register_global_builtin_callable(
            js_string!("sleep"),
            1,
            NativeFunction::from_async_fn(sleep),
        )
        .expect("the sleep builtin shouldn't exist yet");

    context.register_global_class::<Blob>().unwrap();
    // context
    //     .register_global_builtin_callable(
    //         "get_blob".into(),
    //         1,
    //         unsafe { NativeFunction::from_closure(move |_this, args, context| {
    //             let blob_id = args.get_or_undefined(0).to_string(context).unwrap();

    //         }) },
    //     )
    //     .unwrap();

    context
}

fn sleep(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> impl Future<Output = JsResult<JsValue>> {
    let delay = args.get_or_undefined(0).to_u32(context).unwrap();
    async move {
        tokio::time::sleep(std::time::Duration::from_millis(u64::from(delay))).await;
        Ok(JsValue::undefined())
    }
}

#[derive(Debug, Finalize, JsData)]
pub struct Blob {}

unsafe impl Trace for Blob {
    empty_trace!();
}

impl Class for Blob {
    const NAME: &'static str = "Blob";

    fn data_constructor(
        new_target: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<Self> {
        Ok(Blob {})
    }

    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        let fn_get = NativeFunction::from_fn_ptr(Self::get);

        class.method(js_string!("get"), 1, fn_get);

        Ok(())
    }
}

impl Blob {
    pub fn get(_this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
        let output = js_string!("blob");
        Ok(output.into())
    }
}
