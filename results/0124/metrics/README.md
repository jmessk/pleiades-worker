# 2025-0124-1400

```bash
#!/bin/bash

for item in blocking cooperative-6 overloaded-60 overloaded-100 overloaded-140 ; do
    echo "Running $item"

    for i in {1..3}; do
        ssh jme@node1.local "cd /home/jme/workspace/mecrm-server-docker && docker compose down && docker compose up -d"
        cargo run --release --example job_generator3 -- -n 120
        # ./run_worker.bash --config ./config/$item.yml -n 0
        cargo run --release --bin worker_limited -- --config ./config-limited/$item.yml -n 120
    done
done

echo "Done"
```

- 予備実験として，コンテキストスイッチによるオーバーヘッドを示したかった
- cooperative の場合は，特定のキューが最後に長引くことによって，実行時間が長くなる
