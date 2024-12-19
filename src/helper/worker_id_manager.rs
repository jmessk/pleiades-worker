use std::{collections::HashMap, sync::Arc};

pub struct WorkerIdManager {
    pub client: Arc<pleiades_api::Client>,
    pub set: HashMap<String, String>,
}

impl WorkerIdManager {
    pub async fn new(client: Arc<pleiades_api::Client>) -> Self {
        let mut manager = Self {
            client,
            set: HashMap::new(),
        };

        manager.insert("default", &["pleiades+example"]).await;
        manager
    }

    pub async fn register_worker(&self, runtimes: &[&str]) -> String {
        let request = pleiades_api::api::worker::register::Request::builder()
            .runtimes(runtimes)
            .build();

        let response = self.client.call_api(&request).await.unwrap();

        response.worker_id
    }

    pub async fn insert<T: Into<String>>(&mut self, name: T, runtimes: &[&str]) {
        let worker_id = self.register_worker(runtimes).await;
        self.set.insert(name.into(), worker_id);
    }

    pub fn get(&self, name: &str) -> Option<String> {
        self.set.get(name).cloned()
    }

    pub fn get_default(&self) -> String {
        self.get("default").unwrap()
    }
}
