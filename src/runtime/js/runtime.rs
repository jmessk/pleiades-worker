use boa_engine::{js_string, Context, JsResult, Module, NativeFunction, Source};
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
        let loader = Rc::new(module::CustomModuleLoader::new());

        let context = Self::init_context(loader.clone());

        let mut runtime = Self { context, job };

        runtime.register_builtin_class();
        runtime.register_builtin_function();
        runtime.register_builtin_module(loader.clone());

        runtime.register_user_defined_function(loader).unwrap();
        runtime.register_job_context();
        runtime.finalize();

        runtime
    }

    fn init_context(loader: Rc<module::CustomModuleLoader>) -> Context {
        let mut context = Context::builder()
            .job_queue(Rc::new(job_queue::TokioJobQueue::new()))
            .module_loader(loader)
            .build()
            .unwrap();

        // init console
        boa_runtime::Console::init(&mut context);

        context
    }

    fn register_builtin_class(&mut self) {
        use super::class;

        self.context.register_global_class::<class::Blob>().unwrap();
        self.context.register_global_class::<class::Nn>().unwrap();
        self.context
            .register_global_class::<class::HttpClient>()
            .unwrap();
    }

    fn register_builtin_function(&mut self) {
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
                js_string!("setOutput"),
                1,
                NativeFunction::from_fn_ptr(function::set_output),
            )
            .unwrap();
    }

    fn register_builtin_module(&mut self, loader: Rc<module::CustomModuleLoader>) {
        let module = module::pleiades::get_module(&mut self.context);
        loader.add_module("pleiades", module);
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
    fn register_user_defined_function(
        &mut self,
        loader: Rc<module::CustomModuleLoader>,
    ) -> JsResult<()> {
        let source = Source::from_bytes(&self.job.lambda.code.data);
        let module = Module::parse(source, None, &mut self.context)?;

        loader.add_module("user", module);

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

    fn finalize(&mut self) {
        let entry_point = r#"
            import fetch from "user";

            let job = getJobContext();
            setOutput(await fetch(job));
        "#;

        let source = Source::from_bytes(entry_point);
        let module = Module::parse(source, None, &mut self.context).unwrap();

        let _ = module.load_link_evaluate(&mut self.context);
    }

    pub fn run(&mut self) -> Option<Bytes> {

        self.context.run_jobs();
        // context.run_jobs_async().await;

        self.output()
    }

    fn output(&mut self) -> Option<Bytes> {
        self.context
            .realm()
            .host_defined()
            .get::<host_defined::UserOutput>()
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

        async function fetch(job) {
            let someData = blob.get(job);
            return "output"; 
        }

        export default fetch;
    "#;

    fn new_job() -> Job {
        Job {
            id: "11111".to_string(),
            status: types::JobStatus::Assigned,
            lambda: types::Lambda {
                id: "22222".to_string(),
                code: types::Blob {
                    id: "33333".to_string(),
                    data: Bytes::from(CODE),
                },
            },
            input: types::Blob {
                id: "44444".to_string(),
                data: Bytes::from("this_is_input_data"),
            },
        }
    }

    #[test]
    fn test_runtime_run() {
        let job = new_job();

        let mut runtime = Runtime::new(job);

        let output = runtime.run();

        assert_eq!(output, Some(Bytes::from("output")));
    }
}
