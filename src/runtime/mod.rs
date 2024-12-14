use bytes::Bytes;
use std::fmt::Debug;

use crate::pleiades_type::Job;

mod js;
mod python;

pub use js::JsRuntime;

pub trait Runtime {
    fn init() -> Self;
    fn process(&mut self, job: Job) -> Job;
}

pub trait Context {
    fn init() -> Self;
}

#[derive(Debug)]
pub enum RuntimeContext {
    JavaScript(Box<js::JsContext>),
    Python,
}

/// RuntimeRequest
///
///
///
///
///
#[derive(Debug)]
pub enum RuntimeRequest {
    Gpu(gpu::Request),
    Blob(blob::Request),
    Http(http::Request),
}

/// RuntimeResponse
///
///
///
///
#[derive(Debug)]
pub enum RuntimeResponse {
    Gpu(gpu::Response),
    Blob(blob::Response),
    Http(http::Response),
}

pub mod blob {
    use crate::pleiades_type::Blob;

    use super::*;

    #[derive(Debug)]
    pub enum Request {
        Get(String),
        Post(Bytes),
    }

    #[derive(Debug)]
    pub enum Response {
        Get(Option<Blob>),
        Post(Blob),
    }
}

pub mod gpu {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct Request {
        pub data: Bytes,
    }

    #[derive(Debug, Clone)]
    pub struct Response {
        pub data: Bytes,
    }
}

pub mod http {
    use super::*;

    #[derive(Debug, Clone)]
    pub enum Request {
        Get(String),
        Post { url: String, body: Bytes },
    }

    #[derive(Debug, Clone)]
    pub enum Response {
        Get(Option<Bytes>),
        Post(Option<Bytes>),
    }
}
