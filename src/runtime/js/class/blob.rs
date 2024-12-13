use boa_engine::{
    builtins::promise::ResolvingFunctions,
    class::{Class, ClassBuilder},
    js_string,
    object::builtins::{JsPromise, JsUint8Array},
    Context, JsData, JsError, JsNativeError, JsResult, JsValue, NativeFunction,
};
use boa_gc::{empty_trace, Finalize, Trace};
use std::future::Future;

use crate::runtime::js::host_defined::HostDefined as _;
use crate::runtime::{blob, RuntimeRequest, RuntimeResponse};

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
        RuntimeRequest::Blob(blob::Request::Get(blob_id)).insert_into_context(context);
        println!("Blob.get: request inserted into context");

        // need to delete
        RuntimeResponse::Blob(blob::Response::Get("test_insert".into()))
            .insert_into_context(context);

        let executor = |resolvers: &ResolvingFunctions, context: &mut Context| {
            println!("Blob.get: executor called");

            let response = RuntimeResponse::get_from_context(context);

            let result = match response {
                Some(RuntimeResponse::Blob(blob::Response::Get(data))) => {
                    let array = JsUint8Array::from_iter(data, context)?;
                    println!("Blob.get: response found: {:?}", array);

                    JsValue::from(array)
                }
                _ => {
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
        println!("Blob.getAsync: called: {:?}", args);

        // let blob_id = match args.first() {
        //     Some(blob_id) => blob_id.to_string(context)?.to_std_string_escaped(),
        //     None => {
        //         return Err(JsError::from_native(
        //             JsNativeError::error().with_message("Blob.get: blob_id is required"),
        //         ))
        //     }
        // };

        let blob_id = args
            .first()
            .unwrap()
            .to_string(context)
            .unwrap()
            .to_std_string_escaped();

        // Create a request object and insert it into the context
        RuntimeRequest::Blob(blob::Request::Get(blob_id)).insert_into_context(context);
        println!("Blob.getAsync: request inserted into context");

        async {
            // let response = RuntimeResponse::get_from_context(context);

            // let result = match response {
            //     Some(RuntimeResponse::Blob(blob::Response::Get(data))) => {
            //         let array = JsUint8Array::from_iter(data, context).unwrap();
            //         println!("Blob.getAsync: response found: {:?}", array);

            //         JsValue::from(array)
            //     }
            //     _ => {
            //         println!("Blob.getAsync: response not found");
            //         JsValue::undefined()
            //     }
            // };

            // Ok(result)
            Ok(JsValue::undefined())
        }
    }
}
