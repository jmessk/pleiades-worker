use std::time::Duration;

use crate::scheduler::local_sched;

struct Item {
    pub local_sched: local_sched::Controller,
    pub deadline: Duration,
}

#[derive(Default)]
pub struct LocalSchedManager {
    list: Vec<Item>,
    deadline_sum: Duration,
}

impl LocalSchedManager {
    pub fn builder() -> Builder {
        Builder::default()
    }

    pub fn num_items(&self) -> usize {
        self.list.len()
    }

    pub fn shortest(&self) -> &local_sched::Controller {
        let item = self
            .list
            .iter()
            .min_by_key(|item| {
                item.local_sched.holding()
                // tracing::debug!("Executor {}: max_queuing_time: {:?}", item.controller.id, a);
                // println!("Executor {}: max_queuing_time: {:?}", item.controller.id, a);
            })
            .unwrap();

        &item.local_sched
    }

    pub fn capacity_sum(&self) -> Duration {
        let used_sum = self
            .list
            .iter()
            .map(|item| item.local_sched.holding())
            .sum();

        self.deadline_sum
            .checked_sub(used_sum)
            .unwrap_or(Duration::ZERO)
    }

    // pub fn queuing_time_sum(&self) -> Duration {
    //     self.list
    //         .iter()
    //         .map(|item| item.local_sched.max_queueing_time())
    //         .sum()
    // }
}

#[derive(Default)]
pub struct Builder {
    list: Vec<Item>,
}

impl Builder {
    pub fn insert(&mut self, local_sched_controller: local_sched::Controller, deadline: Duration) {
        self.list.push(Item {
            local_sched: local_sched_controller,
            deadline,
        });
    }

    pub fn build(self) -> anyhow::Result<LocalSchedManager> {
        if self.list.is_empty() {
            anyhow::bail!("ExecutorManagerBuilder: no executor controller is provided");
        }

        let deadline_sum = self.list.iter().map(|item| item.deadline).sum();

        Ok(LocalSchedManager {
            list: self.list,
            deadline_sum,
        })
    }
}
