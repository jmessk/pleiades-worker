#!/bin/bash

set -euo pipefail

configs=(
	# ./benchmark/config.cooperative.yml
	./benchmark/config.blocking-512.yml
	./benchmark/config.blocking-1024.yml
	./benchmark/config.blocking-2048.yml
	./benchmark/config.blocking-4096.yml
)

scripts=(
	# ./benchmark/scripts/0-short.js
	# ./benchmark/scripts/1-mid-short.js
	# ./benchmark/scripts/2-mid.js
	# ./benchmark/scripts/3-mid-long.js
	# ./benchmark/scripts/4-long.js
	# ./benchmark/scripts/http+short.js
	./benchmark/scripts/http.js
)

for script in "${scripts[@]}"; do
    for config in "${configs[@]}"; do
		cargo run --release --bin worker_acdsa -- \
			--config "$config" \
			--script "$script"
	done
done
