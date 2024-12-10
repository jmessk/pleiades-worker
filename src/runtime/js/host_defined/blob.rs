use boa_engine::JsData;
use boa_gc::{empty_trace, Finalize, Trace};
use bytes::Bytes;

use super::HostDefined;

pub mod get {
    use super::*;

    /// Request
    ///
    ///
    #[derive(Debug, Finalize, JsData)]
    pub struct Request {
        pub blob_id: String,
    }

    unsafe impl Trace for Request {
        empty_trace!();
    }

    impl HostDefined for Request {
        fn get_from_context(context: &boa_engine::Context) -> Option<Self> {
            let host_defined = context.realm().host_defined();

            host_defined.get::<Self>().map(|request| Self {
                blob_id: request.blob_id.clone(),
            })
        }
    }

    /// Response
    ///
    ///
    #[derive(Debug, Finalize, JsData)]
    pub struct Response {
        pub data: Bytes,
    }

    unsafe impl Trace for Response {
        empty_trace!();
    }

    impl HostDefined for Response {
        fn get_from_context(context: &boa_engine::Context) -> Option<Self> {
            let host_defined = context.realm().host_defined();

            host_defined.get::<Self>().map(|response| Self {
                data: response.data.clone(),
            })
        }
    }
}
