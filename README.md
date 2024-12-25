# Pleiades Worker

## Config

```yml
num_contractors: 8
num_executors: 4
policy: cooperative_pipeline
exec_deadline: 300ms
job_deadline: 100ms
```

```bash
cargo run --release --bin worker -- --config ./config/cooperative.yml
```
