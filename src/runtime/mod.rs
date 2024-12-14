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
    JavaScript(js::JsContext),
    Python,
}

#[derive(Debug, Clone)]
pub enum RuntimeRequest {
    Gpu(gpu::Request),
    Blob(blob::Request),
    Http(http::Response),
}

#[derive(Debug, Clone)]
pub enum RuntimeResponse {
    Gpu(gpu::Response),
    Blob(blob::Response),
    Http(http::Response),
}

pub mod blob {
    use super::*;

    #[derive(Debug, Clone)]
    pub enum Request {
        Get(String),
        Post(Bytes),
    }

    #[derive(Debug, Clone)]
    pub enum Response {
        Get(Bytes),
        Post(String),
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
    pub enum Command {
        Get(String),
        Post(Bytes),
    }

    #[derive(Debug, Clone)]
    pub enum Response {
        Get(Bytes),
        Post(String),
    }
}
