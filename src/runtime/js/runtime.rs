use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsResult, Module, NativeFunction, Source};
use boa_runtime::Console;
use bytes::Bytes;
use std::rc::Rc;

use crate::runtime::js::{host_defined, job_queue, module};
use crate::types::Job;

// #[derive(Default)]
pub struct Runtime {
    context: Context,
    job: Job,
}

impl Runtime {
    pub fn new(job: Job) -> Self {
        let mut runtime = Self {
            context: Self::init_context(),
            job,
        };

        // builtin
        runtime.register_builtin_properties();
        runtime.register_builtin_classes();
        runtime.register_builtin_functions();
        runtime.register_builtin_modules();

        // user-defined
        runtime.register_user_defined_functions().unwrap();
        runtime.register_job_context();
        runtime.register_entrypoint();

        runtime
    }

    fn init_context() -> Context {
        Context::builder()
            // .job_queue(Rc::new(job_queue::TokioJobQueue::new()))
            .job_queue(Rc::new(job_queue::SyncJobQueue::new()))
            .module_loader(Rc::new(module::CustomModuleLoader::new()))
            .build()
            .unwrap()
    }

    fn register_builtin_properties(&mut self) {
        let console = Console::init(&mut self.context);

        self.context
            .register_global_property(js_string!(Console::NAME), console, Attribute::all())
            .unwrap();
    }

    fn register_builtin_classes(&mut self) {
        use super::class;

        self.context.register_global_class::<class::Blob>().unwrap();
        self.context.register_global_class::<class::Nn>().unwrap();
        self.context
            .register_global_class::<class::HttpClient>()
            .unwrap();
    }

    fn register_builtin_functions(&mut self) {
        use super::function;

        self.context
            .register_global_builtin_callable(
                js_string!("getJobContext"),
                0,
                NativeFunction::from_fn_ptr(function::get_job_context),
            )
            .unwrap();

        self.context
            .register_global_builtin_callable(
                js_string!("setUserOutput"),
                1,
                NativeFunction::from_fn_ptr(function::set_user_output),
            )
            .unwrap();
    }

    fn register_builtin_modules(&mut self) {
        let pleiades = module::pleiades::get_module(&mut self.context);

        self.context
            .module_loader()
            .register_module(js_string!("pleiades"), pleiades);
    }

    /// ## Register user-defined function
    ///
    /// - The shape of user-defined function is as follows:
    ///
    /// ```js
    /// import { blob } from "pleiades"
    ///
    /// async function fetch(job) {
    ///     const inputData = blob.get(job.input.id);
    ///  
    ///    return "output";
    /// }
    ///
    /// export default fetch;
    /// ```
    fn register_user_defined_functions(&mut self) -> JsResult<()> {
        let source = Source::from_bytes(&self.job.lambda.code.data);
        let module = Module::parse(source, None, &mut self.context)?;

        self.context
            .module_loader()
            .register_module(js_string!("user"), module);

        Ok(())
    }

    fn register_job_context(&mut self) {
        self.context
            .realm()
            .host_defined_mut()
            .insert(host_defined::JobContext {
                id: self.job.input.id.clone(),
                data: self.job.input.data.clone(),
            });
    }

    fn register_entrypoint(&mut self) {
        let entry_point = r#"
            import fetch from "user";

            let job = getJobContext();
            setUserOutput(await fetch(job));
        "#;

        let source = Source::from_bytes(entry_point);
        let module = Module::parse(source, None, &mut self.context).unwrap();

        let _ = module.load_link_evaluate(&mut self.context);
    }

    pub fn run(&mut self) -> Option<Bytes> {
        // self.context.run_jobs();
        // context.run_jobs_async().await;
        // self.context
        //     .eval(Source::from_bytes("setOutput('inner eval')"))
        //     .unwrap();
        self.context.run_jobs();

        self.output()
    }

    fn output(&mut self) -> Option<Bytes> {
        let output_object = self.context.realm().host_defined();
        let output = output_object.get::<host_defined::UserOutput>();

        match output {
            Some(output) => output.data.clone(),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types;

    fn generate_sample_job(code: &'static str) -> Job {
        Job {
            id: "11111".into(),
            status: types::JobStatus::Assigned,
            lambda: types::Lambda {
                id: "22222".into(),
                code: types::Blob {
                    id: "33333".into(),
                    data: code.into(),
                },
            },
            input: types::Blob {
                id: "44444".into(),
                data: "this_is_input_data".into(),
            },
        }
    }

    #[test]
    fn test_class() {
        let code = r#"
        import { blob } from "pleiades"

        async function fetch(job) {
            let someData = await blob.get(job);
            console.log(someData);

            return "test_output"; 
        }

        export default fetch;
    "#;

        let job = generate_sample_job(code);

        let mut runtime = Runtime::new(job);

        let output = runtime.run();

        assert_eq!(output, Some(Bytes::from("test_output")));
    }
}
