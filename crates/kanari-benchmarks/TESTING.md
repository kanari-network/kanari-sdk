# Kanari Benchmarks Testing

This crate measures Kanari throughput using explicit benchmark modes.

The default mode is `production`, which exercises the full local blockchain production path with
an in-memory engine and a signed zero-gas native workload:

1. Generate and sign deterministic native transactions.
2. Submit every transaction through `BlockchainEngine::submit_transaction`.
3. Produce a Mysticeti checkpoint through `BlockchainEngine::produce_checkpoint`.
4. Report TPS from the measured `submit_transaction + produce_checkpoint` window.

## Quick Checks

Run the crate tests:

```powershell
cargo test -p kanari-benchmarks --quiet
```

Run the production benchmark:

```powershell
cargo run --release -p kanari-benchmarks -- --txs 1024 --json
```

Run the 100k TPS high-throughput target benchmark:

```powershell
cargo run --release -p kanari-benchmarks -- --high-throughput --json
```

Equivalent explicit command:

```powershell
cargo run --release -p kanari-benchmarks -- --mode parallel-exec-only --txs 10000 --senders 10000 --runs 3 --target-tps 100000 --json
```

## Modes

- `production`
  Full local production path: mempool submission plus `produce_checkpoint`.
- `immediate`
  Executes and applies transactions one by one. Useful as a correctness baseline, not a peak TPS number.
- `parallel`
  Executes transactions in parallel and applies successful changesets. Useful for runtime/state apply experiments.
- `parallel-exec-only`
  Measures execution engine ceiling only. This can produce high numbers, but it is not full blockchain TPS.

## High Throughput Target

`--high-throughput` is the quick engine-ceiling target for 100k TPS. It expands to:

- `--mode parallel-exec-only`
- `--txs 10000`
- `--senders 10000`
- `--runs 3`
- `--target-tps 100000`

Every run must meet the target. If a run drops below 100k TPS, the process exits with an error so CI and local scripts catch it immediately.

For production checkpoint TPS, run `production` mode explicitly with your target threshold.

## Interpreting Results

Example output:

```json
{
  "requested_txs": 128,
  "mode": "production",
  "executed": 128,
  "failed": 0,
  "tx_count": 128,
  "submit_secs": 0.0128,
  "produce_secs": 0.0122,
  "duration_secs": 0.123456,
  "tps": 1036.806635
}
```

Important fields:

- `mode`: benchmark path being measured
- `executed`: transactions that completed successfully
- `failed`: transactions that failed in the selected mode
- `tx_count`: transactions included in the measured path
- `duration_secs`: measured benchmark window
- `submit_secs`: time spent in batch mempool submission
- `produce_secs`: time spent in block/DAG production
- `tps`: `tx_count / duration_secs`
- `target_tps`: configured TPS threshold, when set
- `target_status`: `pass` or `fail`, when a threshold is set

## Notes

- Use `--release` for throughput numbers.
- Treat `production` as the honest local blockchain TPS benchmark without disk I/O.
- Treat `parallel-exec-only` as an execution ceiling, not as chain TPS.
- Larger `--txs` values can take much longer in production mode because they include signature verification, state-root work, and block production.
- On the current Windows test machine, `production --txs 10000` can exceed 180k TPS on the local in-memory path after warm-up.

## Soak Test

The long-running production soak test is ignored by default. Run it explicitly:

```powershell
$env:KANARI_SOAK_SECONDS=86400
$env:KANARI_SOAK_TXS=10000
$env:KANARI_SOAK_MIN_TPS=1
cargo test -p kanari-benchmarks production_soak_test -- --ignored --nocapture
```

For a 72-hour run, set `KANARI_SOAK_SECONDS=259200`.
