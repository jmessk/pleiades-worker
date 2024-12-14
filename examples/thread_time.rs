fn main() {
    let start = cpu_time::ThreadTime::now();
    std::thread::sleep(std::time::Duration::from_secs(3));
    println!("Time elapsed: {:?}", start.elapsed());
}
