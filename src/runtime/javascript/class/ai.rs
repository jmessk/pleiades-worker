use std::future::Future;

use boa_engine::{
    class::{Class, ClassBuilder},
    js_string, Context, JsData, JsResult, JsValue, NativeFunction,
};
use boa_gc::{empty_trace, Finalize, Trace};

#[derive(Debug, Finalize, JsData)]
pub struct Ai {}

unsafe impl Trace for Ai {
    empty_trace!();
}

impl Class for Ai {
    const NAME: &'static str = "Ai";
    const LENGTH: usize = 0;

    fn data_constructor(
        _new_target: &JsValue,
        _args: &[JsValue],
        _context: &mut Context,
    ) -> JsResult<Self> {
        Ok(Ai {})
    }

    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        // let fn_get = NativeFunction::from_fn_ptr(Self::get);
        let fn_get = NativeFunction::from_async_fn(Self::get);

        class.method(js_string!("get"), 1, fn_get);

        Ok(())
    }
}

impl Ai {
    pub fn get(
        _this: &JsValue,
        _args: &[JsValue],
        _context: &mut Context,
    ) -> impl Future<Output = JsResult<JsValue>> {
        async {
            let output = js_string!("blob");
            Ok(output.into())
        }
    }
}
