mod class;
mod context;
mod function;
mod host_defined;
mod job_queue;
mod module;

pub use context::JsContext;

use crate::pleiades_type::{Blob, JobStatus, Lambda};
use crate::runtime::{Context, Runtime};

use super::RuntimeResponse;

pub struct JsRuntime {
    context: Option<JsContext>,
}

impl Runtime for JsRuntime {
    type Context = JsContext;

    fn init() -> Self {
        Self { context: None }
    }

    fn create_context(&self, lambda: &Lambda, input: &Blob) -> anyhow::Result<JsContext> {
        let mut context = JsContext::init();

        // if context
        //     .register_user_defined_functions(&lambda.code.data)
        //     .is_err()
        // {
        //     anyhow::bail!("Failed to register user-defined functions");
        // }
        if let Err(e) = context.register_user_defined_functions(&lambda.code.data) {
            anyhow::bail!("Failed to register user-defined functions: {:?}", e);
        }

        context.register_user_input(&input.data);

        tracing::trace!("Context created");

        Ok(context)
    }

    fn set_context(&mut self, context: Self::Context) {
        self.context = Some(context);
    }

    fn get_context(&mut self) -> Option<Self::Context> {
        self.context.take()
    }

    fn set_runtime_response(&mut self, runtime_response: RuntimeResponse) -> anyhow::Result<()> {
        if let Some(context) = &mut self.context {
            context.set_response(runtime_response);
            Ok(())
        } else {
            anyhow::bail!("No context found");
        }
    }

    fn execute(&mut self) -> anyhow::Result<JobStatus> {
        let context = self
            .context
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No context found"))?;

        match context.step() {
            Some(request) => Ok(JobStatus::Pending(request)),
            None => Ok(JobStatus::Finished(context.get_output())),
        }
    }

    // fn executea(&mut self, job: &mut Job) {
    //     let status = job.status;

    //     // set job status to running
    //     job.status = JobStatus::Running;

    //     // get the context from the job status
    //     let mut context = match status {
    //         JobStatus::Assigned => {
    //             let mut context = javascript::JsContext::init();

    //             if context
    //                 .register_user_defined_functions(&job.lambda.code.data)
    //                 .is_err()
    //             {
    //                 println!("Failed to register user-defined functions");
    //                 job.cancel();

    //                 // return job;
    //                 return;
    //             }

    //             context.register_user_input(&job.input.data);

    //             Box::new(context)
    //         }
    //         JobStatus::Ready(response) => match job.context {
    //             Some(RuntimeContext::JavaScript(context)) => {
    //                 context.set_response(response);
    //                 context
    //             }
    //             _ => {
    //                 println!("No context found");
    //                 job.cancel();

    //                 // return job;
    //                 return;
    //             }
    //         },
    //         _ => {
    //             println!("Invalid job status: {:?}", job.status);
    //             job.cancel();

    //             // return job;
    //             return;
    //         }
    //     };

    //     match context.step() {
    //         Some(request) => job.status = JobStatus::Pending(request),
    //         None => {
    //             let output = context.get_output();
    //             job.status = JobStatus::Finished(output)
    //         }
    //     }

    //     job.context = Some(RuntimeContext::JavaScript(context));
    //     // job
    // }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::pleiades_type::{Blob, Job, JobStatus};
    use crate::runtime::{blob, RuntimeRequest};

    use super::*;

    #[test]
    fn test_runtime_simple() {
        let code = r#"
            async function fetch(input) {
                console.log(input);

                return "Hello, World!";
            }

            export default fetch;
        "#;

        let mut job = Job::default();
        job.lambda.code.data = code.into();

        let mut runtime = JsRuntime::init();
        let context = runtime.create_context(&job.lambda, &job.input).unwrap();
        runtime.set_context(context);

        let status = runtime.execute().unwrap();

        assert_eq!(status, JobStatus::Finished(Some("Hello, World!".into())));
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
        job.input.data = "Hello, World!".into();

        let mut runtime = JsRuntime::init();
        let context = runtime.create_context(&job.lambda, &job.input).unwrap();
        runtime.set_context(context);

        let status = runtime.execute().unwrap();

        assert!(matches!(status, JobStatus::Finished(None)));
    }

    #[test]
    fn test_runtime_blob() {
        let code = r#"
            import { blob } from "pleiades"

            async function fetch(input) {
                console.log("fetch called");
                let someData = await blob.get("12345");

                console.log(someData);

                return "Hello, World!"; 
            }

            export default fetch;
        "#;

        let mut job = Job::default();
        job.lambda.code.data = code.into();

        let mut runtime = JsRuntime::init();
        let context = runtime.create_context(&job.lambda, &job.input).unwrap();
        runtime.set_context(context);

        // get_blob called

        let status = runtime.execute().unwrap();

        assert_eq!(
            status,
            JobStatus::Pending(RuntimeRequest::Blob(blob::Request::Get(
                "12345".to_string()
            )))
        );

        // set response

        let blob = Blob {
            data: Bytes::from("Hello, World!"),
            ..Default::default()
        };

        runtime
            .set_runtime_response(RuntimeResponse::Blob(blob::Response::Get(Some(blob))))
            .unwrap();

        let status = runtime.execute().unwrap();

        assert_eq!(status, JobStatus::Finished(Some("Hello, World!".into())));
    }

    #[test]
    fn test_promise() {
        let code: Bytes = r#"
            async function fetch(input) {
                new Promise((resolve) => {
                    console.log("promise called");
                    resolve();
                }).then(() => {
                    console.log("promise resolved");
                });

                return blob;
            }

            export default fetch;
        "#
        .into();

        let mut job = Job::default();
        job.lambda.code.data = code;
        job.input.data = "Hello, World!".into();

        let mut runtime = JsRuntime::init();
        let context = runtime.create_context(&job.lambda, &job.input).unwrap();
        runtime.set_context(context);

        let status = runtime.execute().unwrap();

        if let JobStatus::Pending(RuntimeRequest::Blob(blob::Request::Get(blob_id))) = status {
            assert_eq!(blob_id, "12345");
        }
    }
}
