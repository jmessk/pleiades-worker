use boa_engine::{
    module::ModuleLoader, Context, JsNativeError, JsResult, JsString, Module,
};
use boa_gc::GcRefCell;
use std::collections::HashMap;

pub mod pleiades;

#[derive(Debug, Default)]
pub struct CustomModuleLoader {
    inner: GcRefCell<HashMap<String, Module>>,
}

impl CustomModuleLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_module(&self, name: &str, module: Module) {
        self.inner.borrow_mut().insert(name.to_string(), module);
    }
}

impl ModuleLoader for CustomModuleLoader {
    fn load_imported_module(
        &self,
        _referrer: boa_engine::module::Referrer,
        specifier: JsString,
        finish_load: Box<dyn FnOnce(JsResult<Module>, &mut Context)>,
        context: &mut Context,
    ) {
        let module_name = specifier.to_std_string_escaped();
        // println!("load_imported_module: {}", module_name);
        let module = self.inner.borrow().get(&module_name).cloned();

        match module {
            Some(module) => {
                // println!("load_imported_module: found module");
                finish_load(Ok(module), context)
            }
            None => finish_load(
                Err(JsNativeError::typ()
                    .with_message("module import error")
                    .into()),
                context,
            ),
        }
    }
}
