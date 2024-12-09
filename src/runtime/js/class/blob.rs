use std::future::Future;

use boa_engine::{
    builtins::promise::ResolvingFunctions,
    class::{Class, ClassBuilder},
    js_string,
    object::builtins::JsPromise,
    Context, JsData, JsError, JsNativeError, JsResult, JsValue, NativeFunction,
};
use boa_gc::{empty_trace, Finalize, Trace};
use bytes::Bytes;

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
        // let fn_get = NativeFunction::from_fn_ptr(Self::get);
        let fn_get = NativeFunction::from_async_fn(Self::get);

        class.method(js_string!("get"), 1, fn_get);

        Ok(())
    }
}

impl Blob {
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

    pub fn get_async(
        _this: &JsValue,
        _args: &[JsValue],
        _context: &mut Context,
    ) -> impl Future<Output = JsResult<JsValue>> {
        async {
            let output = js_string!("blob");
            Ok(output.into())
        }
    }

    pub fn get_promise(
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        // let blob_id = args
        //     .first()
        //     .unwrap()
        //     .to_string(context)?
        //     .to_std_string_escaped();

        let blob_id = match args.first() {
            Some(blob_id) => blob_id.to_string(context)?.to_std_string_escaped(),
            None => {
                return Err(JsError::from_native(
                    JsNativeError::error().with_message("Blob.get: blob_id is required"),
                ))
            }
        };

        let request = get::Request { blob_id };

        // context.realm().host_defined_mut().insert(value)

        let executor = move |resolvers: &ResolvingFunctions, context| {
            resolvers
                .resolve
                .call(&JsValue::undefined(), &[result], context)?;
            Ok(JsValue::undefined())
        };

        let promise = JsPromise::new(executor, context);

        Ok(promise.into())
    }
}

// #[derive(Debug, Finalize, JsData)]
// pub struct HostDefinedGet {
//     pub id: String,
// }

// unsafe impl Trace for HostDefinedGet {
//     empty_trace!();
// }

pub mod get {
    use super::*;

    #[derive(Debug, Finalize, JsData)]
    pub struct Request {
        pub blob_id: String,
    }

    unsafe impl Trace for Request {
        empty_trace!();
    }

    pub struct Response {
        pub data: Bytes,
    }
}
