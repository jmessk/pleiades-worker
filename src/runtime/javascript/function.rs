use std::time::Duration;

use boa_engine::{
    job::NativeJob,
    object::builtins::{JsPromise, JsUint8Array},
    value::Type,
    Context, JsResult, JsValue,
};
use bytes::Bytes;

use crate::runtime::{
    javascript::host_defined::{HostDefined, UserInput, UserOutput},
    RuntimeRequest,
};

pub fn get_user_input(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    UserInput::extract(context.realm())
        .map(|input| {
            let array = JsUint8Array::from_iter(input.data, context)?;
            Ok(JsValue::from(array))
        })
        .unwrap_or_else(|| Ok(JsValue::undefined()))
}

pub fn set_user_output(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data_js_obj = args.first().expect("data is required");

    let data = match data_js_obj.get_type() {
        Type::Undefined | Type::Null => None,
        _ => {
            // let data = data_js_obj
            //     .to_string(context)
            //     .unwrap()
            //     .to_std_string_escaped();
            let data = data_js_obj.to_object(context)?;
            let data: Vec<u8> = JsUint8Array::from_object(data)?.iter(context).collect();

            Some(Bytes::from(data))
        }
    };

    UserOutput { data }.insert(context.realm());

    Ok(JsValue::undefined())
}

pub fn sleep(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    // let callable = match args.first() {
    //     Some(callback) => match callback.as_callable() {
    //         Some(callback) => callback.clone(),
    //         None => return Err(JsError::from_native(JsNativeError::ERROR)),
    //     },
    //     None => return Err(JsError::from_native(JsNativeError::ERROR)),
    // };

    // let ms = match args.get(1) {
    //     Some(ms) => ms.to_number(context)? as u64,
    //     None => 0,
    // };

    let ms = match args.first() {
        Some(ms) => ms.to_number(context)? as u64,
        None => 0,
    };

    // Create a request object and insert it into the context
    RuntimeRequest::Sleep(Duration::from_millis(ms)).insert(context.realm());
    tracing::trace!("request inserted into context");

    let (promise, resolver) = JsPromise::new_pending(context);

    let job = NativeJob::new(move |context| {
        tracing::trace!("promise job called");

        // let _ = RuntimeResponse::extract(context.realm()).unwrap();
        // let _ = callable.call(&JsValue::undefined(), &[], context).unwrap();

        resolver
            .resolve
            .call(&JsValue::undefined(), &[JsValue::undefined()], context)
    });

    context.job_queue().enqueue_promise_job(job, context);

    Ok(JsValue::from(promise))
}
