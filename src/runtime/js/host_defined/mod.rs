pub mod blob;
mod job_context;
mod user_output;

pub use job_context::JobContext;
pub use user_output::UserOutput;

use boa_engine::{Context, JsData, NativeObject};
use boa_gc::Finalize;

pub trait HostDefined
where
    Self: Sized + Finalize + JsData + NativeObject,
{
    fn insert_to_context(self, context: &mut Context) -> Option<Box<Self>> {
        context.realm().host_defined_mut().insert(self)
    }

    fn get_from_context(context: &Context) -> Option<Self>;

    fn exists_in_context(context: &Context) -> bool {
        Self::get_from_context(context).is_some()
    }
}
