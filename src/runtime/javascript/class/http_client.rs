use boa_engine::{
    class::{Class, ClassBuilder},
    job::NativeJob,
    js_string,
    object::builtins::{JsPromise, JsUint8Array},
    Context, JsData, JsObject, JsResult, JsValue, NativeFunction,
};
use boa_gc::{empty_trace, Finalize, Trace};
use bytes::Bytes;

use crate::runtime::{
    http,
    javascript::{class::ByteData, host_defined::HostDefined as _},
    RuntimeRequest, RuntimeResponse,
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
        class.method(js_string!("get"), 1, NativeFunction::from_fn_ptr(Self::get));
        class.method(
            js_string!("post"),
            2,
            NativeFunction::from_fn_ptr(Self::post),
        );

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
                    tracing::trace!("response found: size: {:?} Bytes", body.len());
                    // let array = JsUint8Array::from_iter(body, context)?;
                    // JsValue::from(array)
                    let data = ByteData { inner: body };
                    JsValue::from(JsObject::from_proto_and_data(None, data))
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

        let body_obj = args.get(1).unwrap().to_object(context).unwrap();
        // let body: Bytes = JsUint8Array::from_object(body_obj)?.iter(context).collect();
        let body: Bytes = body_obj.downcast_ref::<ByteData>().unwrap().inner.clone();

        // Create a request object and insert it into the context
        RuntimeRequest::Http(http::Request::Post { url, body }).insert(context.realm());
        tracing::trace!("request inserted into context");

        let (promise, resolver) = JsPromise::new_pending(context);

        let job = NativeJob::new(move |context| {
            tracing::trace!("promise job called");

            let response = RuntimeResponse::extract(context.realm());

            let result = match response {
                Some(RuntimeResponse::Http(http::Response::Get(Some(body)))) => {
                    tracing::trace!("response found: {:?}", body.len());
                    // let array = JsUint8Array::from_iter(body, context)?;
                    // JsValue::from(array)
                    let data = ByteData { inner: body };
                    JsValue::from(JsObject::from_proto_and_data(None, data))
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
