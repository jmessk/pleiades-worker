pub mod job_queue;
pub mod module_loader;

use boa_engine::{Context, JsArgs, JsResult, JsValue, NativeFunction, Source};

use crate::types::Job;

// #[derive(Default)]
pub struct Runtime {
    context: Context,
    job: Job,
}

impl Runtime {
    pub fn new(job: Job) -> Self {
        Self {
            context: Self::init(),
            job,
        }
    }

    pub fn init() -> Context {
        let context = Context::default();

        todo!()
    }

    pub fn run(&mut self, job_context: Job) -> JsResult<()> {
        

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::types::LambdaMetadata;

    use super::*;

    #[test]
    fn test_runtime_run() {
        // let mut runtime = Runtime::default();
        // let job = JobMetadata {
        //     id: "1".to_string(),
        //     lambda: LambdaMetadata {
        //         id: "1".to_string(),
        //         code: "console.log('Hello, World!');".to_string(),
        //     },
        //     input_id: "0".to_string(),
        // };

        // runtime.run(job).unwrap();
    }
}
