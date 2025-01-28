#!/bin/bash

for item in cooperative-4 overloaded-44 overloaded-66; do
    echo "Running $item"

    for i in {1..3}; do
        ssh jme@node1.local "cd /home/jme/workspace/mecrm-server-docker && docker compose down && docker compose up -d"
        # ./run_generator.bash -n 150
        cargo run --release --example job_generator -- -n 150
        # ./run_worker.bash --config ./config/$item.yml -n 0
        cargo run --release --bin worker -- --config ./config/$item.yml -n 0
    done
done

echo "Done"
