use boa_engine::{js_string, value::Type, Context, JsResult, JsValue};
use bytes::Bytes;

use crate::runtime::js::host_defined;

pub fn get_job_context(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let binding = context.realm().host_defined();
    let input = binding.get::<host_defined::JobContext>();

    match input {
        Some(input) => Ok(js_string!(input.id.clone()).into()),
        None => Ok(JsValue::undefined()),
    }
}

pub fn set_output(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
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

    context
        .realm()
        .host_defined_mut()
        .insert(host_defined::UserOutput { data });

    Ok(JsValue::undefined())
}
