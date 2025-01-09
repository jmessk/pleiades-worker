# Pleiades Worker

## Config

### Simple Job Generator

- options
  - `--script`: path to the script file
  - `--input`: path to the input file
  - `--num_iteration`: number of iterations. default: 10

```bash
./run_simple_generator.bash -s ./examples/script/hello.js -n 10
# cargo run --release --example simple_job_generator -- --script ./examples/script/hello.js --num_iteration 100
```

### Worker

```yml
num_contractors: 8
num_executors: 4
policy: cooperative_pipeline
exec_deadline: 300ms
job_deadline: 100ms
```

- options
  - `--config`: path to the config file

```bash
./run_worker.bash --config ./config/cooperative.yml
# cargo run --release --bin worker -- --config ./config/cooperative.yml
```
