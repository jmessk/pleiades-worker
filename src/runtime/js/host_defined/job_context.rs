use boa_engine::{Context, JsData};
use boa_gc::{empty_trace, Finalize, Trace};

use super::HostDefined;

#[derive(Debug, Finalize, JsData)]
pub struct JobContext {
    pub id: String,
    pub data: bytes::Bytes,
}

unsafe impl Trace for JobContext {
    empty_trace!();
}

impl HostDefined for JobContext {
    fn get_from_context(context: &Context) -> Option<Self> {
        let host_defined = context.realm().host_defined();

        host_defined.get::<Self>().map(|job_context| Self {
            id: job_context.id.clone(),
            data: job_context.data.clone(),
        })
    }
}
