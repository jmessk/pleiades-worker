use boa_engine::{
    module::ModuleLoader, Context, JsNativeError, JsResult, JsString, Module, Source,
};

const JS_CODE: &str = r#"
    const blob = new Blob();

    export { blob };
"#;

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
                let module = Module::parse(Source::from_bytes(JS_CODE), None, context);

                finish_load(module, context);
            }
            _ => finish_load(
                Err(JsNativeError::typ()
                    .with_message("module import error")
                    .into()),
                context,
            ),
        };
    }
}
