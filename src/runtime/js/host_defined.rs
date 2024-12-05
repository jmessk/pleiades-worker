use boa_engine::{
    native_function::NativeFunction, Context, JsArgs, JsData, JsError, JsNativeError, JsString,
    JsValue, Source,
};
use boa_gc::{empty_trace, Finalize, Trace};
use bytes::Bytes;

#[derive(Debug, Finalize, JsData)]
pub struct InputBlob {
    pub id: String,
    // pub data: bytes::Bytes,
}

unsafe impl Trace for InputBlob {
    empty_trace!();
}

#[derive(Debug, Finalize, JsData)]
pub struct Output {
    pub data: Option<Bytes>,
}

unsafe impl Trace for Output {
    empty_trace!();
}
