pub mod class;
mod context;
pub mod function;
pub mod host_defined;
pub mod job_queue;
pub mod module;
mod context_old;

pub use context::JsContext;

use crate::pleiades_type::{Job, JobStatus};
use crate::runtime::{js, Context, Runtime, RuntimeContext};

pub struct JsRuntime {}

// impl Runtime for JsRuntime {
//     fn init() -> Self {
//         Self {}
//     }

//     fn process(&mut self, mut job: Job) -> Job {
//         let status = job.status;

//         // set job status to running
//         job.status = JobStatus::Running;

//         // get the context from the job status
//         let mut context = match status {
//             JobStatus::Assigned => js::JsContext::init(),
//             JobStatus::Ready {
//                 context: RuntimeContext::JavaScript(mut context),
//                 response,
//             } => {
//                 context.set_response(response);
//                 context
//             }
//             _ => {
//                 println!("Invalid job status: {:?}", job.status);
//                 job.cancel();

//                 return job;
//             }
//         };

//         match context.step() {
//             Some(request) => {
//                 job.status = JobStatus::Waiting {
//                     context: RuntimeContext::JavaScript(context),
//                     request,
//                 }
//             }
//             None => match context.get_output() {
//                 Some(output) => job.status = JobStatus::Finished(output),
//                 None => job.status = JobStatus::Finished("".into()),
//             },
//         }

//         job
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_runtime() {
//         let code = r#"
//             async function fetch(input) {
//                 console.log(input);

//                 return input;
//             }

//             // export default fetch;
//         "#;

//         let mut job = Job::default();
//         job.lambda.code.data = code.into();

//         let mut runtime = JsRuntime::init();

//         runtime.process(job);
//     }
// }
