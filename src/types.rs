use bytes::Bytes;

#[derive(Debug)]
pub struct Blob {
    pub id: String,
    pub data: Bytes,
}

// pub struct LambdaMetadata {
//     pub id: String,
//     pub code: BlobMetadata,
// }

#[derive(Debug)]
pub struct JobMetadata {
    pub id: String,
    pub lambda_id: String,
    pub input_id: String,
}
