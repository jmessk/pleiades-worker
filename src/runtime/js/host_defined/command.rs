use boa_engine::JsData;
use boa_gc::{empty_trace, Finalize, Trace};
use bytes::Bytes;

use super::HostDefined;

pub mod request {
    use super::*;

    #[derive(Debug, Clone, JsData, Finalize)]
    pub enum Message {
        Output(Bytes),
        Blob(Blob),
    }

    impl HostDefined for Message {
        fn get_from_context(context: &boa_engine::Context) -> Option<Self> {
            context.realm().host_defined().get::<Self>().cloned()
        }
    }

    unsafe impl Trace for Message {
        empty_trace!();
    }

    #[derive(Debug, Clone)]
    pub enum Blob {
        Get(String),
        Post(Bytes),
    }
}

pub mod response {
    use super::*;

    #[derive(Debug, Clone, JsData, Finalize)]
    pub enum Message {
        Blob(Blob),
    }

    impl HostDefined for Message {
        fn get_from_context(context: &boa_engine::Context) -> Option<Self> {
            context.realm().host_defined().get::<Self>().cloned()
        }
    }

    unsafe impl Trace for Message {
        empty_trace!();
    }

    #[derive(Debug, Clone)]
    pub enum Blob {
        Get(Bytes),
        Post(String),
    }
}

#[derive(Debug, Finalize, JsData)]
pub enum WorkerMassage {
    Blob(),
}

pub use crate::types::{RuntimeRequest, RuntimeResponse};

impl JsData for RuntimeRequest {}
impl Finalize for RuntimeRequest {}
unsafe impl Trace for RuntimeRequest {
    empty_trace!();
}

impl HostDefined for RuntimeRequest {
    fn get_from_context(context: &boa_engine::Context) -> Option<Self> {
        context.realm().host_defined().get::<Self>().cloned()
    }
}

impl JsData for RuntimeResponse {}
impl Finalize for RuntimeResponse {}
unsafe impl Trace for RuntimeResponse {
    empty_trace!();
}

impl HostDefined for RuntimeResponse {
    fn get_from_context(context: &boa_engine::Context) -> Option<Self> {
        context.realm().host_defined().get::<Self>().cloned()
    }
}
