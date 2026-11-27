# Kanari Benchmarks Testing

This crate measures Kanari throughput using explicit benchmark modes.

The default mode is `production`, which exercises the full local blockchain production path with
an in-memory engine and a signed unfunded native workload at the minimum gas price:

1. Generate and sign deterministic native transactions.
2. Submit every transaction through the batch mempool API.
3. Produce a Mysticeti/DAG checkpoint through `BlockchainEngine::produce_checkpoint`.
4. Report production TPS from the measured `produce_checkpoint` window and show submit time separately.

## Quick Checks

Run the crate tests:

```powershell
cargo test -p kanari-benchmarks --quiet
```

Run the production benchmark:

```powershell
cargo run --release -p kanari-benchmarks -- --txs 1024 --json
```

Run the 60k TPS production target benchmark:

```powershell
cargo run --release -p kanari-benchmarks -- --high-throughput --json
```

Equivalent explicit command:

```powershell
cargo run --release -p kanari-benchmarks -- --mode production --txs 10000 --senders 10000 --runs 3 --target-tps 60000 --json
```

## Modes

- `production`
  Full local production path: mempool submission plus repeated `produce_checkpoint` calls until
  the submitted batch is drained. Transactions sharing a primary access lane are intentionally
  placed in later checkpoints, so this measures end-to-end local batch throughput rather than
  silently measuring only the first conflict-free wave.
- `immediate`
  Executes and applies transactions one by one. The built-in workload is deliberately a
  zero-effect native workload optimized for checkpoint production, so it has no spendable coin
  object for direct execution and will be rejected in this mode. Do not use its result as a TPS
  comparison; use a separately funded workload if testing direct execution.

## High Throughput Target

`--high-throughput` is the quick production-path target for 60k TPS. It expands to:

- `--mode production`
- `--txs 10000`
- `--senders 10000`
- `--runs 3`
- `--target-tps 60000`

Every run must meet the target. If a run drops below 60k TPS, the process exits with an error so CI and local scripts catch it immediately.

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
- `duration_secs`: measured benchmark window. In `production` and `owned-fastpath`
  mode this is `submit_secs + produce_secs` so long chunked runs report honest
  wall-clock throughput.
- `submit_secs`: total time spent submitting every chunk into the mempool
- `produce_secs`: time spent in block/DAG/checkpoint production, excluding submit time
- `tps`: `tx_count / duration_secs`
- `target_tps`: configured TPS threshold, when set
- `target_status`: `pass` or `fail`, when a threshold is set

## Notes

- Use `--release` for throughput numbers.
- Treat `production` as the honest local blockchain TPS benchmark without disk I/O.
- Production may use the engine's zero-effect native fast path; its `executed`/`failed` counts
  still come from checkpoint execution and state validation.
- Larger `--txs` values can take much longer in production mode because they include signature verification, state-root work, and block production.
- Clean in-memory numbers on the current Windows test machine (no concurrent tests or builds):
  `admission --txs 100000 --senders 10000 --runs 3` measured **115,409.03 TPS median**
  (109,890.57–116,588.87); `production --txs 10000 --senders 256 --runs 3` measured
  **29,701.43 TPS median** (28,924.84–32,481.93); `--senders 10000` measured
  **22,056.06 TPS median** (21,831.15–23,332.32). Every run had `executed=10000`,
  `failed=0`, `tx_count=10000`. Older 13,836.711 / 26.8k figures came from runs with
  background work and should not be reused.
- `--high-throughput` is admission-only and must not be reported as committed TPS.
- Canonical roots are recomputed from canonical state entries on both backends. The DB-backed SMT is not on the hot commit path; it remains available only for explicit diagnostics/maintenance tooling.
- Persistent RocksDB numbers are measured separately with the script below; they are not
  comparable to the in-memory numbers above.

## Soak Test

The long-running production soak test is ignored by default. Run it explicitly:

```powershell
$env:KANARI_SOAK_SECONDS=86400
$env:KANARI_SOAK_TXS=10000
$env:KANARI_SOAK_MIN_TPS=1
cargo test -p kanari-benchmarks production_soak_test -- --ignored --nocapture
```

For a 72-hour run, set `KANARI_SOAK_SECONDS=259200`.

## Persistent RocksDB Profile

Use the dedicated runner before making storage performance claims. It executes
the full production workload against a temporary RocksDB store and records
memtable, L0, pending-compaction, flush, compaction, and write-stall metrics:

```powershell
.\scripts\run-persistent-rocksdb-benchmark-profile.ps1 -Transactions 10000 -Senders 256 -Runs 3 -StateCfShards 8
```

The command intentionally does not set a TPS target: disk throughput depends on
the machine and storage configuration. Treat a nonzero `total-stops`,
`total-delays`, `compaction-pending`, or `estimate-pending-compaction-bytes` as
a signal to investigate the RocksDB configuration and workload before release.

### State column-family sharding

`KANARI_STATE_CF_SHARDS` (default `8`, clamped to `1..=64`) selects how many
`state_N` column families hold canonical state keys. All 64 CFs are created up
front. The active count is persisted in `default` under
`runtime:state_cf_shard_count_v1`; once present it wins over a later env change,
so a DB is never re-routed silently. Keys are routed with a stable FNV-1a hash of
the exact key bytes — **key and value bytes are unchanged**. SMT `n:`/`d:` nodes,
SMT `meta:` maintenance keys, and the layout marker stay in `default` so
`get_db()` consumers continue to see them. An existing single-CF database is
migrated atomically on first open; the state-key moves and the marker are in one
`WriteBatch`, so a crash cannot expose a half-migrated state root.

Measured on this machine with `production --txs 10000 --senders 256 --runs 3`
(every run `executed=10000 failed=0 tx_count=10000`):

| `KANARI_STATE_CF_SHARDS` | median TPS |       min |       max |
| ------------------------ | ---------: | --------: | --------: |
| 1                        |  18,293.85 | 17,972.44 | 18,558.58 |
| 8                        |  17,973.26 | 17,773.47 | 18,315.53 |
| 16                       |  18,163.93 | 17,694.78 | 18,913.02 |

The ranges overlap: CF sharding did **not** improve single-node TPS on this
workload. A later repeat on the same binary measured 11,201–12,279 TPS median
for both CF=1 and CF=8, so host-level variance is larger than any CF effect;
treat the absolute values as machine snapshots, not stable capacity figures.
Its value is isolating memtable/compaction pressure per state CF and keeping
aggregate RocksDB property reporting honest; do not claim a throughput win from
these numbers.

### Checkpoint commit optimizations

Two low-risk hot-path changes were made without touching consensus, root
semantics, key/value bytes, batch ordering, or durability:

- `PersistentStore` resolves each RocksDB column-family handle once per batch
  instead of once per key, for both `multi_get_cf` and `WriteBatch` staging.
  Input order is preserved, so repeated keys keep the same last-write-wins
  result.
- `finalize_checkpoint` overlaps checkpoint metadata encoding and durable-marker
  encoding with supply validation and with each other via `rayon::join`. The
  closures only read immutable inputs; error priority and marker append order
  are unchanged, and the raw changes still go into the same atomic commit.

Stability check on the final binary: 12 sequential `--runs 1` invocations of
`production --txs 10000 --senders 256` measured **18,824.05 TPS median**
(min 17,922.63, p95 19,601.12, max 19,601.12, mean 18,795.18, sd 571.25,
CV 3.04%). All 12 runs reported `executed=10000 failed=0 tx_count=10000`.
Earlier 3-run sets measured 11,975.96 / 17,713.65 / 19,316.53 median under
host contention; treat those as host noise, not as a regression. Stability,
not peak TPS, is the intended reading of these numbers. Thread-count A/B on
the same binary did not change that conclusion: `RAYON_NUM_THREADS=32`
measured 18,835.62 median / 20,546.61 max / CV 3.13% over 12 runs, and
`RAYON_NUM_THREADS=64` reached 21,264.87 once but measured 18,780.78 median
with CV 6.73% over 8 runs. The 21k observation is a spike, not a stable
capacity figure, so no code default was changed; a 24-thread launch option is
documented below as a host-level setting only. This is still a single-node
result: do not compare it to Aptos' 160k execution-only or 6M off-chain
figures.

### Host-level stability launch (no code change)

No background processes were killed. The benchmark child process can be
launched with `High` priority, all logical CPUs affine on hosts with fewer
than 63 logical CPUs (default affinity otherwise), and
`RAYON_NUM_THREADS=24` via
`scripts/invoke-persistent-rocksdb-benchmark-stable.ps1`. This only changes
scheduling for the benchmark process; it does not change consensus, root
semantics, key/value bytes, durability, guard values, or committed code.

On this 16-logical-CPU host, paired A/B runs gave:

| launch                                   |    median |     CV | note                                           |
| ---------------------------------------- | --------: | -----: | ---------------------------------------------- |
| default                                  | 19,037.37 |  2.31% | quiet host, 8 runs                             |
| High priority, 24 threads                | 19,007.23 |  3.25% | quiet host, 8 runs, +633 mean vs baseline      |
| default                                  | 17,978.59 | 11.23% | later noisy host, 12 runs (one 11,830 outlier) |
| High priority, 24 threads                | 18,836.32 |  5.21% | same noisy set, max 20,773.60                  |
| High priority, pin CPUs 0-11, 12 threads | 18,520.36 |  4.59% | lowest spread in the noisy set                 |

A clean scripted 3-run check measured **18,829.67 TPS median**
(18,672.12–19,511.86), again with `executed=10000 failed=0 tx_count=10000`.
The highest observed values were 20,773.60 TPS with High/24 and 21,264.87 TPS
with 64 threads, both spikes. Host-level tuning therefore buys roughly
0.5–1k TPS and reduces some jitter, but **does not establish stable 21k
sustained throughput**. Use `-Threads 24` for the best median, or
`-Threads 12 -PinCpuCount 12` when minimum spread matters more than peak.
