const ITERATION: usize = 10;

#[tokio::main]
async fn main() {
    // Load .env file
    //
    dotenvy::dotenv().expect(".env file not found");
    let pleiades_url = std::env::var("PLEIADES_URL").expect("PLEIADES_URL must be set");
    //
    // /////

    let client = pleiades::Client::try_new(&pleiades_url).unwrap();

    let lambda_blob = client
        .blob()
        .new(include_bytes!("./script/get-blob.js").as_slice())
        .await
        .unwrap();

    let lambda = lambda_blob.into_lambda("pleiades+example").await.unwrap();

    let input = client
        .blob()
        .new(r#"{"input": "test_input"}"#)
        .await
        .unwrap();

    let mut join_set = tokio::task::JoinSet::new();
    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(100));

    let start_outer = std::time::Instant::now();

    for _ in 0..ITERATION {
        let lambda = lambda.clone();
        let input = input.clone();

        join_set.spawn(async move {
            let start = std::time::Instant::now();
            let job = lambda.invoke(input, None).await.unwrap();

            let output = job.wait_finished(std::time::Duration::from_secs(10)).await;

            match output {
                Ok(_) => println!("job finished in {:?}", start.elapsed()),
                _ => println!("job timeout"),
            }
        });

        ticker.tick().await;
    }

    join_set.join_all().await;

    println!("all jobs finished in {:?}", start_outer.elapsed());
}
