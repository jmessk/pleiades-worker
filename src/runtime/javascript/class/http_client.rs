use boa_engine::{
    class::{Class, ClassBuilder},
    job::NativeJob,
    js_string,
    object::builtins::{JsPromise, JsUint8Array},
    Context, JsData, JsResult, JsValue, NativeFunction,
};
use boa_gc::{empty_trace, Finalize, Trace};

use crate::runtime::{
    http, javascript::host_defined::HostDefined as _, RuntimeRequest, RuntimeResponse,
};

#[derive(Debug, Finalize, JsData)]
pub struct HttpClient {}

unsafe impl Trace for HttpClient {
    empty_trace!();
}

impl Class for HttpClient {
    const NAME: &'static str = "HttpClient";
    const LENGTH: usize = 0;

    fn data_constructor(
        _new_target: &JsValue,
        _args: &[JsValue],
        _context: &mut Context,
    ) -> JsResult<Self> {
        Ok(HttpClient {})
    }

    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        let fn_get = NativeFunction::from_fn_ptr(Self::get);

        class.method(js_string!("get"), 1, fn_get);

        Ok(())
    }
}

impl HttpClient {
    pub fn get(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let url = args
            .first()
            .unwrap()
            .to_string(context)
            .unwrap()
            .to_std_string_escaped();

        // Create a request object and insert it into the context
        RuntimeRequest::Http(http::Request::Get(url)).insert(context.realm());
        tracing::trace!("request inserted into context");

        let (promise, resolver) = JsPromise::new_pending(context);

        let job = NativeJob::new(move |context| {
            tracing::trace!("promise job called");

            let response = RuntimeResponse::extract(context.realm());

            let result = match response {
                Some(RuntimeResponse::Http(http::Response::Get(Some(body)))) => {
                    let array = JsUint8Array::from_iter(body, context)?;
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

    pub fn post(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let url = args
            .first()
            .unwrap()
            .to_string(context)
            .unwrap()
            .to_std_string_escaped();

        // Create a request object and insert it into the context
        RuntimeRequest::Http(http::Request::Get(url)).insert(context.realm());
        tracing::trace!("request inserted into context");

        let (promise, resolver) = JsPromise::new_pending(context);

        let job = NativeJob::new(move |context| {
            tracing::trace!("promise job called");

            let response = RuntimeResponse::extract(context.realm());

            let result = match response {
                Some(RuntimeResponse::Http(http::Response::Get(Some(body)))) => {
                    let array = JsUint8Array::from_iter(body, context)?;
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
