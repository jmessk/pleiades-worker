use std::time::Duration;

const ITERATION: usize = 300;
const INTERVAL: Duration = Duration::from_millis(1);

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
    // let script = include_bytes!("./script/counter.js");
    let script = include_bytes!("./script/counter-sleep.js");
    // let script = include_bytes!("./script/sleep.js");
    // let script = include_bytes!("./script/get-blob.js");
    // let script = include_bytes!("./script/get-blob-10.js");
    // let script = include_bytes!("./script/pend-sleep.js");
    // let script = include_bytes!("./script/encoder-decoder.js");

    let lambda_blob = client.blob().new(script.as_ref()).await.unwrap();
    let lambda = lambda_blob.into_lambda("pleiades+example").await.unwrap();

    let input = client
        .blob()
        .new(r#"{"input": "test_input"}"#)
        .await
        .unwrap();

    let mut job_list = tokio::task::JoinSet::new();
    let mut ticker = tokio::time::interval(INTERVAL);

    for i in 0..ITERATION {
        let lambda = lambda.clone();
        let input = input.clone();

        job_list.spawn(async move {
            println!("start job: {}", i);
            let _job = lambda.invoke(input, None).await.unwrap();
            // let _output = job.wait_finished(std::time::Duration::from_secs(10)).await;
        });

        ticker.tick().await;
    }

    let _job_list = job_list.join_all().await;

    // let mut finished_job_list = tokio::task::JoinSet::new();

    // wait the first job
    // let first = job_list.first().unwrap().await;

    // job_list.into_iter().for_each(|job| {
    //     finished_job_list.spawn(async move {
    //         let output = job.wait_finished(std::time::Duration::from_secs(10)).await;

    //         match output {
    //             Ok(finished_job) => Some(finished_job.id.0.into_owned()),
    //             _ => {
    //                 println!("job timeout");
    //                 None
    //             }
    //         }
    //     });
    // });

    // join all and filter None
    // let finished_job_list = finished_job_list.join_all().await;
    // let finished_job_list: Vec<Option<String>> = finished_job_list.join_all().await;
    // let num_timeout = finished_job_list.iter().filter(|job| job.is_none()).count();
    // println!(
    //     "all jobs finished in {:?}, num_timeout: {num_timeout}",
    //     start_outer.elapsed()
    // );

    // println!("start getting metrics");
    // let raw_metrics = get_job_metrics(&client, finished_job_list).await;
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

// fn write_csv(metrics_list: Vec<serde_json::Value>) {
//     let timestump = Local::now().format("%H:%M:%S").to_string();
//     let file_name = format!("requester_{}.csv", timestump);
//     let mut file = std::fs::File::create(file_name).unwrap();

//     file.write_all(b"id,total,sched\n").unwrap();

//     metrics_list.into_iter().for_each(|metrics| {

//     });
// }
