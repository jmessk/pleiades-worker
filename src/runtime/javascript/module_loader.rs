use boa_engine::{
    module::ModuleLoader,  Context, JsNativeError, JsResult, JsString,Module, Source
};

#[derive(Debug, Default)]
pub struct MyModuleLoader;

impl ModuleLoader for MyModuleLoader {
    fn load_imported_module(
        &self,
        _referrer: boa_engine::module::Referrer,
        specifier: JsString,
        finish_load: Box<dyn FnOnce(JsResult<Module>, &mut Context)>,
        context: &mut Context,
    ) {
        match specifier.to_std_string_escaped().as_str() {
            "pleiades" => {
                let js_code = "export default 'Hello'";
                
                let module = Module::parse(Source::from_bytes(js_code), None, context);

                finish_load(module, context);
            }
            _ => finish_load(Err(JsNativeError::typ().with_message("module import error!").into()), context)
        };
    }
}
