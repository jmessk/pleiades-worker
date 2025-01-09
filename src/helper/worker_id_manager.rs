use std::{collections::HashMap, sync::Arc, time::Duration};

#[derive(Debug, Clone)]
pub struct WorkerIdManager {
    pub client: Arc<pleiades_api::Client>,
    pub set: HashMap<String, (String, Duration)>,
}

impl WorkerIdManager {
    pub async fn new(client: Arc<pleiades_api::Client>, default_job_deadline: Duration) -> Self {
        let mut manager = Self {
            client,
            set: HashMap::new(),
        };

        manager
            .insert(
                "default",
                &["pleiades+example", "js+compress", "js+resize"],
                default_job_deadline,
            )
            .await;
        manager
    }

    pub async fn register_worker(&self, runtimes: &[&str]) -> String {
        let request = pleiades_api::api::worker::register::Request::builder()
            .runtimes(runtimes)
            .build();

        let response = self.client.call_api(&request).await.unwrap();

        response.worker_id
    }

    pub async fn insert<T: Into<String>>(
        &mut self,
        key: T,
        runtimes: &[&str],
        job_deadline: Duration,
    ) {
        let worker_id = self.register_worker(runtimes).await;
        self.set.insert(key.into(), (worker_id, job_deadline));
    }

    pub fn get(&self, key: &str) -> Option<(String, Duration)> {
        self.set.get(key).cloned()
    }

    pub fn get_default(&self) -> (String, Duration) {
        self.get("default").unwrap()
    }
}
