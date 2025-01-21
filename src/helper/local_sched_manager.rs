use std::time::Duration;

use tokio::task::JoinSet;

use crate::scheduler::local_sched;

struct Item {
    pub local_sched: local_sched::Controller,
    pub deadline: Duration,
}

#[derive(Default)]
pub struct LocalSchedManager {
    list: Vec<Item>,
    pub deadline_sum: Duration,
}

impl LocalSchedManager {
    pub fn builder() -> Builder {
        Builder::default()
    }

    pub fn num_items(&self) -> usize {
        self.list.len()
    }

    pub fn shortest(&mut self) -> &mut local_sched::Controller {
        let item = self
            .list
            .iter_mut()
            .min_by_key(|item| {
                let used_time = item.local_sched.used_time();

                tracing::trace!(
                    "Executor {}: max_queuing_time: {:?}",
                    item.local_sched.id,
                    used_time
                );

                used_time
            })
            .unwrap();

        &mut item.local_sched
    }
 
    pub fn shortest_pending(&mut self) -> &mut local_sched::Controller {
        let item = self
            .list
            .iter_mut()
            // .filter(|item| )
            .min_by_key(|item| item.local_sched.pending())
            .unwrap();

        &mut item.local_sched
    }

    pub fn shortest_queuing(&mut self) -> &mut local_sched::Controller {
        let item = self
            .list
            .iter_mut()
            .min_by_key(|item| item.local_sched.queuing())
            .unwrap();

        &mut item.local_sched
    }

    pub fn capacity_sum(&self) -> Duration {
        let used_sum = self.used_sum();
        self.deadline_sum
            .checked_sub(used_sum)
            .unwrap_or(Duration::ZERO)
    }

    pub fn used_sum(&self) -> Duration {
        self.list
            .iter()
            .map(|item| item.local_sched.used_time())
            .sum()
    }

    pub async fn signal_shutdown_req(&self) {
        for item in &self.list {
            item.local_sched.signal_shutdown_req().await;
        }
    }
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
