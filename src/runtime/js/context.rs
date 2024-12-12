use boa_engine::property::Attribute;
use boa_runtime::Console;
use bytes::Bytes;
use std::rc::Rc;

use crate::runtime::js::{host_defined, job_queue, module};
use crate::runtime::Context;
use crate::types::Job;

use super::host_defined::command;
use super::host_defined::{HostDefined, UserOutput};

#[derive(Debug)]
pub struct JsContext {
    pub context: boa_engine::Context,
}

impl Context for JsContext {
    fn init(job: &Job) -> Self {
        let mut runtime = Self {
            context: Self::init_context(),
        };

        // builtin
        runtime.register_builtin_properties();
        runtime.register_builtin_classes();
        runtime.register_builtin_functions();
        runtime.register_builtin_modules();

        // user-defined
        runtime.register_user_defined_functions(job).unwrap();
        runtime.register_user_input(job);
        runtime.register_entrypoint();

        runtime
    }
}

impl JsContext {
    fn init_context() -> boa_engine::Context {
        boa_engine::Context::builder()
            .job_queue(Rc::new(job_queue::TokioJobQueue::new()))
            // .job_queue(Rc::new(job_queue::SyncJobQueue::new()))
            .module_loader(Rc::new(module::CustomModuleLoader::new()))
            .build()
            .unwrap()
    }

    fn register_builtin_properties(&mut self) {
        let console = Console::init(&mut self.context);

        self.context
            .register_global_property(
                boa_engine::js_string!(Console::NAME),
                console,
                Attribute::all(),
            )
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
                boa_engine::js_string!("getUserInput"),
                0,
                boa_engine::NativeFunction::from_fn_ptr(function::get_user_input),
            )
            .unwrap();

        self.context
            .register_global_builtin_callable(
                boa_engine::js_string!("setUserOutput"),
                1,
                boa_engine::NativeFunction::from_fn_ptr(function::set_user_output),
            )
            .unwrap();
    }

    fn register_builtin_modules(&mut self) {
        let pleiades = module::pleiades::get_module(&mut self.context);

        self.context
            .module_loader()
            .register_module(boa_engine::js_string!("pleiades"), pleiades);
    }

    /// Register user-defined function
    ///
    /// - The shape of user-defined function is as follows:
    ///
    /// ```js
    /// import { blob } from "pleiades"
    ///
    /// async function fetch(job) {
    ///     const inputData = await blob.get(job.input.id);
    ///  
    ///    return "output";
    /// }
    ///
    /// export default fetch;
    /// ```
    fn register_user_defined_functions(&mut self, job: &Job) -> boa_engine::JsResult<()> {
        let source = boa_engine::Source::from_bytes(&job.lambda.code.data);
        let module = boa_engine::Module::parse(source, None, &mut self.context)?;

        self.context
            .module_loader()
            .register_module(boa_engine::js_string!("user"), module);

        Ok(())
    }

    fn register_user_input(&mut self, job: &Job) {
        self.context
            .realm()
            .host_defined_mut()
            .insert(host_defined::UserInput {
                id: job.input.id.clone(),
                data: job.input.data.clone(),
            });
    }

    fn register_entrypoint(&mut self) {
        let entry_point = r#"
            import fetch from "user";

            let job = getUserInput();
            setUserOutput(await fetch(job));
        "#;

        let source = boa_engine::Source::from_bytes(entry_point);
        let module = boa_engine::Module::parse(source, None, &mut self.context).unwrap();

        let _ = module.load_link_evaluate(&mut self.context);
    }

    pub fn step(&mut self) -> Option<command::RuntimeRequest> {
        self.context.job_queue().run_jobs(&mut self.context);
        command::RuntimeRequest::get_from_context(&mut self.context)
    }

    pub fn set_response(&mut self, response: command::RuntimeResponse) {
        response.insert_to_context(&mut self.context);
    }

    pub fn get_output(&mut self) -> Option<Bytes> {
        UserOutput::get_from_context(&self.context).unwrap().data
    }
}

#[cfg(test)]
mod tests {
    use host_defined::HostDefined;

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

        let mut runtime = JsContext::new(job);

        runtime.step();
        runtime.step();
        let output = runtime.output();

        assert_eq!(output, Some("test_output".into()));
    }

    #[test]
    fn test_output() {
        let code = r#"
        import { blob } from "pleiades"

        async function fetch(job) {
            let someData = await blob.getAsync(job);
            console.log(someData);

            return "test_output"; 
        }

        export default fetch;
    "#;

        let job = generate_sample_job(code);

        let mut runtime = JsContext::new(job);

        // runtime.step();

        use crate::runtime::js::host_defined::blob;
        blob::get::Response {
            data: Bytes::from("test_output"),
        }
        .insert_to_context(&mut runtime.context);

        runtime.step();

        println!("**********");

        runtime.step();
        let output = runtime.output();

        assert_eq!(output, Some("test_output".into()));
    }

    #[tokio::test]
    async fn test_tokio() {
        let code = r#"
        import { blob } from "pleiades"

        async function fetch(job) {
            let someData = await blob.getAsync(job);
            console.log(someData);

            return "test_output"; 
        }

        export default fetch;
    "#;

        let job = generate_sample_job(code);

        let mut runtime = JsContext::new(job);

        // runtime.step();

        use crate::runtime::js::host_defined::blob;
        blob::get::Response {
            data: Bytes::from("test_output"),
        }
        .insert_to_context(&mut runtime.context);

        runtime.context.run_jobs_async().await;
        // runtime.step();

        println!("**********");

        // runtime.step();
        let output = runtime.output();

        assert_eq!(output, Some("test_output".into()));
    }
}
