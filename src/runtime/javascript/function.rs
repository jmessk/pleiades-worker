use boa_engine::{
    object::builtins::JsUint8Array, value::Type, Context, JsResult, JsValue,
};
use bytes::Bytes;

use crate::runtime::javascript::host_defined::{HostDefined, UserInput, UserOutput};

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
    UserOutput { data }.insert(context.realm());

    Ok(JsValue::undefined())
}

pub fn sleep(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let ms = match args.first() {
        Some(ms) => ms.to_number(context)? as u64,
        None => 0,
    };

    std::thread::sleep(std::time::Duration::from_millis(ms));

    Ok(JsValue::undefined())
}
