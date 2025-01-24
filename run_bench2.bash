#!/bin/bash

for item in overloaded-66 overloaded-88 overloaded-110 cooperative-6 cooperative-10; do
    echo "Running $item"

    for i in {1..3}; do
        ssh jme@node1.local "cd /home/jme/workspace/mecrm-server-docker && docker compose down && docker compose up -d"
        ./run_generator.bash -n 150
        cargo run --release --bin worker -- --config ./config/$item.yml -n 0
    done
done

echo "Done"
