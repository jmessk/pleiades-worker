use clap::Parser;
use std::time::Duration;

const INTERVAL: Duration = Duration::from_millis(20);

#[tokio::main]
async fn main() {
    // Load .env file
    //
    dotenvy::dotenv().expect(".env file not found");
    let pleiades_url = std::env::var("PLEIADES_URL").expect("PLEIADES_URL must be set");
    //
    // /////

    // parse command line arguments
    //
    let args = Arg::parse();
    let script = std::fs::read(args.script_path).unwrap();
    //
    // /////

    let client = pleiades::Client::try_new(&pleiades_url).expect("failed to create client");

    let lambda_blob = client.blob().new(script).await.unwrap();
    let lambda = lambda_blob.into_lambda("pleiades+example").await.unwrap();

    let input = {
        let input = args
            .input_path
            .map(|input_path| std::fs::read(input_path).unwrap())
            .unwrap_or_else(|| b"example input".to_vec());
        client.blob().new(input).await.unwrap()
    };

    let mut job_list = tokio::task::JoinSet::new();
    let mut ticker = tokio::time::interval(INTERVAL);

    for _i in 0..args.num_iteration {
        let lambda = lambda.clone();
        let input = input.clone();

        job_list.spawn(async move {
            let _job = lambda.invoke(input, None).await.unwrap();
        });

        ticker.tick().await;
    }

    let _job_list = job_list.join_all().await;

    println!("All jobs are generated");
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
    #[clap(long = "script", short = 's')]
    script_path: String,

    #[clap(long = "input", short = 'i')]
    input_path: Option<String>,

    #[clap(long = "num", short = 'n', default_value = "10")]
    num_iteration: usize,
}
