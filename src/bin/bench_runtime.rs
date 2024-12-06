use pleiades_worker::runtime::js::runtime::Runtime;

use pleiades_worker::types;

const CODE: &str = r#"
    import { blob } from "pleiades"

    async function fetch(job) {
        let someData = blob.get(job);
        return "output"; 
    }

    export default fetch;
"#;

fn main() {
    for _ in 0..100 {
        let start = std::time::Instant::now();
        let job = new_job();

        let mut runtime = Runtime::new(job);
        let output = runtime.run();
        println!("{:?}", output);
        println!("time: {:?}", start.elapsed());
    }
}

fn new_job() -> types::Job {
    types::Job {
        id: "11111".to_string(),
        status: types::JobStatus::Assigned,
        lambda: types::Lambda {
            id: "22222".to_string(),
            code: types::Blob {
                id: "33333".to_string(),
                data: bytes::Bytes::from(CODE),
            },
        },
        input: types::Blob {
            id: "44444".to_string(),
            data: bytes::Bytes::from("this_is_input_data"),
        },
    }
}
