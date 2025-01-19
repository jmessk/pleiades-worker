use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, Mutex, Semaphore};

use crate::{
    data_manager,
    metric::Metric,
    pleiades_type::{Job, JobStatus},
};

/// Contractor
///
///
///
///
pub struct Updater {
    client: Arc<pleiades_api::Client>,

    /// data manager api
    ///
    data_manager_controller: data_manager::Controller,

    /// interface to access this component
    ///
    // command_sender: mpsc::Sender<Command>,
    command_receiver: mpsc::Receiver<Command>,

    enable_metrics: bool,
    metric_sender: mpsc::Sender<Metric>,

    /// number of jobs currently being contracted
    max_concurrency: usize,
    semaphore: Arc<Semaphore>,
}

impl Updater {
    /// new
    ///
    pub fn new(
        client: Arc<pleiades_api::Client>,
        data_manager_controller: data_manager::Controller,
        enable_metrics: bool,
    ) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(128);
        let (metric_sender, metric_receiver) = mpsc::channel(128);

        let updater = Self {
            client,
            data_manager_controller,
            command_receiver,
            enable_metrics,
            metric_sender,
            max_concurrency: 64,
            semaphore: Arc::new(Semaphore::new(64)),
        };

        let controller = Controller {
            command_sender,
            metric_receiver: Arc::new(Mutex::new(metric_receiver)),
        };

        (updater, controller)
    }

    /// run
    ///
    pub async fn run(&mut self) {
        tracing::info!("running");

        while let Some(command) = self.command_receiver.recv().await {
            tracing::trace!("updating job");

            // clone fields to bind
            let client = self.client.clone();
            let data_manager_controller = self.data_manager_controller.clone();
            let metric_sender = self.metric_sender.clone();
            let enable_metrics = self.enable_metrics;
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();

            // new task
            match command {
                Command::Update(request) => {
                    tokio::spawn(async move {
                        let metric =
                            Self::task_update_job_metrics(client, data_manager_controller, request)
                                .await;
                        if enable_metrics {
                            metric_sender.send(metric).await.unwrap();
                        }
                        drop(permit);
                    });
                }
            }
        }

        self.wait_for_shutdown().await;

        // edit
        // let metrics = join_set.join_all().await;
        // match first_instant {
        //     Some(instant) => {
        //         let summary = end - instant;
        //         tracing::info!("summary: {summary:?}, {} jobs", metrics.len());
        //         // tracing::info!(summary, metrics.len());
        //         save_csv(summary, metrics, &self.policy);
        //     }
        //     None => {
        //         tracing::info!("finished 0 job");
        //     }
        // };
    }

    async fn wait_for_shutdown(&self) {
        tracing::debug!(
            "shutting down {} tasks",
            self.max_concurrency - self.semaphore.available_permits()
        );

        let _ = self
            .semaphore
            .acquire_many(self.max_concurrency as u32)
            .await;

        tracing::info!("shutdown");
    }

    /// task
    ///
    async fn _task_update_job(
        client: Arc<pleiades_api::Client>,
        data_manager_controller: data_manager::Controller,
        request: update::Request,
    ) {
        match request.job.status {
            JobStatus::Finished(output) => {
                let output = output.unwrap_or(bytes::Bytes::new());

                let post_handle = data_manager_controller.post_blob(output).await;
                let post_response = post_handle.recv().await;

                let update_request = pleiades_api::api::job::update::Request::builder()
                    .job_id(request.job.id)
                    .data_id(post_response.blob.id)
                    .status("finished")
                    .build();

                let _update_response = client
                    .call_api(&update_request)
                    .await
                    .expect("no error handling: update");

                tracing::debug!("updated finished job");
            }
            JobStatus::Cancelled => {
                let update_request = pleiades_api::api::job::update::Request::builder()
                    .job_id(request.job.id)
                    .data_id("0")
                    .status("cancelled")
                    .build();

                let _update_response = client
                    .call_api(&update_request)
                    .await
                    .expect("no error handling: update");

                tracing::debug!("updated cancelled job");
            }
            _ => {}
        };
    }

    async fn task_update_job_metrics(
        client: Arc<pleiades_api::Client>,
        data_manager_controller: data_manager::Controller,
        request: update::Request,
    ) -> Metric {
        //
        let splited = request.job.lambda.runtime.split('_').collect::<Vec<&str>>()[1]
            .split("-")
            .collect::<Vec<&str>>();
        let sleep_time = splited[1].parse::<u64>().unwrap();
        // println!("sleep_time: {}", sleep_time);
        tokio::time::sleep(Duration::from_secs(sleep_time)).await;
        //

        match request.job.status {
            JobStatus::Finished(output) => {
                let output = output.unwrap_or_else(bytes::Bytes::new);

                let post_handle = data_manager_controller.post_blob(output).await;
                let post_response = post_handle.recv().await;

                let update_request = pleiades_api::api::job::update::Request::builder()
                    .job_id(&request.job.id)
                    .data_id(post_response.blob.id)
                    .status("finished")
                    .build();

                let _update_response = client
                    .call_api(&update_request)
                    .await
                    .expect("no error handling: update");

                tracing::debug!("updated finished job");

                Metric {
                    id: request.job.id,
                    runtime: request.job.lambda.runtime,
                    status: "Finished".to_string(),
                    start: request.job.contracted_at,
                    end: Instant::now(),
                    elapsed: request.job.contracted_at.elapsed(),
                    consumed: request.job.consumed,
                }
            }
            JobStatus::Cancelled => {
                let update_request = pleiades_api::api::job::update::Request::builder()
                    .job_id(&request.job.id)
                    .data_id("0")
                    .status("cancelled")
                    .build();

                let _update_response = client
                    .call_api(&update_request)
                    .await
                    .expect("no error handling: update");

                tracing::debug!("updated cancelled job");

                // (
                //     request.job.id,
                //     request.job.lambda.runtime,
                //     "Cancelled",
                //     request.job.contracted_at.elapsed(),
                //     request.job.consumed,
                // )
                Metric {
                    id: request.job.id,
                    runtime: request.job.lambda.runtime,
                    status: "Cancelled".to_string(),
                    start: request.job.contracted_at,
                    end: Instant::now(),
                    elapsed: request.job.contracted_at.elapsed(),
                    consumed: request.job.consumed,
                }
            }
            _ => unreachable!(),
        }
    }
}

fn _save_csv(
    elapsed: Duration,
    metrics: Vec<(String, String, &'static str, Duration, Duration)>,
    policy: &str,
) {
    use chrono;
    use std::fs::File;
    use std::io::prelude::*;

    std::fs::create_dir_all("./metrics").unwrap();

    let file_name = format!(
        "metrics/{}.csv",
        chrono::Local::now().format("%Y-%m%d-%H%M%S")
    );
    let mut file = File::create(&file_name).unwrap();

    file.write_all(format!("summary: {}\n", elapsed.as_millis()).as_bytes())
        .unwrap();
    file.write_all(format!("policy: {policy}\n").as_bytes())
        .unwrap();
    file.write_all(b"id,runtime,status,elapsed,consumed\n")
        .unwrap();

    metrics
        .iter()
        .for_each(|(job_id, runtime, status, elapsed, comsumed)| {
            let line = format!(
                "{},{},{},{},{}\n",
                job_id,
                runtime,
                status,
                elapsed.as_millis(),
                comsumed.as_millis()
            );
            file.write_all(line.as_bytes()).unwrap();
        });

    tracing::info!("saved metrics to ./{}", file_name);
}

/// Api
///
///
///
///
#[derive(Clone)]
pub struct Controller {
    command_sender: mpsc::Sender<Command>,
    metric_receiver: Arc<Mutex<mpsc::Receiver<Metric>>>,
}

impl Controller {
    /// get_blob
    ///
    pub async fn update_job(&self, job: Job) {
        let request = Command::Update(update::Request { job });

        if self.command_sender.capacity() == 0 {
            tracing::warn!("command queue is full");
        }

        self.command_sender.send(request).await.unwrap();
    }

    pub fn update_job_nowait(&self, job: Job) {
        let request = Command::Update(update::Request { job });

        if self.command_sender.capacity() == 0 {
            tracing::warn!("command queue is full");
        }

        self.command_sender.blocking_send(request).unwrap();
    }
    pub async fn recv_metric(&mut self) -> Option<Metric> {
        self.metric_receiver.lock().await.recv().await
    }
}

pub enum Command {
    Update(update::Request),
    // PostBlob(cannel::Request),
}

pub mod update {
    use super::*;

    pub struct Request {
        pub job: Job,
    }
}

// pub mod cannel {
//     use super::*;

//     pub struct Request {
//         pub response_sender: oneshot::Sender<Response>,
//         pub data: Bytes,
//     }

//     #[derive(Debug)]
//     pub struct Response {
//         pub blob_id: String,
//     }

//     pub struct Handler {
//         pub response_receiver: oneshot::Receiver<Response>,
//     }

//     impl Handler {
//         pub async fn recv_nowait(&mut self) -> Option<Response> {
//             self.response_receiver.try_recv().ok()
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{contractor::Contractor, data_manager::DataManager, fetcher::Fetcher};

    async fn register_worker(client: &Arc<pleiades_api::Client>) -> String {
        let request = pleiades_api::api::worker::register::Request::builder()
            .runtimes(&["pleiades+test"])
            .build();

        let response = client.call_api(&request).await;

        response
            .expect("no error handling: register worker")
            .worker_id
    }

    async fn generate_job(client: &Arc<pleiades_api::Client>) -> bytes::Bytes {
        use pleiades_api::api;

        let code_blob = {
            let request = api::data::upload::Request::builder()
                .data(r#"console.log("hello, world!");"#)
                .build();
            client.call_api(&request).await.unwrap()
        };

        // lambda
        let lambda = {
            let request = api::lambda::create::Request::builder()
                .data_id(code_blob.data_id)
                .runtime("pleiades+test")
                .build();
            client.call_api(&request).await.unwrap()
        };

        // input blob
        let input = {
            let request = api::data::upload::Request::builder()
                .data("test input")
                .build();
            client.call_api(&request).await.unwrap()
        };

        // create job
        let create_job = {
            let request = api::job::create::Request::builder()
                .lambda_id(lambda.lambda_id)
                .data_id(input.data_id)
                .build();
            client.call_api(&request).await.unwrap()
        };

        // wait for finish
        let job_info = {
            let request = api::job::info::Request::builder()
                .job_id(create_job.job_id)
                .except("Finished")
                .timeout(10)
                .build();
            client.call_api(&request).await.unwrap()
        };

        // download output
        let output = {
            let request = api::data::download::Request::builder()
                .data_id(job_info.output.unwrap().data_id)
                .build();
            client.call_api(&request).await.unwrap()
        };

        output.data
    }

    async fn finish_job(client: &Arc<pleiades_api::Client>, job_id: &str) {
        use pleiades_api::api;

        let output = {
            let request = api::data::upload::Request::builder()
                .data("test output")
                .build();
            client.call_api(&request).await.unwrap()
        };

        let _update = {
            let request = api::job::update::Request::builder()
                .job_id(job_id)
                .data_id(output.data_id)
                .status("finished")
                .build();
            client.call_api(&request).await.unwrap()
        };
    }

    #[tokio::test]
    async fn test_contract() {
        let client =
            Arc::new(pleiades_api::Client::try_new("http://192.168.1.47/api/v0.5/").unwrap());

        let (mut fetcher, fetcher_api) = Fetcher::new(client.clone());
        let (mut data_manager, data_manager_controller) = DataManager::new(fetcher_api);
        let (mut contractor, api) = Contractor::new(
            client.clone(),
            data_manager_controller.clone(),
            16,
            Duration::from_millis(100),
        );
        let (mut updater, _updater_api) =
            Updater::new(client.clone(), data_manager_controller, false);

        tokio::spawn(async move {
            fetcher.run().await;
        });
        tokio::spawn(async move { data_manager.run().await });
        tokio::spawn(async move {
            contractor.run().await;
        });
        tokio::spawn(async move {
            updater.run().await;
        });

        let requester_client = client.clone();
        let requester = tokio::spawn(async move { generate_job(&requester_client).await });

        let worker_id = register_worker(&client).await;
        let handle = api.try_contract(worker_id).await.unwrap();

        let response = handle.recv().await;

        println!("{:?}", response);
        assert!(response.contracted.is_some());

        let job_id = response.contracted.unwrap().id;
        finish_job(&client, &job_id).await;

        let output = requester.await.unwrap();

        assert_eq!(output, bytes::Bytes::from("test output"));
    }
}
