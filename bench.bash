#!/bin/bash

for item in cooperative blocking overloaded; do
    echo "Running $item"

    for i in {1..3}; do
        ./run_generator.bash -n 10
        ./run_worker.bash --config ./config/$item.yml
    done
done

echo "Done"
