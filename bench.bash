#!/bin/bash

for item in blocking-2; do
    echo "Running $item"

    for i in {1..3}; do
        ./run_generator.bash -n 50
        ./run_worker.bash --config ./config/$item.yml -n 300
    done
done

echo "Done"
