#!/bin/bash

for item in cooperative-6 cooperative-10 cooperative-16; do
    echo "Running $item"

    for i in {1..3}; do
        ssh jme@node1.local "cd /home/jme/workspace/mecrm-server-docker && docker compose down && docker compose up -d"
        ./run_generator.bash -n 150
        ./run_worker.bash --config ./config/$item.yml -n 0
    done
done

echo "Done"
