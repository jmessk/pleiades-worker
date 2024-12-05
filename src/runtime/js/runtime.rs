use boa_engine::{context, js_string, Context, Module, NativeFunction, Source};
use bytes::Bytes;
use std::rc::Rc;

use crate::runtime::js::{function, host_defined, job_queue, module_loader};
use crate::types::Job;

// #[derive(Default)]
pub struct Runtime {
    context: Context,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            context: Self::init_context(),
        }
    }
}

impl Runtime {
    fn init_context() -> Context {
        let mut context = Context::builder()
            .job_queue(Rc::new(job_queue::TokioJobQueue::new()))
            .module_loader(Rc::new(module_loader::CustomModuleLoader {}))
            .build()
            .unwrap();


        // init console
        boa_runtime::Console::init(&mut context);
        
        Self::register_class(&mut context);
        Self::register_function(&mut context);

        // context.insert_data(host_defined::Blob {
        //     id: "0".to_string(),
        //     // data: bytes::Bytes::from("Hello, World!"),
        // });

        // host defined
        // context
        //     .realm()
        //     .host_defined_mut()
        //     .insert(host_defined::InputBlob {
        //         id: job.id.clone(),
        //         // data: bytes::Bytes::from("Hello, World!"),
        //     });

        context
    }

    fn register_class(context: &mut Context) {
        use super::class;

        context.register_global_class::<class::Blob>().unwrap();
        context.register_global_class::<class::Nn>().unwrap();
        context
            .register_global_class::<class::HttpClient>()
            .unwrap();
    }

    fn register_function(context: &mut Context) {
        use super::function;

        context
            .register_global_builtin_callable(
                js_string!("getJobContext"),
                0,
                NativeFunction::from_fn_ptr(function::get_job_context),
            )
            .unwrap();

        context
            .register_global_builtin_callable(
                js_string!("setOutput"),
                1,
                NativeFunction::from_fn_ptr(function::set_output),
            )
            .unwrap();
    }

    // pub async fn run(&mut self) -> Option<Bytes> {
    //     // let code = String::from_utf8(self.job.lambda.code.data.to_vec()).unwrap();
    //     let context = &mut self.context;

    //     let module = Module::parse(
    //         Source::from_bytes(&self.job.lambda.code.data),
    //         None,
    //         context,
    //     )
    //     .unwrap();

    //     let _ = module.load_link_evaluate(context);
    //     // context.run_jobs();
    //     context.run_jobs_async().await;

    //     self.output()
    // }

    fn output(&mut self) -> Option<Bytes> {
        self.context
            .get_data::<host_defined::Output>()
            .unwrap()
            .data
            .clone()
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::types;

//     const CODE: &str = r#"
//         import { blob } from "pleiades"

//         async function fetch(job) {
//             let someData = blob.get(job.id);
//             console.log(someData);

//             return "output"; 
//         }

//         let job = getJobContext();

//         finishJob(await fetch(job));

//         export default fetch;
//     "#;

//     #[tokio::test]
//     // #[test]
//     async fn test_runtime_run() {
//         let job = Job {
//             id: "0".to_string(),
//             status: types::JobStatus::Assigned,
//             lambda: types::Lambda {
//                 id: "0".to_string(),
//                 code: types::Blob {
//                     id: "0".to_string(),
//                     data: Bytes::from(CODE),
//                 },
//             },
//             input: types::Blob {
//                 id: "0".to_string(),
//                 data: Bytes::from(""),
//             },
//         };

//         let mut runtime = JsContext::new(job);
//         let output = runtime.run().await;

//         assert_eq!(output, Some(Bytes::from("output")));
//     }
// }
