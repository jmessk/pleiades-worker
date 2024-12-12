use tokio::sync::mpsc;

use crate::runtime::js;
use crate::types::Job;

pub struct Executor {
    pub tx: mpsc::Sender<Job>,
    pub rx: mpsc::Receiver<Job>,
}

impl Default for Executor {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(8);

        Self { tx, rx }
    }
}

impl Executor {
    pub fn run(self) {
        todo!()
    }
}
