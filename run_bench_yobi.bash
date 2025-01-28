#!/bin/bash

for item in overloaded-60 cooperative-6; do
    echo "Running $item"

    for i in {1..3}; do
        ssh jme@node1.local "cd /home/jme/workspace/mecrm-server-docker && docker compose down && docker compose up -d"
        cargo run --release --example job_generator3 -- -n 240
        # ./run_worker.bash --config ./config/$item.yml -n 0
        cargo run --release --bin worker_limited -- --config ./config-limited/$item.yml -n 240
    done
done

echo "Done"
