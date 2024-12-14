use boa_engine::{realm::Realm, Context, JsData};
use boa_gc::{empty_trace, Finalize, Trace};
use bytes::Bytes;

use super::HostDefined;

#[derive(Debug, Finalize, JsData)]
pub struct UserInput {
    pub data: Bytes,
}

unsafe impl Trace for UserInput {
    empty_trace!();
}

impl HostDefined for UserInput {
    // fn get_from_context(realm: &Realm) -> Option<Self> {
    //     let host_defined = realm.host_defined();

    //     host_defined.get::<Self>().map(|job_context| Self {
    //         data: job_context.data.clone(),
    //     })
    // }
}

#[derive(Debug, Finalize, JsData)]
pub struct UserOutput {
    pub data: Option<Bytes>,
}

unsafe impl Trace for UserOutput {
    empty_trace!();
}

impl HostDefined for UserOutput {
    // fn get_from_context(realm: &Realm) -> Option<Self> {
    //     let host_defined = realm.host_defined();

    //     host_defined.get::<Self>().map(|user_output| Self {
    //         data: user_output.data.clone(),
    //     })
    // }
}
