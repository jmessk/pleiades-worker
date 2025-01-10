#!/bin/bash

for item in cooperative blocking overloaded; do
    echo "Running $item"

    for i in {1..5}; do
        ./run_generator.bash -n 250
        ./run_worker.bash --config ./config/$item.yml
    done
done

echo "Done"
