use pleiades_worker::rm::fetcher::Fetcher;
use bytes::Bytes;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let client = Arc::new(pleiades_api::Client::new("http://192.168.1.47/api/v0.5/").unwrap());
    let mut fetcher = Fetcher::new(client);
    let api = fetcher.api();

    tokio::spawn(async move {
        fetcher.run().await;
    });

    let data = Bytes::from("hello world");

    println!("1");
    let handle = api.upload_blob(data.clone()).await;
    println!("2");
    let response = handle.recv().await;

    println!("3");
    let handle = api.download_blob(response.blob_id).await;
    println!("4");
    let response = handle.recv().await;

    println!("5");
    assert_eq!(response.data, data);
}
