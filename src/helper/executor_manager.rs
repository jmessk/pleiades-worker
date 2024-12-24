use std::time::Duration;

use crate::executor;

pub struct ExecutorItem {
    pub controller: executor::Controller,
    pub deadline: Duration,
}

#[derive(Default)]
pub struct ExecutorManager {
    // list: Vec<(executor::Controller, Duration)>,
    list: Vec<ExecutorItem>,
    // pub contracting: Duration,
    // pub pending: Duration,
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

    // pub fn estimated_shortest(&self) -> &executor::Controller {
    //     let item = self
    //         .list
    //         .iter()
    //         .min_by_key(|item| item.controller.max_queueing_time() + item.contracting)
    //         .unwrap();

    //     &item.controller
    // }

    // pub fn update_queuing_time(&mut self) {
    //     self.list
    //         .iter_mut()
    //         .for_each(|item| *item.ad = c.max_queueing_time());
    // }
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

        Ok(ExecutorManager { list: self.list })
    }
}
