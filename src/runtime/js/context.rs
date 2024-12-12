use boa_engine::property::Attribute;
use boa_engine::{js_string, JsResult, Module, NativeFunction, Source};
use boa_runtime::Console;
use bytes::Bytes;
use std::rc::Rc;

use crate::pleiades_type::Job;
use crate::runtime::js::{
    host_defined::{HostDefined as _, UserInput, UserOutput},
    job_queue, module,
};
use crate::runtime::Context;
use crate::runtime::{RuntimeRequest, RuntimeResponse};

#[derive(Debug)]
pub struct JsContext {
    pub context: boa_engine::Context,
}

unsafe impl Sync for JsContext {}
unsafe impl Send for JsContext {}

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
                js_string!("getUserInput"),
                0,
                NativeFunction::from_fn_ptr(function::get_user_input),
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
    fn register_user_defined_functions(&mut self, job: &Job) -> JsResult<()> {
        let source = Source::from_bytes(&job.lambda.code.data);
        let module = Module::parse(source, None, &mut self.context)?;

        self.context
            .module_loader()
            .register_module(js_string!("user"), module);

        Ok(())
    }

    fn register_user_input(&mut self, job: &Job) {
        self.context.realm().host_defined_mut().insert(UserInput {
            data: job.input.data.clone(),
        });
    }

    fn register_entrypoint(&mut self) {
        let entry_point = r#"
            import fetch from "user";

            let job = getUserInput();
            setUserOutput(await fetch(job));
        "#;

        let source = Source::from_bytes(entry_point);
        let module = Module::parse(source, None, &mut self.context).unwrap();

        let _ = module.load_link_evaluate(&mut self.context);
    }

    pub fn step(&mut self) -> Option<RuntimeRequest> {
        self.context.job_queue().run_jobs(&mut self.context);
        RuntimeRequest::get_from_context(&self.context)
    }

    pub fn set_response(&mut self, response: RuntimeResponse) {
        response.insert_into_context(&mut self.context);
    }

    pub fn get_output(&mut self) -> Option<Bytes> {
        match UserOutput::get_from_context(&self.context) {
            Some(UserOutput { data }) => data,
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class() {
        let code = r#"
        import { blob } from "pleiades"

        async function fetch(job) {
            let someData = await blob.get("12345");
            console.log(someData);

            return "test_output"; 
        }

        export default fetch;
    "#;

        let mut job = Job::default();
        job.lambda.code.data = code.into();

        let mut context = JsContext::init(&job);

        let request = context.step();
        println!("{:?}", request);

        let output = context.get_output();
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

        let mut job = Job::default();
        job.lambda.code.data = code.into();

        let mut context = JsContext::init(&job);

        // use crate::runtime::js::host_defined::blob;
        // blob::get::Response {
        //     data: Bytes::from("test_output"),
        // }
        // .insert_to_context(&mut context.context);

        context.step();

        println!("**********");

        context.step();
        let output = context.get_output();

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
        let mut job = Job::default();
        job.lambda.code.data = code.into();

        let mut context = JsContext::init(&job);

        // use crate::runtime::js::host_defined::blob;
        // blob::get::Response {
        //     data: Bytes::from("test_output"),
        // }
        // .insert_to_context(&mut context.context);

        context.context.run_jobs_async().await;
        // runtime.step();

        println!("**********");

        // runtime.step();
        let output = context.get_output();

        assert_eq!(output, Some("test_output".into()));
    }
}
