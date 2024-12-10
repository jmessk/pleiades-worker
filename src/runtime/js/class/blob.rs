use std::future::Future;

use boa_engine::{
    builtins::promise::ResolvingFunctions,
    class::{Class, ClassBuilder},
    js_string,
    object::builtins::JsPromise,
    Context, JsData, JsError, JsNativeError, JsResult, JsValue, NativeFunction,
};
use boa_gc::{empty_trace, Finalize, Trace};

use crate::runtime::js::host_defined::{blob, HostDefined as _};

#[derive(Debug, Finalize, JsData)]
pub struct Blob {}

unsafe impl Trace for Blob {
    empty_trace!();
}

impl Class for Blob {
    const NAME: &'static str = "Blob";
    const LENGTH: usize = 0;

    fn data_constructor(
        _new_target: &JsValue,
        _args: &[JsValue],
        _context: &mut Context,
    ) -> JsResult<Self> {
        Ok(Blob {})
    }

    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        class
            .method(js_string!("get"), 1, NativeFunction::from_fn_ptr(Self::get))
            .method(
                js_string!("getAsync"),
                1,
                NativeFunction::from_async_fn(Self::get_async),
            );

        Ok(())
    }
}

impl Blob {
    pub fn get(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        println!("Blob.getPromise is called: {:?}", args);

        let blob_id = match args.first() {
            Some(blob_id) => blob_id.to_string(context)?.to_std_string_escaped(),
            None => {
                return Err(JsError::from_native(
                    JsNativeError::error().with_message("Blob.get: blob_id is required"),
                ))
            }
        };

        blob::get::Request { blob_id }.insert_to_context(context);

        let executor = |resolvers: &ResolvingFunctions, context: &mut Context| {
            let result = js_string!("getPromise").into();

            resolvers
                .resolve
                .call(&JsValue::undefined(), &[result], context)?;

            Ok(JsValue::undefined())
        };

        let promise = JsPromise::new(executor, context);

        Ok(promise.into())
    }

    pub fn get_async(
        _this: &JsValue,
        _args: &[JsValue],
        _context: &mut Context,
    ) -> impl Future<Output = JsResult<JsValue>> {
        async {
            let output = js_string!("blob_async");
            Ok(output.into())
        }
    }
}
