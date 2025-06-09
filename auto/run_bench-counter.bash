#!/bin/bash

EPOCHS=3
DIR=config-20-counter
NUM_JOBS=1500

# for item in cooperative-6 cooperative-6-2 cooperative-8 cooperative-8-2; do
for item in cooperative-6 cooperative-6-2; do
    echo "Running $item"

    for i in $(seq 1 ${EPOCHS}); do
        ssh jme@node1.local "cd /home/jme/workspace/mecrm-server-docker && docker compose down && docker compose up -d"
        
        cargo run --release --example job_generator_counter -- -n ${NUM_JOBS}
        cargo run --release --bin worker -- --config ./${DIR}/${item}.yml -n 0
    done
done

for item in blocking overloaded-60 overloaded-100; do
    echo "Running $item"

    for i in $(seq 1 ${EPOCHS}); do
        ssh jme@node1.local "cd /home/jme/workspace/mecrm-server-docker && docker compose down && docker compose up -d"

        cargo run --release --example job_generator_counter -- -n ${NUM_JOBS}
        cargo run --release --bin worker -- --config ./${DIR}/${item}.yml -n 0
    done
done



echo "Done"
