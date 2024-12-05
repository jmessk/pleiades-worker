use boa_engine::{Context, JsArgs, JsResult, JsValue, NativeFunction, Source};
use std::sync::mpsc;

// use crate::runtime::javascript::Context;
use crate::types::JobMetadata;

pub struct RuntimeThread {
    runtime: Context,
    job_sender: mpsc::Sender<JobMetadata>,
    job_receiver: mpsc::Receiver<JobMetadata>,
}

// impl Default for RuntimeThread {
//     fn default() -> Self {
//         let (job_sender, job_receiver) = mpsc::channel();

//         Self {
//             runtime: Runtime::default(),
//             job_sender,
//             job_receiver,
//         }
//     }
// }

impl RuntimeThread {
    pub fn sender(&self) -> mpsc::Sender<JobMetadata> {
        self.job_sender.clone()
    }

    // pub fn run(&mut self) {
    //     loop {
    //         let job = self.job_receiver.recv().unwrap();
    //         self.runtime.run(job).unwrap();
    //     }
    // }
}
