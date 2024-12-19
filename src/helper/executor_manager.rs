use std::time::Duration;

use crate::executor;

#[derive(Default)]
pub struct ExecutorManager {
    list: Vec<(executor::Controller, Duration)>,
}

impl ExecutorManager {
    pub fn builder() -> ExecutorManagerBuilder {
        ExecutorManagerBuilder::default()
    }

    pub fn num_executors(&self) -> usize {
        self.list.len()
    }

    pub fn current_shortest(&self) -> &executor::Controller {
        let executor_controller = self
            .list
            .iter()
            .min_by_key(|(c, _)| c.max_queueing_time())
            .unwrap();
        &executor_controller.0
    }

    pub fn estimated_shortest(&self) -> &executor::Controller {
        let executor_controller = self
            .list
            .iter()
            .min_by_key(|(_, duration)| duration)
            .unwrap();
        &executor_controller.0
    }

    pub fn update_queuing_time(&mut self) {
        self.list
            .iter_mut()
            .for_each(|(c, d)| *d = c.max_queueing_time());
    }
}

#[derive(Default)]
pub struct ExecutorManagerBuilder {
    list: Vec<executor::Controller>,
}

impl ExecutorManagerBuilder {
    pub fn insert(&mut self, controller: executor::Controller) {
        self.list.push(controller);
    }

    pub fn build(self) -> anyhow::Result<ExecutorManager> {
        if self.list.is_empty() {
            anyhow::bail!("ExecutorManagerBuilder: no executor controller is provided");
        }

        let list = self
            .list
            .into_iter()
            .map(|c| (c, Duration::from_secs(0)))
            .collect();

        Ok(ExecutorManager { list })
    }
}
