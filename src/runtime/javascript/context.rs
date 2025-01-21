use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsResult, Module, NativeFunction, Source};
use boa_runtime::{Console, TextDecoder, TextEncoder};
use bytes::Bytes;
use std::rc::Rc;

use crate::runtime;
use crate::runtime::javascript::{
    host_defined::{HostDefined as _, UserInput, UserOutput},
    job_queue, module,
};
use crate::runtime::{RuntimeRequest, RuntimeResponse};

#[derive(Debug)]
pub struct JsContext {
    context: Box<Context>,
}

unsafe impl Sync for JsContext {}
unsafe impl Send for JsContext {}

impl runtime::Context for JsContext {
    fn init() -> Self {
        let mut context = Self::init_inner();

        // builtin
        Self::register_builtin_properties(&mut context);
        Self::register_builtin_classes(&mut context);
        Self::register_builtin_functions(&mut context);
        Self::register_builtin_modules(&mut context);

        Self {
            context: Box::new(context),
        }
    }
}

impl JsContext {
    fn init_inner() -> Context {
        Context::builder()
            // .job_queue(Rc::new(job_queue::TokioJobQueue::new()))
            .job_queue(Rc::new(job_queue::SyncJobQueue::new()))
            .module_loader(Rc::new(module::CustomModuleLoader::new()))
            .build()
            .unwrap()
    }

    fn register_builtin_properties(context: &mut Context) {
        let console = Console::init(context);

        context
            .register_global_property(js_string!(Console::NAME), console, Attribute::all())
            .unwrap();
    }

    fn register_builtin_classes(context: &mut Context) {
        use super::class;

        TextDecoder::register(context).unwrap();
        TextEncoder::register(context).unwrap();

        context.register_global_class::<class::Blob>().unwrap();
        context.register_global_class::<class::Ai>().unwrap();
        context
            .register_global_class::<class::HttpClient>()
            .unwrap();
        context.register_global_class::<class::ByteData>().unwrap();
    }

    fn register_builtin_functions(context: &mut Context) {
        use super::function;

        // getUserInput
        context
            .register_global_builtin_callable(
                js_string!("getUserInput"),
                0,
                NativeFunction::from_fn_ptr(function::get_user_input),
            )
            .unwrap();

        // setUserOutput
        context
            .register_global_builtin_callable(
                js_string!("setUserOutput"),
                1,
                NativeFunction::from_fn_ptr(function::set_user_output),
            )
            .unwrap();

        // sleep
        context
            .register_global_builtin_callable(
                js_string!("sleep"),
                1,
                NativeFunction::from_fn_ptr(function::sleep),
            )
            .unwrap();

        context
            .register_global_builtin_callable(
                js_string!("blockingSleep"),
                1,
                NativeFunction::from_fn_ptr(function::blocking_sleep),
            )
            .unwrap();

        context
            .register_global_builtin_callable(
                js_string!("yieldNow"),
                0,
                NativeFunction::from_fn_ptr(function::yield_now),
            )
            .unwrap();

        context
            .register_global_builtin_callable(
                js_string!("compress"),
                1,
                NativeFunction::from_fn_ptr(function::compress),
            )
            .unwrap();

        context
            .register_global_builtin_callable(
                js_string!("resize"),
                1,
                NativeFunction::from_fn_ptr(function::resize),
            )
            .unwrap();

        context
            .register_global_builtin_callable(
                js_string!("busy"),
                1,
                NativeFunction::from_fn_ptr(function::busy),
            )
            .unwrap();

        context
            .register_global_builtin_callable(
                js_string!("count"),
                1,
                NativeFunction::from_fn_ptr(function::count),
            )
            .unwrap();
    }

    fn register_builtin_modules(context: &mut Context) {
        let pleiades_module = module::pleiades::get_module(context);

        context
            .module_loader()
            .register_module(js_string!("pleiades"), pleiades_module.clone());

        pleiades_module.load_link_evaluate(context);
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
    pub fn register_user_defined_functions(&mut self, lambda: &Bytes) -> JsResult<()> {
        // let code_string = String::from_utf8_lossy(lambda);
        // let source = Source::from_bytes(code_string.as_bytes());
        let source = Source::from_bytes(lambda);
        let module = Module::parse(source, None, &mut self.context)?;

        self.context
            .module_loader()
            .register_module(js_string!("user"), module);

        self.register_entrypoint();

        Ok(())
    }

    pub fn register_user_input(&mut self, input: &Bytes) {
        self.context.realm().host_defined_mut().insert(UserInput {
            data: input.clone(),
        });
    }

    fn register_entrypoint(&mut self) {
        let entry_point = r#"
            import fetch from "user";

            try {
                let input = getUserInput();
                setUserOutput(await fetch(input));
            } catch (e) {
                console.error(e);
            }
        "#;

        let source = Source::from_bytes(entry_point);
        let module = Module::parse(source, None, &mut self.context).unwrap();

        let _ = module.load_link_evaluate(&mut self.context);
    }

    pub fn step(&mut self) -> Option<RuntimeRequest> {
        self.context.job_queue().run_jobs(&mut self.context);
        RuntimeRequest::extract(self.context.realm())
    }

    pub fn set_response(&mut self, response: RuntimeResponse) {
        response.insert(self.context.realm());
    }

    pub fn get_output(&mut self) -> Option<Bytes> {
        match UserOutput::extract(self.context.realm()) {
            Some(UserOutput { data }) => data,
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use runtime::Context as _;

    use crate::{pleiades_type::Blob, runtime::blob};

    use super::*;

    #[test]
    fn test_output() {
        let code: Bytes = r#"
            import { blob } from "pleiades"

            async function fetch(input) {
                console.log("fetch called");
                let someData = await blob.get("12345");

                console.log(someData);

                return "test_output"; 
            }

            export default fetch;
        "#
        .into();

        let input: Bytes = "test_input".into();

        let code = code.to_vec();
        let code = Bytes::from(code);
        println!("{:?}", code);

        let mut context = JsContext::init();
        context.register_user_defined_functions(&code).unwrap();
        context.register_user_input(&input);

        // test start

        let request = context.step().unwrap();
        println!("request: {:?}", request);

        println!("set runtime response");

        context.set_response(RuntimeResponse::Blob(blob::Response::Get(Some(Blob {
            data: "test_insert".into(),
            id: "12345".into(),
        }))));

        context.step();

        let output = context.get_output();

        assert_eq!(output, Some("test_output".into()));
    }
}
