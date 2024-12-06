use pleiades_worker::runtime::js::runtime::Runtime;

fn main() {

    for _ in 0..100 {
        let start = std::time::Instant::now();
        // let _context = Runtime::default();

        println!("time: {:?}", start.elapsed());
    }
}
