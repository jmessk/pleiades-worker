#!/bin/bash

NUM_JOBS=50
EPOCHS=1

for item in cooperative-6; do
    echo "Running $item"

    for i in $(seq 1 ${EPOCHS}); do
        ssh jme@node1.local "cd /home/jme/workspace/mecrm-server-docker && docker compose down && docker compose up -d"
        
        cargo run --release --example job_generator_co -- -n ${NUM_JOBS}
        cargo run --release --bin worker -- --config ./config/$item.yml -n 0
    done
done



for item in overloaded-100; do
    echo "Running $item"

    for i in $(seq 1 ${EPOCHS}); do
        ssh jme@node1.local "cd /home/jme/workspace/mecrm-server-docker && docker compose down && docker compose up -d"

        cargo run --release --example job_generator_b -- -n ${NUM_JOBS}
        cargo run --release --bin worker -- --config ./config/$item.yml -n 0
    done
done



echo "Done"
