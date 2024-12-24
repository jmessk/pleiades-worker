use boa_engine::{
    class::{Class, ClassBuilder},
    job::NativeJob,
    js_string,
    object::builtins::{JsPromise, JsUint8Array},
    Context, JsData, JsResult, JsValue, NativeFunction,
};
use boa_gc::{empty_trace, Finalize, Trace};

use crate::runtime::javascript::host_defined::HostDefined as _;
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
            // .method(js_string!("get"), 1, NativeFunction::from_fn_ptr(Self::get))
            .method(js_string!("get"), 1, NativeFunction::from_fn_ptr(Self::get));

        Ok(())
    }
}

impl Blob {
    pub fn get(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let blob_id = args
            .first()
            .unwrap()
            .to_string(context)
            .unwrap()
            .to_std_string_escaped();

        // Create a request object and insert it into the context
        RuntimeRequest::Blob(blob::Request::Get(blob_id)).insert(context.realm());
        tracing::trace!("request inserted into context");

        let (promise, resolver) = JsPromise::new_pending(context);

        let job = NativeJob::new(move |context| {
            tracing::trace!("promise job called");

            let response = RuntimeResponse::extract(context.realm());

            let result = match response {
                Some(RuntimeResponse::Blob(blob::Response::Get(Some(blob)))) => {
                    let array = JsUint8Array::from_iter(blob.data, context)?;
                    tracing::trace!("response found: {:?}", array);

                    JsValue::from(array)
                }
                _ => {
                    tracing::trace!("response not found");
                    JsValue::undefined()
                }
            };

            resolver
                .resolve
                .call(&JsValue::undefined(), &[result], context)
        });

        context.job_queue().enqueue_promise_job(job, context);

        Ok(JsValue::from(promise))
    }
}
