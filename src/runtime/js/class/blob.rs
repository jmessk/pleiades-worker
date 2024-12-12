use std::future::Future;

use boa_engine::{
    builtins::promise::ResolvingFunctions,
    class::{Class, ClassBuilder},
    js_string,
    object::builtins::{JsPromise, JsUint8Array},
    Context, JsData, JsError, JsNativeError, JsResult, JsValue, NativeFunction,
};
use boa_gc::{empty_trace, Finalize, Trace};

use crate::runtime::js::host_defined::{self, HostDefined as _};

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
        println!("Blob.get: called: {:?}", args);

        let blob_id = match args.first() {
            Some(blob_id) => blob_id.to_string(context)?.to_std_string_escaped(),
            None => {
                return Err(JsError::from_native(
                    JsNativeError::error().with_message("Blob.get: blob_id is required"),
                ))
            }
        };

        // Create a request object and insert it into the context
        host_defined::blob::get::Request { blob_id }.insert_to_context(context);
        println!("Blob.get: request inserted into context");

        let executor = |resolvers: &ResolvingFunctions, context: &mut Context| {
            println!("Blob.get: executor called");

            use crate::runtime::js::host_defined::blob;
            blob::get::Response {
                data: "test_output".into(),
            }
            .insert_to_context(context);

            blob::get::Request::remove_from_context(context);

            let response = host_defined::blob::get::Response::get_from_context(context);
            let result = match response {
                Some(response) => {
                    let array = JsUint8Array::from_iter(response.data, context)?;
                    println!("Blob.get: response found: {:?}", array);
                    JsValue::from(array)
                }
                None => {
                    println!("Blob.get: response not found");
                    JsValue::undefined()
                }
            };

            let _ = resolvers
                .resolve
                .call(&JsValue::undefined(), &[result], context)?;

            Ok(JsValue::undefined())
        };

        let promise = JsPromise::new(executor, context);

        Ok(promise.into())
    }

    pub fn get_async(
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> impl Future<Output = JsResult<JsValue>> {
        println!("Blob.get_async: called: {:?}", args);

        // let blob_id = match args.first() {
        //     Some(blob_id) => blob_id.to_string(context)?.to_std_string_escaped(),
        //     None => {
        //         return Err(JsError::from_native(
        //             JsNativeError::error().with_message("Blob.get: blob_id is required"),
        //         ))
        //     }
        // };

        // // Create a request object and insert it into the context
        // host_defined::blob::get::Request { blob_id }.insert_to_context(context);
        // println!("Blob.get: request inserted into context");

        async {
            println!("Blob.get_async: executor called");
            let output = js_string!("blob_async");
            Ok(output.into())
        }
    }
}
