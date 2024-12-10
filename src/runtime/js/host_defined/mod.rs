pub mod blob;
mod user;

pub use user::{UserInput, UserOutput};

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
        let host_defined = context.realm().host_defined();
        host_defined.has::<Self>()
    }

    fn remove_from_context(context: &mut Context) -> Option<Box<Self>> {
        let mut host_defined = context.realm().host_defined_mut();
        host_defined.remove::<Self>()
    }
}
