use boa_engine::{Context, JsData, NativeObject};
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

pub trait HostDefined {
    fn insert_to_context(self, context: &mut Context)
    where
        Self: Sized + Finalize + JsData + NativeObject,
    {
        context.realm().host_defined_mut().insert(self);
    }

    // fn get_from_context(context: &Context) -> Option<&Self> {
    //     let host_defined = context.realm().host_defined();
    //     host_defined.get()
    // }
}
