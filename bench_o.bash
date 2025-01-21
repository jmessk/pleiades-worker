#!/bin/bash

for item in overloaded; do
    echo "Running $item"

    for i in {1..1}; do
        ./run_generator_o.bash -n 200
        ./run_worker.bash --config ./config/$item.yml -n 200
    done
done

echo "Done"
