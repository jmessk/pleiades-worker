pub mod user;

use boa_engine::{realm::Realm, JsData, NativeObject};
use boa_gc::{empty_trace, Finalize, Trace};

use crate::runtime::{RuntimeRequest, RuntimeResponse};
pub use user::{UserInput, UserOutput};

pub trait HostDefined
where
    Self: Sized + Finalize + JsData + NativeObject,
{
    fn insert_into_context(self, realm: &Realm) -> Option<Box<Self>> {
        realm.host_defined_mut().insert(self)
    }

    // fn get_from_context(realm: &Realm) -> Option<Self>;

    fn exists_in_context(realm: &Realm) -> bool {
        let host_defined = realm.host_defined();
        host_defined.has::<Self>()
    }

    fn get_and_remove_from_context(realm: &Realm) -> Option<Self> {
        let mut host_defined = realm.host_defined_mut();
        host_defined.remove::<Self>().map(|r| *r)
    }
}

impl JsData for RuntimeRequest {}
impl Finalize for RuntimeRequest {}
unsafe impl Trace for RuntimeRequest {
    empty_trace!();
}

impl HostDefined for RuntimeRequest {
    // fn get_from_context(realm: &Realm) -> Option<Self> {
    //     realm.host_defined().get::<Self>().cloned()
    // }
}

impl JsData for RuntimeResponse {}
impl Finalize for RuntimeResponse {}
unsafe impl Trace for RuntimeResponse {
    empty_trace!();
}

impl HostDefined for RuntimeResponse {
    // fn get_from_context(realm: &Realm) -> Option<Self> {
    //     realm.host_defined().get::<Self>().cloned()
    // }
}
