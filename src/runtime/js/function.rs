use boa_engine::{js_string, value::Type, Context, JsResult, JsValue};
use bytes::Bytes;

use crate::runtime::js::host_defined::{self, HostDefined};

pub fn get_user_input(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    // let binding = context.realm().host_defined();
    // let input = binding.get::<host_defined::job_context::JobContext>();

    // match input {
    //     Some(input) => Ok(js_string!(input.id.clone()).into()),
    //     None => Ok(JsValue::undefined()),
    // }
    host_defined::UserInput::get_from_context(context)
        .map(|input| Ok(js_string!(input.id).into()))
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
            let data = data_js_obj
                .to_string(context)
                .unwrap()
                .to_std_string_escaped();

            Some(Bytes::from(data))
        }
    };

    // context
    //     .realm()
    //     .host_defined_mut()
    //     .insert(host_defined::user_output::UserOutput { data });
    host_defined::UserOutput { data }.insert_to_context(context);

    Ok(JsValue::undefined())
}
