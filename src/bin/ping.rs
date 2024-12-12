#[tokio::main]
async fn main() {
    let client = pleiades_api::Client::try_new("http://192.168.1.47/api/v0.5/").unwrap();
    println!("{:?}", client.ping().await.unwrap());
}
