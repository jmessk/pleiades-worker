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
    let lambda_test = {
        let script = std::fs::read("./examples/script/test.js").unwrap();
        let blob = client.blob().new(script).await.unwrap();
        blob.into_lambda("pleiades+example").await.unwrap()
    };
    let lambda_compress = {
        let script = std::fs::read("./examples/script/compress.js").unwrap();
        let blob = client.blob().new(script).await.unwrap();
        blob.into_lambda("js+compress").await.unwrap()
    };
    let lambda_resize = {
        let script = std::fs::read("./examples/script/resize.js").unwrap();
        let blob = client.blob().new(script).await.unwrap();
        blob.into_lambda("js+resize").await.unwrap()
    };
    // let lambda_fib = {
    //     let script = std::fs::read("./examples/script/fib.js").unwrap();
    //     let blob = client.blob().new(script).await.unwrap();
    //     blob.into_lambda("js+fib").await.unwrap()
    // };
    let lambda_openpose = {
        let script = std::fs::read("./examples/script/openpose.js").unwrap();
        let blob = client.blob().new(script).await.unwrap();
        blob.into_lambda("js+gpu").await.unwrap()
    };
    // let lambda_counter = {
    //     let script = std::fs::read("./examples/script/counter.js").unwrap();
    //     let blob = client.blob().new(script).await.unwrap();
    //     blob.into_lambda("js+counter").await.unwrap()
    // };
    // let lambda_counter1 = {
    //     let script = std::fs::read("./examples/script/counter-sleep1.js").unwrap();
    //     let blob = client.blob().new(script).await.unwrap();
    //     blob.into_lambda("js+counter").await.unwrap()
    // };
    // let lambda_counter2 = {
    //     let script = std::fs::read("./examples/script/counter-sleep2.js").unwrap();
    //     let blob = client.blob().new(script).await.unwrap();
    //     blob.into_lambda("js+counter").await.unwrap()
    // };
    //
    // /////

    // input
    //
    let blob_blank = client.blob().new("no input").await.unwrap();
    let blob_log_zoom_s = {
        let data = std::fs::read("./assets/zoom-s.json").unwrap();
        let blob = client.blob().new(data).await.unwrap();
        client.blob().new(blob.id.to_string()).await.unwrap()
    };
    // let blob_log_zoom_m = {
    //     let data = std::fs::read("./assets/zoom-m.json").unwrap();
    //     let blob = client.blob().new(data).await.unwrap();
    //     client.blob().new(blob.id.to_string()).await.unwrap()
    // };
    // let blob_log_zoom_l = {
    //     let data = std::fs::read("./assets/zoom-l.json").unwrap();
    //     let blob = client.blob().new(data).await.unwrap();
    //     client.blob().new(blob.id.to_string()).await.unwrap()
    // };

    let blob_images_s = {
        let data = std::fs::read("./assets/images.zip").unwrap();
        let blob = client.blob().new(data).await.unwrap();
        client.blob().new(blob.id.to_string()).await.unwrap()
    };
    let blob_images_mini = {
        let data = std::fs::read("./assets/images_mini.zip").unwrap();
        let blob = client.blob().new(data).await.unwrap();
        client.blob().new(blob.id.to_string()).await.unwrap()
    };
    // let blob_images_s = {
    //     let input = std::fs::read("./assets/images-s.zip").unwrap();
    //     client.blob().new(input).await.unwrap()
    // };
    // let blob_images_m = {
    //     let input = std::fs::read("./assets/images-m.zip").unwrap();
    //     client.blob().new(input).await.unwrap()
    // };
    // let blob_images_l = {
    //     let input = std::fs::read("./assets/images-l.zip").unwrap();
    //     client.blob().new(input).await.unwrap()
    // };
    //
    // /////

    let mut join_set = tokio::task::JoinSet::new();
    let mut ticker = tokio::time::interval(INTERVAL);

    for _i in 0..args.num_iteration {
        join_set.spawn(invoke_helper(&lambda_test, &blob_blank));
        ticker.tick().await;

        // join_set.spawn(invoke_helper(&lambda_compress, &blob_log_zoom_s));
        // ticker.tick().await;

        // join_set.spawn(invoke_helper(&lambda_resize, &blob_images_s));
        // ticker.tick().await;

        // join_set.spawn(invoke_helper(&lambda_fib, &blob_blank));
        // ticker.tick().await;

        // join_set.spawn(invoke_helper(&lambda_openpose, &blob_images_mini));
        // ticker.tick().await;

        // join_set.spawn(invoke_helper(&lambda_counter, &blob_blank));
        // ticker.tick().await;

        // join_set.spawn(invoke_helper(&lambda_counter1, &blob_blank));
        // ticker.tick().await;

        // join_set.spawn(invoke_helper(&lambda_counter2, &blob_blank));
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
