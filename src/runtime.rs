use bytes::Bytes;
use std::{fmt::Debug, time::Duration};

use crate::pleiades_type::{Blob, JobStatus, Lambda};

mod javascript;
mod python;

pub use javascript::JsRuntime;

pub trait Runtime {
    type Context: Context;

    fn init() -> Self;
    fn create_context(&self, lambda: &Lambda, input: &Blob) -> anyhow::Result<Self::Context>;
    fn set_context(&mut self, context: Self::Context);
    fn get_context(&mut self) -> Option<Self::Context>;
    fn set_runtime_response(&mut self, runtime_response: RuntimeResponse) -> anyhow::Result<()>;
    fn execute(&mut self) -> anyhow::Result<JobStatus>;
}

pub trait Context {
    fn init() -> Self;
}

#[derive(Debug)]
pub enum RuntimeContext {
    JavaScript(javascript::JsContext),
    Python,
}

/// RuntimeRequest
///
///
///
///
///
#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeRequest {
    Yield,
    Sleep(Duration),
    Gpu(gpu::Request),
    Blob(blob::Request),
    Http(http::Request),
}

/// RuntimeResponse
///
///
///
///
#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeResponse {
    Yield,
    Sleep,
    Gpu(gpu::Response),
    Blob(blob::Response),
    Http(http::Response),
}

pub mod blob {
    use crate::pleiades_type::Blob;

    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    pub enum Request {
        Get(String),
        Post(Bytes),
    }

    #[derive(Debug, PartialEq, Eq)]
    pub enum Response {
        Get(Option<Blob>),
        Post(Blob),
    }
}

pub mod gpu {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    pub struct Request {
        pub data: Bytes,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct Response {
        pub data: Bytes,
    }
}

pub mod http {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    pub enum Request {
        Get(String),
        Post { url: String, body: Bytes },
    }

    #[derive(Debug, PartialEq, Eq)]
    pub enum Response {
        Get(Option<Bytes>),
        Post(Option<Bytes>),
    }
}
