use boa_engine::JsData;
use boa_gc::{empty_trace, Finalize, Trace};
use bytes::Bytes;

#[derive(Debug, Finalize, JsData)]
pub struct JobContext {
    pub id: String,
    pub data: bytes::Bytes,
}

unsafe impl Trace for JobContext {
    empty_trace!();
}

#[derive(Debug, Finalize, JsData)]
pub struct UserOutput {
    pub data: Option<Bytes>,
}

unsafe impl Trace for UserOutput {
    empty_trace!();
}
