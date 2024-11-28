use crate::types::Job;

use boa_engine::{Context, JsArgs, JsResult, JsValue, NativeFunction, Source};
use std::sync::mpsc;

#[derive(Default)]
pub struct JsRuntime {
    context: Context,
}

impl JsRuntime {
    pub fn new() -> Self {
        Self {
            context: Context::default(),
        }
    }

    pub fn run(&mut self, job_context: Job) -> JsResult<()> {
        let result = self
            .context
            .eval(Source::from_bytes(&job_context.lambda.code))?;
        println!("{}", result.to_string(&mut self.context)?.to_std_string_escaped());

        Ok(())
    }
}







pub struct RuntimeInterface {
    runtime: JsRuntime,
    job_sender: mpsc::Sender<Job>,
    job_receiver: mpsc::Receiver<Job>,
}

impl Default for RuntimeInterface {
    fn default() -> Self {
        let (job_sender, job_receiver) = mpsc::channel();

        Self {
            runtime: JsRuntime::default(),
            job_sender,
            job_receiver,
        }
    }
}

impl RuntimeInterface {
    pub fn sender(&self) -> mpsc::Sender<Job> {
        self.job_sender.clone()
    }

    pub fn run(&mut self) {
        loop {
            let job = self.job_receiver.recv().unwrap();
            self.runtime.run(job).unwrap();
        }
    }
}









#[cfg(test)]
mod tests {
    use crate::types::Lambda;

    use super::*;

    #[test]
    fn test_runtime_run() {
        let mut runtime = JsRuntime::default();
        let job = Job {
            id: "1".to_string(),
            lambda: Lambda {
                id: "1".to_string(),
                code: "console.log('Hello, World!');".to_string(),
            },
            input_id: "0".to_string(),
        };

        runtime.run(job).unwrap();
    }
}
