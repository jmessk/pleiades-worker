use tokio::sync::mpsc;

use crate::runtime::{JsRuntime, Runtime};
use crate::pleiades_type::Job;

pub struct Executor {
    pub tx: mpsc::Sender<Job>,
    pub rx: mpsc::Receiver<Job>,
    runtime: JsRuntime,
}

// impl Default for Executor {
//     fn default() -> Self {
//         let (tx, rx) = mpsc::channel(8);

//         Self {
//             tx,
//             rx,
//             runtime: JsRuntime::init(),
//         }
//     }
// }

impl Executor {
    pub fn run(self) {
        todo!()
    }
}
