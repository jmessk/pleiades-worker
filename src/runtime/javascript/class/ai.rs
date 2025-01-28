use std::{time::Duration};

use boa_engine::{
    class::{Class, ClassBuilder},
    job::NativeJob,
    js_string,
    object::builtins::JsPromise,
    Context, JsData, JsObject, JsResult, JsValue, NativeFunction,
};
use boa_gc::{empty_trace, Finalize, Trace};

use crate::runtime::{
    javascript::{class::ByteData, host_defined::HostDefined as _},
    RuntimeRequest, RuntimeResponse,
};

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
        class.method(
            js_string!("infer"),
            2,
            NativeFunction::from_fn_ptr(Self::infer),
        );

        Ok(())
    }
}

impl Ai {
    pub fn infer(
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let _model = args
            .first()
            .unwrap()
            .to_string(context)
            .unwrap()
            .to_std_string_escaped();

        let _input = args
            .get(1)
            .unwrap()
            .to_object(context)
            .unwrap()
            .downcast_ref::<ByteData>()
            .unwrap()
            .inner
            .clone();

        // Create a request object and insert it into the context
        RuntimeRequest::Sleep(Duration::from_millis(75 * 10)).insert(context.realm());
        tracing::trace!("request inserted into context");
        let (promise, resolver) = JsPromise::new_pending(context);

        let job = NativeJob::new(move |context| {
            tracing::trace!("promise job called");
            resolver
                .resolve
                .call(&JsValue::undefined(), &[JsValue::undefined()], context)
        });

        context.job_queue().enqueue_promise_job(job, context);

        Ok(JsValue::from(promise))
    }
}
