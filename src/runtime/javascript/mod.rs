pub mod class;
pub mod global_fn;
pub mod host_defined;
pub mod job_queue;
pub mod module_loader;

use boa_engine::{js_string, Context, Module, NativeFunction, Source};
use bytes::Bytes;
use std::rc::Rc;

use crate::types::Job;

// #[derive(Default)]
pub struct Runtime {
    context: Context,
    job: Job,
}

impl Runtime {
    pub fn new(job: Job) -> Self {
        Self {
            context: Self::init(&job),
            job,
        }
    }

    fn init(job: &Job) -> Context {
        let mut context = Context::builder()
            .job_queue(Rc::new(job_queue::TokioJobQueue::new()))
            .module_loader(Rc::new(module_loader::CustomModuleLoader {}))
            .build()
            .unwrap();

        // init console
        boa_runtime::Console::init(&mut context);

        // context.insert_data(host_defined::Blob {
        //     id: "0".to_string(),
        //     // data: bytes::Bytes::from("Hello, World!"),
        // });

        // host defined
        context
            .realm()
            .host_defined_mut()
            .insert(host_defined::InputBlob {
                id: job.id.clone(),
                // data: bytes::Bytes::from("Hello, World!"),
            })
            .unwrap();

        // register global class
        context.register_global_class::<class::Blob>().unwrap();
        context.register_global_class::<class::Nn>().unwrap();
        context
            .register_global_class::<class::HttpClient>()
            .unwrap();

        context
            .register_global_builtin_callable(
                js_string!("getJobContext"),
                0,
                NativeFunction::from_fn_ptr(global_fn::get_job_context),
            )
            .unwrap();

        context
    }

    pub fn run(&mut self) -> Option<Bytes> {
        // let code = String::from_utf8(self.job.lambda.code.data.to_vec()).unwrap();
        let context = &mut self.context;

        let module = Module::parse(
            Source::from_bytes(&self.job.lambda.code.data),
            None,
            context,
        )
        .unwrap();

        let _ = module.load_link_evaluate(context);
        context.run_jobs();

        self.output()
    }

    pub fn register_lambda() {
        
    }

    pub fn output(&mut self) -> Option<Bytes> {
        self.context
            .get_data::<host_defined::Output>()
            .unwrap()
            .data
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types;

    const CODE: &str = r#"
        import { blob } from "pleiades"

        async fn fetch(job) {
            let someData = blob.get(job.id);
            console.log(someData);

            return "output"; 
        }

        export default fetch;
    "#;

    #[test]
    fn test_runtime_run() {
        let job = Job {
            id: "0".to_string(),
            status: types::JobStatus::Assigned,
            lambda: types::Lambda {
                id: "0".to_string(),
                code: types::Blob {
                    id: "0".to_string(),
                    data: Bytes::from(CODE),
                },
            },
            input: types::Blob {
                id: "0".to_string(),
                data: Bytes::from(""),
            },
        };

        let mut runtime = Runtime::new(job);
        let output = runtime.run();
    }
}
