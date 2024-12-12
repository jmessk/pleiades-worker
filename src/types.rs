use std::time::Duration;

use bytes::Bytes;

use crate::runtime::js;

#[derive(Debug)]
pub struct BlobMetadata {
    pub id: String,
}

#[derive(Debug)]
pub struct LambdaMetadata {
    pub id: String,
    pub code: BlobMetadata,
}

#[derive(Debug)]
pub struct JobMetadata {
    pub id: String,
    pub lambda_id: String,
    pub input_id: String,
}

/////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Blob {
    pub id: String,
    pub data: Bytes,
}

#[derive(Debug)]
pub struct Lambda {
    pub id: String,
    pub code: Blob,
}

#[derive(Debug)]
pub struct Job {
    pub status: JobStatus,
    pub time_counter: Duration,
    pub id: String,
    pub lambda: Lambda,
    pub input: Blob,
}

#[derive(Debug)]
pub enum JobStatus {
    Assigned,
    Running,
    Ready {
        context: RuntimeContext,
        response: RuntimeResponse,
    },
    Waiting {
        context: RuntimeContext,
        request: RuntimeRequest,
    },
    Finished(Bytes),
    Cancelled,
}

#[derive(Debug)]
pub enum RuntimeContext {
    JavaScript(js::JsContext),
    Python,
}

/////////////////////////////////////////////////////////

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
        data: Bytes,
    }

    #[derive(Debug, Clone)]
    pub struct Response {
        data: Bytes,
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

// #[derive(Debug, Clone)]
// pub enum BlobCommand {
//     Get(String),
//     Post(Bytes),
// }

// #[derive(Debug, Clone)]
// pub enum HttpCommand {
//     Get(String),
//     Post(Bytes),
// }
