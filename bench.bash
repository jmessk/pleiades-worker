#!/bin/bash

for item in cooperative blocking overloaded; do
    echo "Running $item"

    for i in {1..1}; do
        ./run_generator.bash -n 200
        ./run_worker.bash --config ./config/$item.yml -n 600
    done
done

echo "Done"
