#!/bin/bash

# cargo run --release --bin worker_acdsa -- --config ./benchmark/config.blocking-512.yml --script ./benchmark/0-short/0-short.js
cargo run --release --bin worker_acdsa -- --config ./benchmark/config.blocking-2048.yml --script ./benchmark/0-short/0-short.js
# cargo run --release --bin worker_acdsa -- --config ./benchmark/config.cooperative.yml --script ./benchmark/0-short/0-short.js

# cargo run --release --bin worker_acdsa -- --config ./benchmark/config.blocking-512.yml --script ./benchmark/2-mid/2-mid.js
cargo run --release --bin worker_acdsa -- --config ./benchmark/config.blocking-2048.yml --script ./benchmark/2-mid/2-mid.js
# cargo run --release --bin worker_acdsa -- --config ./benchmark/config.cooperative.yml --script ./benchmark/2-mid/2-mid.js
