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
    blob, javascript::host_defined::HostDefined as _, RuntimeRequest, RuntimeResponse,
};

#[derive(Debug, Finalize, JsData)]
pub struct ByteData {
    pub inner: Bytes,
}

unsafe impl Trace for ByteData {
    empty_trace!();
}

impl Class for ByteData {
    const NAME: &'static str = "ByteData";
    const LENGTH: usize = 0;

    fn data_constructor(
        _new_target: &JsValue,
        _args: &[JsValue],
        _context: &mut Context,
    ) -> JsResult<Self> {
        println!("new");
        Ok(ByteData {
            inner: bytes::Bytes::new(),
        })
    }

    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        class.static_method(
            js_string!("fromIntU8Array"),
            2,
            NativeFunction::from_fn_ptr(Self::from_int_u8_array),
        );

        Ok(())
    }
}

impl ByteData {
    pub fn from_int_u8_array(
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let data_obj = args.first().unwrap().to_object(context).unwrap();
        let data: Bytes = JsUint8Array::from_object(data_obj)?.iter(context).collect();

        Ok(JsValue::from(JsObject::from_proto_and_data(
            None,
            Self { inner: data },
        )))
    }
}
