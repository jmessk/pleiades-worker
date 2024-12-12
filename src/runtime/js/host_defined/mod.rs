pub mod user;

use boa_engine::{Context, JsData, NativeObject};
use boa_gc::{empty_trace, Finalize, Trace};

use crate::runtime::{RuntimeRequest, RuntimeResponse};
pub use user::{UserInput, UserOutput};

pub trait HostDefined
where
    Self: Sized + Finalize + JsData + NativeObject,
{
    fn insert_into_context(self, context: &mut Context) -> Option<Box<Self>> {
        context.realm().host_defined_mut().insert(self)
    }

    fn get_from_context(context: &Context) -> Option<Self>;

    fn exists_in_context(context: &Context) -> bool {
        let host_defined = context.realm().host_defined();
        host_defined.has::<Self>()
    }

    fn remove_from_context(context: &mut Context) -> Option<Box<Self>> {
        let mut host_defined = context.realm().host_defined_mut();
        host_defined.remove::<Self>()
    }
}

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
