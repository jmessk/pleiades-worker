use crate::types::Job;

use boa_engine::{Context, JsArgs, JsResult, JsValue, NativeFunction, Source};
use std::sync::mpsc;

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
        // let result = self
        //     .context
        //     .eval(Source::from_bytes(&job_context.lambda.code))?;
        // println!("{}", result.to_string(&mut self.context)?.to_std_string_escaped());
        // self.context;
    
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
