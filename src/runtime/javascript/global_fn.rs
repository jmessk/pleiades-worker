use std::{f32::consts::PI, future::Future};

use boa_engine::{
    builtins::array_buffer::ArrayBuffer, js_string, native_function::NativeFunction,
    object::builtins::JsArrayBuffer, value::Type, Context, JsArgs, JsData, JsError, JsNativeError,
    JsObject, JsResult, JsString, JsValue, NativeObject, Source,
};
use bytes::Bytes;

use crate::runtime::javascript::host_defined;

pub fn get_job_context(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let input = context.get_data::<host_defined::InputBlob>().unwrap();
    // let obj = JsObject::
    Ok(js_string!(input.id.clone()).into())
}

pub fn finish_job(_this: &JsValue, args: &[JsValue], context: &mut Context) {
    let data_js_obj = args.first().expect("data is required");

    let data = match data_js_obj.get_type() {
        Type::Undefined | Type::Null => None,
        _ => {
            let data = data_js_obj
                .to_string(context)
                .unwrap()
                .to_std_string_escaped();

            Some(Bytes::from(data))
        }
    };

    context.insert_data(host_defined::Output { data });
}
