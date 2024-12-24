use std::time::Duration;

const ITERATION: usize = 10;
const INTERVAL: Duration = Duration::from_millis(20);

#[tokio::main]
async fn main() {
    // Load .env file
    //
    dotenvy::dotenv().expect(".env file not found");
    let pleiades_url = std::env::var("PLEIADES_URL").expect("PLEIADES_URL must be set");
    //
    // /////

    let client = pleiades::Client::try_new(&pleiades_url).expect("failed to create client");

    // let script = include_bytes!("./script/hello.js");
    let script = include_bytes!("./script/counter.js");
    // let script = include_bytes!("./script/sleep.js");
    // let script = include_bytes!("./script/get-blob.js");
    // let script = include_bytes!("./script/pend-sleep.js");

    let lambda_blob = client.blob().new(script.as_ref()).await.unwrap();

    let lambda = lambda_blob.into_lambda("pleiades+example").await.unwrap();

    let input = client
        .blob()
        .new(r#"{"input": "test_input"}"#)
        .await
        .unwrap();

    let mut join_set = tokio::task::JoinSet::new();
    let mut ticker = tokio::time::interval(INTERVAL);

    let start_outer = std::time::Instant::now();

    for i in 0..ITERATION {
        let lambda = lambda.clone();
        let input = input.clone();

        join_set.spawn(async move {
            println!("start job: {}", i);
            let start = std::time::Instant::now();
            let job = lambda.invoke(input, None).await.unwrap();

            let output = job.wait_finished(std::time::Duration::from_secs(20)).await;

            match output {
                Ok(finished_job) => {
                    // let output = finished_job.output.fetch().await.unwrap();
                    // println!("job {} finished in {:?}: {:?}", i, start.elapsed(), output);
                    println!("job {} finished in {:?}", i, start.elapsed());
                    Some(finished_job.id.0.into_owned())
                }
                _ => {
                    println!("job timeout");
                    None
                }
            }
        });

        ticker.tick().await;
    }

    let job_id_list = join_set.join_all().await;
    println!("all jobs finished in {:?}", start_outer.elapsed());

    // println!("start getting metrics");
    // get_job_metrics(&client, job_id_list).await;
}

async fn get_job_metrics(client: &pleiades::Client, job_id_list: Vec<Option<String>>) {
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

                println!("response: {:?}", response);

                response.json::<serde_json::Value>().await.unwrap()
            });
        }
        None => {
            println!("job timeout");
        }
    });

    let metrics_list = metrics_list.join_all().await;
    println!("metrics_list: {:?}", metrics_list);
}
