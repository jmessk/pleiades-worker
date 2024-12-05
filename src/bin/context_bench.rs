use pleiades_worker::runtime::js::context::JsContext;

fn main() {

    for _ in 0..100 {
        let start = std::time::Instant::now();
        let _context = JsContext::default();

        println!("time: {:?}", start.elapsed());
    }
}
