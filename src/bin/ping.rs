#[tokio::main]
async fn main() {
    // Load .env file
    //
    dotenvy::dotenv().expect(".env file not found");
    let pleiades_url = std::env::var("PLEIADES_URL").expect("PLEIADES_URL must be set");
    //
    // /////

    let client = pleiades_api::Client::try_new(&pleiades_url).unwrap();
    println!("{:?}", client.ping().await.unwrap());
}
