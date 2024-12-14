mod class;
mod function;
mod host_defined;
mod job_queue;
mod module;
mod context;

pub use context::JsContext;

use crate::pleiades_type::{Job, JobStatus};
use crate::runtime::{js, Context, Runtime, RuntimeContext};

pub struct JsRuntime {}

impl Runtime for JsRuntime {
    fn init() -> Self {
        Self {}
    }

    fn process(&mut self, mut job: Job) -> Job {
        let status = job.status;

        // set job status to running
        job.status = JobStatus::Running;

        // get the context from the job status
        let mut context = match status {
            JobStatus::Assigned => {
                let mut context = js::JsContext::init();

                if context
                    .register_user_defined_functions(&job.lambda.code.data)
                    .is_err()
                {
                    println!("Failed to register user-defined functions");
                    job.cancel();

                    return job;
                }

                context.register_user_input(&job.input.data);

                Box::new(context)
            }
            JobStatus::Ready {
                context: RuntimeContext::JavaScript(mut context),
                response,
            } => {
                context.set_response(response);
                context
            }
            _ => {
                println!("Invalid job status: {:?}", job.status);
                job.cancel();

                return job;
            }
        };

        match context.step() {
            Some(request) => {
                job.status = JobStatus::Pending {
                    context: RuntimeContext::JavaScript(context),
                    request,
                }
            }
            None => {
                let output = context.get_output();
                job.status = JobStatus::Finished(output)
            }
        }

        job
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::{blob, RuntimeRequest};

    use super::*;

    #[test]
    fn test_runtime_simple() {
        let code = r#"
            async function fetch(input) {
                console.log(input);

                return input;
            }

            export default fetch;
        "#;

        let mut job = Job::default();
        job.lambda.code.data = code.into();

        let mut runtime = JsRuntime::init();
        runtime.process(job);
    }

    #[test]
    fn test_runtime_no_output() {
        let code = r#"
            async function fetch(input) {
                console.log("fetch called");
            }

            export default fetch;
        "#;

        let mut job = Job::default();
        job.lambda.code.data = code.into();

        let mut runtime = JsRuntime::init();

        let job = runtime.process(job);
        assert!(matches!(job.status, JobStatus::Finished(None)));
    }

    #[test]
    fn test_runtime_blob() {
        let code = r#"
            import { blob } from "pleiades"

            async function fetch(input) {
                let blob = blob.get("12345");

                return blob;
            }

            export default fetch;
        "#;

        let mut job = Job::default();
        job.lambda.code.data = code.into();

        let mut runtime = JsRuntime::init();

        // start processing the job
        let job = runtime.process(job);

        if let JobStatus::Pending {
            context: _,
            request: RuntimeRequest::Blob(blob::Request::Get(blob_id)),
        } = job.status
        {
            assert_eq!(blob_id, "12345");
        }
    }
}
