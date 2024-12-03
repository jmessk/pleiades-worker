use bytes::Bytes;

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
    pub id: String,
    pub lambda: Lambda,
    pub input: Blob,
}

#[derive(Debug)]
pub enum JobStatus {
    Assigned,
    Running,
    Waiting,
    Finished(Bytes),
    Cancelled,
}

/////////////////////////////////////////////////////////

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
