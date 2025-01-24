use clap::Parser;
use pleiades::{Blob, Lambda};
use std::{future::Future, time::Duration};

const INTERVAL: Duration = Duration::from_millis(20);

#[tokio::main]
async fn main() {
    // Load .env file
    //
    dotenvy::dotenv().expect(".env file not found");
    let pleiades_url = std::env::var("PLEIADES_URL").expect("PLEIADES_URL must be set");
    //
    // /////

    // Initialize tracing
    //
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    // tracing_subscriber::registry()
    //     .with(tracing_subscriber::EnvFilter::from_default_env())
    //     .with(tracing_subscriber::fmt::layer())
    //     .with(console_subscriber::spawn())
    //     .try_init()
    //     .unwrap();

    // console_subscriber::init();
    //
    // /////

    // Parse arguments
    //
    let args = Arg::parse();
    //
    // /////

    let client = pleiades::Client::try_new(&pleiades_url).expect("failed to create client");

    // lambda
    //
    let lambda_counter = {
        let script = std::fs::read("./examples/script/counter.js").unwrap();
        let blob = client.blob().new(script).await.unwrap();
        blob.into_lambda("pleiades+example").await.unwrap()
    };
    // let lambda_1 = {
    //     let script = std::fs::read("./examples/script-test2/test1_0-0.js").unwrap();
    //     let blob = client.blob().new(script).await.unwrap();
    //     blob.into_lambda("test1_0-0").await.unwrap()
    // };
    // let lambda_2 = {
    //     let script = std::fs::read("./examples/script-test2/test2_1-0.js").unwrap();
    //     let blob = client.blob().new(script).await.unwrap();
    //     blob.into_lambda("test2_1-0").await.unwrap()
    // };
    // let lambda_3 = {
    //     let script = std::fs::read("./examples/script-test2/test3_1-1.js").unwrap();
    //     let blob = client.blob().new(script).await.unwrap();
    //     blob.into_lambda("test3_1-1").await.unwrap()
    // };
    // let lambda_4 = {
    //     let script = std::fs::read("./examples/script-test2/test4_1-0.js").unwrap();
    //     let blob = client.blob().new(script).await.unwrap();
    //     blob.into_lambda("test4_1-0").await.unwrap()
    // };
    // let lambda_5 = {
    //     let script = std::fs::read("./examples/script-test2/test5_1-1.js").unwrap();
    //     let blob = client.blob().new(script).await.unwrap();
    //     blob.into_lambda("test5_1-1").await.unwrap()
    // };
    // let lambda_6 = {
    //     let script = std::fs::read("./examples/script-test2/test6_0-0.js").unwrap();
    //     let blob = client.blob().new(script).await.unwrap();
    //     blob.into_lambda("test6_0-0").await.unwrap()
    // };
    //
    // /////

    // input
    //
    let blob_blank = client.blob().new("no input").await.unwrap();
    //
    // /////

    let mut join_set = tokio::task::JoinSet::new();
    let mut ticker = tokio::time::interval(INTERVAL);

    for _i in 0..args.num_iteration {
        join_set.spawn(invoke_helper(&lambda_counter, &blob_blank));
        ticker.tick().await;
        // join_set.spawn(invoke_helper(&lambda_1, &blob_blank));
        // ticker.tick().await;

        // join_set.spawn(invoke_helper(&lambda_2, &blob_blank));
        // ticker.tick().await;

        // join_set.spawn(invoke_helper(&lambda_3, &blob_blank));
        // ticker.tick().await;

        // join_set.spawn(invoke_helper(&lambda_4, &blob_blank));
        // ticker.tick().await;

        // join_set.spawn(invoke_helper(&lambda_5, &blob_blank));
        // ticker.tick().await;

        // join_set.spawn(invoke_helper(&lambda_6, &blob_blank));
        // ticker.tick().await;
    }

    join_set.join_all().await;

    println!("All jobs are generated");
}

fn invoke_helper(lambda: &Lambda, input: &Blob) -> impl Future<Output = ()> {
    let lambda = lambda.clone();
    let input = input.clone();

    async move {
        lambda.invoke(input, None).await.unwrap();
    }
}

async fn _get_job_metrics(
    client: &pleiades::Client,
    job_id_list: Vec<Option<String>>,
) -> Vec<serde_json::Value> {
    let mut metrics_list = tokio::task::JoinSet::new();

    job_id_list.into_iter().for_each(|job_id| match job_id {
        Some(job_id) => {
            let reqwest_client = client.inner.client.clone();
            let host = client.inner.base_url.clone();

            metrics_list.spawn(async move {
                let response = reqwest_client
                    .get(host.join(&format!("job/{}/trace", job_id)).unwrap())
                    .send()
                    .await
                    .unwrap();

                response.json::<serde_json::Value>().await.unwrap()
            });
        }
        None => {
            println!("job timeout");
        }
    });

    metrics_list.join_all().await
}

#[derive(Debug, clap::Parser)]
struct Arg {
    #[clap(long = "num", short = 'n', default_value = "10")]
    num_iteration: usize,
}
