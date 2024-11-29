use bytes::Bytes;

pub struct BlobMetadata{
    pub id: String,
    pub data: Bytes,
}

pub struct LambdaMetadata {
    pub id: String,
    pub code: BlobMetadata,
}

pub struct JobMetadata {
    pub id: String,
    pub lambda_id: String,
    pub input_id: String,
}
