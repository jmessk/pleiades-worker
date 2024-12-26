# Pleiades Worker

## Config

### Job Generator

- options
  - `--script`: path to the script file
  - `--iteration`: number of iterations. default: 10

```bash
cargo run --release --example job_generator -- --script ./examples/script/hello.js --iteration 100
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
cargo run --release --bin worker -- --config ./config/cooperative.yml
```
