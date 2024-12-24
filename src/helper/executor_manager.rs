use std::time::Duration;

use crate::executor;

pub struct ExecutorItem {
    pub controller: executor::Controller,
    pub deadline: Duration,
}

#[derive(Default)]
pub struct ExecutorManager {
    list: Vec<ExecutorItem>,
    pub deadline_sum: Duration,
}

impl ExecutorManager {
    pub fn builder() -> ExecutorManagerBuilder {
        ExecutorManagerBuilder::default()
    }

    pub fn num_executors(&self) -> usize {
        self.list.len()
    }

    pub fn shortest(&self) -> &executor::Controller {
        let item = self
            .list
            .iter()
            .min_by_key(|item| {
                let a = item.controller.max_queueing_time();
                println!("max_queueing_time: {:?}", a);
                a
            })
            .unwrap();

        &item.controller
    }

    pub fn capacity_sum(&self) -> Duration {
        self.list
            .iter()
            .map(|item| {
                let max_queuing_time = item.controller.max_queueing_time();

                if max_queuing_time < item.deadline {
                    item.deadline - max_queuing_time
                } else {
                    Duration::from_secs(0)
                }
            })
            .sum()
    }

    pub fn queuing_time_sum(&self) -> Duration {
        self.list
            .iter()
            .map(|item| item.controller.max_queueing_time())
            .sum()
    }
}

#[derive(Default)]
pub struct ExecutorManagerBuilder {
    // list: Vec<executor::Controller>,
    list: Vec<ExecutorItem>,
}

impl ExecutorManagerBuilder {
    pub fn insert(&mut self, controller: executor::Controller, deadline: Duration) {
        // self.list.push(controller);
        self.list.push(ExecutorItem {
            controller,
            deadline,
            // contracting: Duration::from_secs(0),
        });
    }

    pub fn build(self) -> anyhow::Result<ExecutorManager> {
        if self.list.is_empty() {
            anyhow::bail!("ExecutorManagerBuilder: no executor controller is provided");
        }

        let deadline_sum = self.list.iter().map(|item| item.deadline).sum();

        Ok(ExecutorManager {
            list: self.list,
            deadline_sum,
        })
    }
}
