# Pleiades Worker

本リポジトリは、卒業研究で作成したサーバレスシステムの Worker ノードです。

## Dependency

- **client library**: [pleiades-api](https://github.com/jmessk/pleiades-rs)
- **resource manager**: [pleiades-core](https://git.short-circuits.org/pleiades/pleiades-core)

## Architecture

![worker-components](docs/worker-components.svg)

## Config

### Simple Job Generator

Shell script to run a simple job generator that generates mock jobs for benchmarking.

- options
  - `--script`: path to the script file
  - `--input`: path to the input file
  - `--num_iteration`: number of iterations. default: 10

```bash
./auto/run_simple_generator.bash -s ./examples/script/hello.js -n 10
# cargo run --release --example simple_job_generator -- --script ./examples/script/hello.js --num_iteration 100
```

### Worker Config

You can specify the configuration for the worker in a YAML file. The following is an example of a configuration file.

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
