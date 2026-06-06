# Kanari Benchmarks Testing

This crate measures Kanari throughput using explicit benchmark modes.

The default mode is `production`, which exercises the full local blockchain production path with
an in-memory engine and a signed zero-gas native workload:

1. Generate and sign deterministic native transactions.
2. Submit every transaction through `BlockchainEngine::submit_transaction`.
3. Produce a DAG-backed block through `BlockchainEngine::produce_block`.
4. Report TPS from the measured `submit_transaction + produce_block` window.

## Quick Checks

Run the crate tests:

```powershell
cargo test -p kanari-benchmarks --quiet
```

Run the production benchmark:

```powershell
cargo run --release -p kanari-benchmarks -- --txs 1024 --json
```

Equivalent explicit command:

```powershell
cargo run --release -p kanari-benchmarks -- --mode production --txs 1024 --json
```

## Modes

- `production`
  Full local production path: mempool submission plus `produce_block`.
- `immediate`
  Executes and applies transactions one by one. Useful as a correctness baseline, not a peak TPS number.
- `parallel`
  Executes transactions in parallel and applies successful changesets. Useful for runtime/state apply experiments.
- `parallel-exec-only`
  Measures execution engine ceiling only. This can produce high numbers, but it is not full blockchain TPS.

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

## Notes

- Use `--release` for throughput numbers.
- Treat `production` as the honest local blockchain TPS benchmark without disk I/O.
- Treat `parallel-exec-only` as an execution ceiling, not as chain TPS.
- Larger `--txs` values can take much longer in production mode because they include signature verification, state-root work, and block production.
- On the current Windows test machine, `parallel-exec-only` exceeds 100k TPS, while `production --txs 10000` is still below 100k because the measured window includes signature verification and DAG production.
