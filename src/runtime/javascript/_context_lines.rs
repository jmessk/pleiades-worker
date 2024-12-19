use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsError, JsResult, Module, NativeFunction, Source};
use boa_runtime::Console;
use bytes::Bytes;
use std::io::BufRead;
use std::rc::Rc;

use crate::runtime::js::{
    host_defined::{HostDefined as _, UserInput, UserOutput},
    job_queue, module,
};

use crate::runtime;
use crate::runtime::{RuntimeRequest, RuntimeResponse};

#[derive(Debug)]
pub enum StepResult {
    Error,
    Finished(Option<Bytes>),
    Paused(RuntimeRequest),
}

#[derive(Debug)]
pub struct JsContext {
    pub context: Context,
    // pub lines: Vec<String>,
    pub line_iter: Option<std::vec::IntoIter<String>>,
}

unsafe impl Sync for JsContext {}
unsafe impl Send for JsContext {}

impl runtime::Context for JsContext {
    fn init() -> Self {
        let mut context = Self::init_context();

        // builtin
        Self::register_builtin_properties(&mut context);
        Self::register_builtin_classes(&mut context);
        Self::register_builtin_functions(&mut context);
        Self::register_builtin_modules(&mut context);

        Self {
            context,
            line_iter: None,
        }
    }
}

impl JsContext {
    fn init_context() -> Context {
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

        context.register_global_class::<class::Blob>().unwrap();
        context.register_global_class::<class::Nn>().unwrap();
        context
            .register_global_class::<class::HttpClient>()
            .unwrap();
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
    }

    fn register_builtin_modules(context: &mut Context) {
        let pleiades_module = module::pleiades::get_module(context);

        context
            .module_loader()
            .register_module(js_string!("pleiades"), pleiades_module.clone());

        pleiades_module.load_link_evaluate(context);
        context.run_jobs();

        let namespace = pleiades_module.namespace(context);
        let result = namespace.get(js_string!("pleiades"), context);
        println!("{:?}", result);
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
    pub fn register_user_defined_functions(&mut self, job: &Bytes) -> JsResult<()> {
        let source = Source::from_bytes(job);
        let module = Module::parse(source, None, &mut self.context)?;

        self.context
            .module_loader()
            .register_module(js_string!("user"), module);

        Ok(())
    }

    pub fn register_user_script(&mut self, code: &Bytes) {
        let lines_str: Vec<_> = code.lines().map(|line| line.unwrap()).collect();
        self.line_iter = Some(lines_str.into_iter());
    }

    pub fn register_user_input(&mut self, input: &Bytes) {
        self.context.realm().host_defined_mut().insert(UserInput {
            data: input.clone(),
        });
    }

    // fn register_entrypoint(&mut self) {
    //     let entry_point = r#"
    //         import fetch from "user";

    //         let job = getUserInput();
    //         setUserOutput(await fetch(job));
    //     "#;

    //     let source = Source::from_bytes(entry_point);
    //     let module = Module::parse(source, None, &mut self.context).unwrap();

    //     let _ = module.load_link_evaluate(&mut self.context);
    // }

    pub fn step(&mut self) -> StepResult {
        let iter = self.line_iter.as_mut().unwrap();

        for line in iter.by_ref() {
            let source = Source::from_bytes(&line);

            // if self.context.eval(source).is_err() {
            //     return StepResult::Error;
            // }

            match self.context.eval(source) {
                Ok(_) => {}
                Err(e) => {
                    dbg!(e);
                    return StepResult::Error;
                }
            }

            self.context.run_jobs();

            if let Some(request) = RuntimeRequest::get_from_context(self.context.realm()) {
                return StepResult::Paused(request);
            }

            if let Some(output) = UserOutput::get_from_context(&self.context.realm()) {
                return StepResult::Finished(output.data);
            }
        }

        StepResult::Finished(None)
    }

    pub fn set_response(&mut self, response: RuntimeResponse) {
        response.insert_into_context(self.context.realm());
    }

    pub fn get_output(&mut self) -> Option<Bytes> {
        match UserOutput::get_from_context(self.context.realm()) {
            Some(UserOutput { data }) => data,
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use runtime::Context as _;

    use super::*;

    #[test]
    fn a() {
        let code: Bytes = r#"
            const input = getUserInput();

            const blob = new Blob();
            const handle = blob.get(input.id);
            console.log(input);

            setUserOutput("test_output");
        "#
        .into();

        let input: Bytes = "test_input".into();

        let mut context = JsContext::init();

        context.register_user_script(&code);
        context.register_user_defined_functions(&code).unwrap();
        context.register_user_input(&input);

        let result = context.step();
        dbg!(result);
    }
}
