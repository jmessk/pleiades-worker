use crate::runtime::{js, Context, Runtime};
use crate::types::{Job, JobStatus, RuntimeContext, RuntimeRequest};

use super::host_defined::HostDefined as _;

pub struct JsRuntime {}

impl Runtime for JsRuntime {
    fn init() -> Self {
        Self {}
    }

    fn process(&mut self, mut job: Job) -> Job {
        let status = job.status;

        // set job status to running
        job.status = JobStatus::Running;

        let mut context = match status {
            JobStatus::Assigned => js::JsContext::init(&job),
            JobStatus::Ready {
                context: RuntimeContext::JavaScript(mut context),
                response,
            } => {
                context.set_response(response);
                context
            }
            _ => {
                println!("Invalid job status: {:?}", job.status);
                job.status = JobStatus::Cancelled;

                return job;
            }
        };

        // check if there is a request in the context
        if let Some(request) = RuntimeRequest::get_from_context(&context.context) {
            job.status = JobStatus::Waiting {
                context: RuntimeContext::JavaScript(context),
                request,
            };

            return job;
        }

        // check if the job is finished
        match context.get_output() {
            Some(output) => {
                job.status = JobStatus::Finished(output);
            }
            None => {
                job.status = JobStatus::Cancelled;
            }
        }

        job
    }
}
