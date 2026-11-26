# Kanari Scripts Usage Guide

This guide documents every automation script in `scripts/` and shows how to run
it on Windows with PowerShell. Commands below assume that the current directory
is the repository root (`D:\move-jamae\kanari-sdk`).

PowerShell 5.1 and PowerShell 7 are supported. Paths use Windows separators in
examples; replace the keystore path and passwords with your own development
values. Do not use a production keystore for load, chaos, fuzz, or campaign
runs.

## Contents

- [Prerequisites](#prerequisites)
- [Common conventions](#common-conventions)
- [CI entry point](#ci-entry-point)
- [Quick index](#quick-index)
- [Shared helper library](#shared-helper-library)
- [Local four-node runners](#local-four-node-runners)
- [RPC load and benchmark scripts](#rpc-load-and-benchmark-scripts)
- [Security, fuzz, and campaign scripts](#security-fuzz-and-campaign-scripts)
- [Low-level lane wrapper](#low-level-lane-wrapper)
- [Artifacts and logs](#artifacts-and-logs)
- [Troubleshooting](#troubleshooting)

## Prerequisites

From the repository [README](../README.md):

- Rust stable and Cargo
- Clang and LLVM
- CMake
- A C/C++ toolchain supported by RocksDB
- PowerShell for the included Windows scripts
- Linux may also need `pkg-config` and OpenSSL development packages

Optional tools used by specific scripts:

| Tool           | Used by                                                                           |
| -------------- | --------------------------------------------------------------------------------- |
| `cargo-audit`  | `run-security-audit.ps1`, `run-crypto-fuzz-campaign.ps1`                          |
| `cargo-clippy` | `run-security-audit.ps1`, `run-fuzz-campaign.ps1`, `run-crypto-fuzz-campaign.ps1` |
| Linux `perf`   | `-StartLinuxPerfRecorders` in the parallel/chaos runners                          |

Most runners build their own binaries. Use `-SkipBuild` only after a successful
build, or when you intentionally want to test an existing
`target\release\kanari-node.exe` / `target\release\kanari.exe`.

Build manually with:

```powershell
cargo build -p kanari-node --release
cargo build -p kanari --release
cargo build -p kanari-rpc-loadgen --release
cargo build -p kanari-benchmarks --release
```

## Common conventions

### Environment variables

| Variable                               | Meaning                                    | Set by                                                      |
| -------------------------------------- | ------------------------------------------ | ----------------------------------------------------------- |
| `KANARI_KEYSTORE_PATH`                 | Keystore used by the Kanari CLI            | Most runners; pass `-KeystorePath` or set it before running |
| `KANARI_LOAD_PASSWORD`                 | Password for the development keystore      | Most runners; pass `-Password` or set it before running     |
| `KANARI_SYNC_ADMISSION_LAG`            | Catch-up lag for the rejoin admission gate | `test-rejoin-admission.ps1` only                            |
| `KANARI_P2P_NAMESPACE`                 | Isolates chaos P2P topics                  | Parallel runner / chaos campaigns                           |
| `KANARI_P2P_CHANNEL_CAPACITY`          | P2P channel capacity                       | Parallel runner / chaos campaigns                           |
| `KANARI_MAX_CONCURRENT_SYNC_MESSAGES`  | Sync fan-in bound                          | Parallel runner / rejoin scenario                           |
| `KANARI_CHAOS_P2P_PUBLISH_DELAY_MS`    | Synthetic gossip delay                     | Parallel runner / rejoin scenario                           |
| `KANARI_CHAOS_P2P_DUPLICATE_PUBLISHES` | Synthetic duplicate gossip                 | Parallel runner / chaos campaigns                           |
| `KANARI_CHAOS_P2P_REORDER_BEST_EFFORT` | Best-effort local reorder                  | Parallel runner / chaos campaigns                           |
| `KANARI_CRYPTO_PROPTEST_CASES`         | Proptest case count                        | `run-crypto-fuzz-campaign.ps1`                              |
| `KANARI_BENCH_STATE_BACKEND`           | Benchmark state backend                    | `run-persistent-rocksdb-benchmark-profile.ps1`              |
| `KANARI_BENCH_ROCKSDB_PROFILE`         | Enable RocksDB profile metrics             | `run-persistent-rocksdb-benchmark-profile.ps1`              |
| `KANARI_BENCH_ROCKSDB_FLUSH_PROFILE`   | Enable flush metrics                       | `run-persistent-rocksdb-benchmark-profile.ps1`              |
| `KANARI_BENCH_ROCKSDB_COMPACT_PROFILE` | Enable compaction metrics                  | `run-persistent-rocksdb-benchmark-profile.ps1`              |

Core strictness knobs are inherited by spawned nodes if you set them before
launching a runner:

```powershell
$env:KANARI_OWNED_FASTPATH_STRICT_FINALIZE = '1'
$env:KANARI_POST_APPLY_SUPPLY_AUDIT = '1'
$env:KANARI_OWNED_FASTPATH_AUDIT_INTERVAL = '1'
$env:KANARI_OWNED_FASTPATH_AUDIT_SMT = '1'
$env:KANARI_VALIDATE_OWNED_FASTPATH_SUPPLY = '1'
```

### Default development wallets

Several runners use these development addresses as defaults:

```text
sender A: 0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146
sender B: 0x7e669cf309e9761beb2083d12564895ccea1389f2851a479edcc0d1ad19e4350
```

The default keystore in the local runners is:

```text
C:\Users\TaroPukpui\.kanari\kanari_config\kanari.keystore
```

Always pass `-KeystorePath` explicitly if your keystore is elsewhere.

### Four-node port map

Four nodes use `base`, `base+10`, `base+20`, and `base+30`. P2P and RPC have
separate bases.

| Runner                                         | Base P2P | Base RPC | Node RPC endpoints         |
| ---------------------------------------------- | -------: | -------: | -------------------------- |
| `run-local-four-node-tx-load.ps1`              |    19300 |    19301 | 19301, 19311, 19321, 19331 |
| `run-local-four-node-rpc-load.ps1`             |    19100 |    19101 | 19101, 19111, 19121, 19131 |
| `run-local-four-node-chaos.ps1`                |    19700 |    19701 | 19701, 19711, 19721, 19731 |
| `run-local-four-node-parallel-tx-chaos.ps1`    |    19500 |    19501 | 19501, 19511, 19521, 19531 |
| `test-rejoin-admission.ps1`                    |    19600 |    19601 | 19601, 19611, 19621, 19631 |
| `run-local-four-node-10k-load.ps1` (wrapper)   |    21400 |    21401 | 21401, 21411, 21421, 21431 |
| `run-chaos-network-campaign.ps1` (default)     |    23000 |    23001 | 23001, 23011, 23021, 23031 |
| `run-long-run-security-campaign.ps1` (fixed)   |    24000 |    24001 | 24001, 24011, 24021, 24031 |
| `run-full-tx-benchmark-profile.ps1` (target 1) |    25000 |    25001 | 25001, 25011, 25021, 25031 |

Campaign runners advance the base by 100 per iteration and skip Windows
excluded/listening port ranges.

### Safety behavior

- Each four-node runner calls `Stop-KanariStaleFourNodeProcesses` before
  starting nodes. It only stops `kanari-node.exe` processes whose command line
  uses the runner's P2P/RPC ports, then fails fast if a listener remains.
- Genesis and keys are created under a fresh `.kanari-runlogs\<prefix>-<timestamp>`
  directory. The genesis DB is separate from `node1-db`.
- Trusted peer-to-authority mappings are generated from each node's real
  libp2p peer ID and passed with `--peer-authority`. This is a policy-only
  input for sync admission; it never changes the committee or quorum.
- `finally` blocks stop spawned node, lane, and profiler processes.
- Native stdout and stderr are written to separate `*.out.log` and `*.err.log`
  files by the shared process helper. Four-node runners therefore do not wrap
  native progress as PowerShell `NativeCommandError`.
- Development logs may contain expected WARN lines about fresh genesis
  metadata, embedded Move bytecode, stale SMT rebuild, single-authority mode,
  dev plaintext keys, or `NoPeersSubscribedToTopic`. Do not suppress real
  errors.

### Amount format and coin fanout

Amounts passed to the CLI are rendered as invariant decimal strings by
`Format-KanariAmount` (no scientific notation, no locale comma).

For native transfer lanes, reserve:

```text
FaucetCoinsPerSender >= (2 * CountPerSender) + CoinReserveBuffer
```

One coin is the mutable transfer object, one is an independent gas object, and
`CoinReserveBuffer` is retained for faucet/fanout bookkeeping. `-AutoCoinFanout`
raises `-FaucetCoinsPerSender` to this minimum instead of failing. When fanout
preview exceeds the Move gas budget, the parallel runner adaptively reduces
the batch size.

## CI entry point

GitHub Actions should call the reusable workflow
[`.github/workflows/kanari-script-runner.yml`](../.github/workflows/kanari-script-runner.yml)
instead of invoking these scripts directly. It centralizes checkout, Rust
toolchain setup, optional `cargo-audit` installation, release builds, argument
parsing, execution, and artifact upload.

Key `with` inputs:

| Input                             | Purpose                                                  |
| --------------------------------- | -------------------------------------------------------- |
| `script`                          | Repository-relative `.ps1` path; omit for tests only     |
| `args_json`                       | JSON array of CLI arguments; default `[]`                |
| `build_packages_json`             | JSON array of Cargo packages to build with `--release`   |
| `env_json`                        | JSON object of extra environment variables               |
| `install_cargo_audit`             | Install `cargo-audit` before the run                     |
| `run_workspace_tests`             | Run `cargo test --workspace` before the script           |
| `workspace_test_args_json`        | JSON array of extra `cargo test` arguments               |
| `cache_prefix`                    | Cargo cache key prefix                                   |
| `runs_on`                         | Runner label; use `windows-latest` for four-node runners |
| `timeout_minutes`                 | Job timeout; default `360`                               |
| `artifact_name` / `artifact_path` | Optional upload of script artifacts                      |

Example:

```yaml
jobs:
  chaos:
    uses: ./.github/workflows/kanari-script-runner.yml
    with:
      script: scripts/run-local-four-node-chaos.ps1
      args_json: '["-Requests","500","-Concurrency","32","-SkipBuild"]'
      build_packages_json: '["kanari-node","kanari-rpc-loadgen"]'
      runs_on: windows-latest
      timeout_minutes: 120
```

Workspace tests can also run without a script:

```yaml
jobs:
  workspace-tests:
    uses: ./.github/workflows/kanari-script-runner.yml
    with:
      run_workspace_tests: true
      cache_prefix: workspace-cargo
      timeout_minutes: 180
```

## Quick index

| Script                                         | Purpose                                                         | Minimal run                                                                                                                                                                                               |
| ---------------------------------------------- | --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `run-local-four-node-tx-load.ps1`              | Deterministic four-node transaction acceptance                  | `./scripts/run-local-four-node-tx-load.ps1 -Password "@Password12345678" -Count 1 -SkipBuild -RootSyncTimeoutSec 120`                                                                                     |
| `run-local-four-node-rpc-load.ps1`             | Four-node production RPC load smoke                             | `./scripts/run-local-four-node-rpc-load.ps1 -Requests 100 -Concurrency 8 -SkipBuild`                                                                                                                      |
| `run-local-four-node-chaos.ps1`                | Four-node RPC load with follower kill/restart                   | `./scripts/run-local-four-node-chaos.ps1 -Requests 100 -Concurrency 8 -SkipBuild`                                                                                                                         |
| `run-local-four-node-10k-load.ps1`             | Wrapper for 10k-style parallel tx load                          | `./scripts/run-local-four-node-10k-load.ps1 -Password "@Password12345678" -CountPerSender 10 -AutoCoinFanout -SkipBuild`                                                                                  |
| `run-local-four-node-parallel-tx-chaos.ps1`    | Parallel sender lanes, P2P faults, crash/recovery, supply audit | `./scripts/run-local-four-node-parallel-tx-chaos.ps1 -Senders 0x3ba6... -Recipient 0x7e66... -Password "@Password12345678" -CountPerSender 1 -FundSenders -AutoCoinFanout -ChaosRounds 0 -SkipBuild`      |
| `test-rejoin-admission.ps1`                    | Verifies catch-up admission gate on rejoin                      | `./scripts/test-rejoin-admission.ps1 -Password "@Password12345678" -RpcTimeoutSec 5`                                                                                                                      |
| `run-production-rpc-load.ps1`                  | Load generator against any live endpoints                       | `./scripts/run-production-rpc-load.ps1 -RpcUrl http://127.0.0.1:6767 -Requests 1000 -Concurrency 32 -SkipBuild`                                                                                           |
| `run-production-benchmark-profile.ps1`         | Labeled production RPC benchmark profile                        | `./scripts/run-production-benchmark-profile.ps1 -RpcUrl http://127.0.0.1:6767 -Requests 1000 -Concurrency 32 -MinRps 0 -MaxP99Ms 0 -MaxClientRejectedPercent 0 -MaxEndpointImbalancePercent 0 -SkipBuild` |
| `run-rpc-load-dos.ps1`                         | PowerShell RPC load/DoS probe                                   | `./scripts/run-rpc-load-dos.ps1 -RpcUrl http://127.0.0.1:6767 -Requests 100 -Concurrency 8`                                                                                                               |
| `run-persistent-rocksdb-benchmark-profile.ps1` | RocksDB write-stall/compaction profile                          | `./scripts/run-persistent-rocksdb-benchmark-profile.ps1 -Transactions 10000 -Senders 100 -Runs 1 -SkipBuild`                                                                                              |
| `run-full-tx-benchmark-profile.ps1`            | Multiple transaction targets through the chaos runner           | `./scripts/run-full-tx-benchmark-profile.ps1 -Password "@Password12345678" -TxTargets 1000 -Wallets 10 -CrashDuringLoadRestartDelaySec 1 -SkipBuild`                                                      |
| `run-security-audit.ps1`                       | `cargo audit` and workspace clippy                              | `./scripts/run-security-audit.ps1 -NoFetch`                                                                                                                                                               |
| `run-fuzz-campaign.ps1`                        | Recurring scheduler/BCS/RPC/object fuzz campaign                | `./scripts/run-fuzz-campaign.ps1 -Hours 1 -Workers 2`                                                                                                                                                     |
| `run-crypto-fuzz-campaign.ps1`                 | Crypto proptest, clippy, and audit campaign                     | `./scripts/run-crypto-fuzz-campaign.ps1 -Minutes 10 -Cases 64 -SkipAudit`                                                                                                                                 |
| `run-adversarial-soak.ps1`                     | Long-run malformed input and Byzantine soak                     | `./scripts/run-adversarial-soak.ps1 -Minutes 10`                                                                                                                                                          |
| `run-chaos-network-campaign.ps1`               | Repeated four-node chaos runs with JSON heartbeat               | `./scripts/run-chaos-network-campaign.ps1 -Minutes 10 -Senders 0x3ba6... -Recipient 0x7e66... -Password "@Password12345678" -SkipBuild`                                                                   |
| `watch-chaos-network-campaign.ps1`             | One-shot watcher for campaign heartbeat                         | `./scripts/watch-chaos-network-campaign.ps1`                                                                                                                                                              |
| `run-long-run-security-campaign.ps1`           | Chaos campaign + adversarial soak split                         | `./scripts/run-long-run-security-campaign.ps1 -Minutes 30 -Senders 0x3ba6... -Recipient 0x7e66... -Password "@Password12345678" -SkipBuild`                                                               |
| `run-kanari-stress-lane.ps1`                   | One CLI stress-test lane (used by parallel runner)              | `./scripts/run-kanari-stress-lane.ps1 -KanariExe .\target\release\kanari.exe -KeystorePath <path> -Sender 0x3ba6... -Recipient 0x7e66... -Amount 0.000000001 -Count 10 -Rpc http://127.0.0.1:19501`       |
| `lib/four-node-common.ps1`                     | Shared helpers; dot-source, not run directly                    | `. ./scripts/lib/four-node-common.ps1`                                                                                                                                                                    |

## Shared helper library

`scripts/lib/four-node-common.ps1` is dot-sourced by all local four-node
runners, the production RPC load wrapper, the RPC DoS probe, and the stress
lane wrapper. It has no top-level side effects other than defining functions.
Callers own state, error policy, and cleanup.

Dot-source it for interactive use:

```powershell
. ./scripts/lib/four-node-common.ps1
```

| Helper                                                                                                                                                                                                                                  | Contract                                                                                                                                                         |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Set-KanariFourNodeEnv -KeystorePath <path> -Password <password>`                                                                                                                                                                       | Sets `KANARI_KEYSTORE_PATH` / `KANARI_LOAD_PASSWORD`; falls back to the development defaults; returns both values                                                |
| `Resolve-KanariFourNodeRoot -ScriptDirectory <dir>`                                                                                                                                                                                     | Resolves the repository root from `$PSScriptRoot`                                                                                                                |
| `Get-KanariFourNodeExePaths -RepoRoot <root> [-BuildProfile release\|debug]`                                                                                                                                                            | Returns `NodeExe` and `KanariExe` paths                                                                                                                          |
| `New-KanariFourNodeRunRoot -RepoRoot <root> -Prefix <name>`                                                                                                                                                                             | Creates `.kanari-runlogs\<prefix>-<yyyyMMdd-HHmmss>`; returns `RunId` and `RunRoot`                                                                              |
| `Get-KanariFourNodeUrls -BaseRpcPort <port>`                                                                                                                                                                                            | Returns the four node RPC URLs (`base`, `+10`, `+20`, `+30`)                                                                                                     |
| `Format-KanariAmount -Amount <double>`                                                                                                                                                                                                  | Invariant decimal string for CLI `--amount`                                                                                                                      |
| `Format-KanariProcessArgument -Value <string>`                                                                                                                                                                                          | Quotes a value for `ProcessStartInfo.Arguments`                                                                                                                  |
| `Start-KanariLoggedProcess -Launch @{ ... }`                                                                                                                                                                                            | Single-hashtable `Start-Process` wrapper; forces `-PassThru`, hidden window; avoids PowerShell 5.1 parameter-set ambiguity                                       |
| `Invoke-KanariCliBounded -Launch @{ ... } -TimeoutSec <n>`                                                                                                                                                                              | Runs a CLI with bounded wait, kills on timeout, writes raw stdout/stderr to separate logs, returns `ExitCode`/`Text`/`Out`/`Err`                                 |
| `Invoke-KanariFourNodeBuild -RepoRoot <root> [-Packages ...] [-BuildProfile ...] -LogDirectory <dir>`                                                                                                                                   | Builds packages with `cargo`; fails fast and tails logs on non-zero exit                                                                                         |
| `New-KanariJsonRpcBody -Method <name> [-Params @{}] [-Id <n>]`                                                                                                                                                                          | Shared JSON-RPC 2.0 body                                                                                                                                         |
| `Get-KanariNearestRankPercentile -Values <int[]> -Percentile <n>`                                                                                                                                                                       | Nearest-rank percentile used by the DoS probe                                                                                                                    |
| `Invoke-KanariRpc -Url <url> -Method <name> [-Params @{}] [-Id <n>] [-TimeoutSec <n>]`                                                                                                                                                  | JSON-RPC POST via `Invoke-RestMethod`                                                                                                                            |
| `New-KanariFourNodeGenesis -NodeExe <exe> -RepoRoot <root> -RunRoot <dir> -PrepareLog <path> [-PrepareErrLog <path>]`                                                                                                                   | Runs `consensus-keygen` and `genesis-export`; returns `KeysDir`, `Genesis`, `SourceData`                                                                         |
| `New-KanariPeerAuthorityMap -NodeExe <exe> -RunRoot <dir>`                                                                                                                                                                              | Derives peer IDs and returns `peerID=0xN,...`                                                                                                                    |
| `Start-KanariValidator -NodeExe <exe> -RepoRoot <root> -RunRoot <dir> -Index <1-4> -BaseP2pPort <port> -Genesis <path> -KeysDir <dir> -PeerAuthorityArg <map> [-RpcPort <port>] [-LogSuffix <s>] [-Environment @{}] [-PassThruDetails]` | Starts one validator; temporarily applies `-Environment` only around spawn; `-PassThruDetails` returns `Name`, `Process`, `Url`, `DataDir`, `Args`, `Out`, `Err` |
| `Wait-KanariRpcReady -Urls <urls> [-Attempts <n>] [-Processes @()] [-RunRoot <dir>]`                                                                                                                                                    | Polls `kanari_health`; fails early if a process exits                                                                                                            |
| `Wait-KanariStatsConverged -Urls <urls> [-TimeoutSec <n>] [-Quiet] [-BeforeStats @{}] [-MinTxDelta <n>] [-MinHeightDelta <n>] [-MinHeight <n>] [-MinTxByUrl @{}] [-Label <s>] [-PassThru]`                                              | Waits for identical root/height/tx count plus optional floors; `-PassThru` returns the latest stats map                                                          |
| `Get-KanariStatsByUrl -Urls <urls> [-Id <n>] [-TimeoutSec <n>]`                                                                                                                                                                         | Fetches `kanari_getStats` into a URL-keyed hashtable                                                                                                             |
| `Format-KanariStats -StatsByUrl @{}`                                                                                                                                                                                                    | Formats stats as `url height=... txs=... root=...`                                                                                                               |
| `Show-KanariStats -Urls <urls>`                                                                                                                                                                                                         | Prints `STATS ...` or `OFFLINE ...` for each URL                                                                                                                 |
| `Stop-KanariStaleFourNodeProcesses -BaseP2pPort <port> -BaseRpcPort <port>`                                                                                                                                                             | Stale-node preflight scoped to the eight runner ports; fails if listeners remain                                                                                 |
| `Stop-KanariProcesses -Processes <array-or-hashtable>`                                                                                                                                                                                  | Stops tracked processes; accepts an array or node-index → process hashtable                                                                                      |

## Local four-node runners

### `run-local-four-node-tx-load.ps1`

Creates a temporary four-node devnet, runs one `kanari client stress-test`
batch through node 1, and asserts that all four nodes reach the same height,
transaction count, and state root.

```powershell
$env:KANARI_KEYSTORE_PATH = "C:\Users\TaroPukpui\.kanari\kanari_config\kanari.keystore"
$env:KANARI_LOAD_PASSWORD = "@Password12345678"

./scripts/run-local-four-node-tx-load.ps1 `
  -Password "@Password12345678" `
  -Count 1 `
  -SkipBuild `
  -RootSyncTimeoutSec 120
```

Expected acceptance output:

```text
  http://127.0.0.1:19301 height=1 txs=1 delta_txs=1 root=7f725dee74a0fc7518241a1f211e42e2a97aaa14b10a1229adce194dc8f0bd55
  http://127.0.0.1:19311 height=1 txs=1 delta_txs=1 root=7f725dee74a0fc7518241a1f211e42e2a97aaa14b10a1229adce194dc8f0bd55
  http://127.0.0.1:19321 height=1 txs=1 delta_txs=1 root=7f725dee74a0fc7518241a1f211e42e2a97aaa14b10a1229adce194dc8f0bd55
  http://127.0.0.1:19331 height=1 txs=1 delta_txs=1 root=7f725dee74a0fc7518241a1f211e42e2a97aaa14b10a1229adce194dc8f0bd55
```

| Parameter             | Default                                       | Notes                                                |
| --------------------- | --------------------------------------------- | ---------------------------------------------------- |
| `-From`               | `0x3ba63...3146`                              | Funded sender address in the keystore                |
| `-To`                 | `0x7e669...4350`                              | Recipient address                                    |
| `-Password`           | `$env:KANARI_LOAD_PASSWORD`, else dev default | CLI password boundary                                |
| `-KeystorePath`       | dev default path                              | Must contain `-From`                                 |
| `-Amount`             | `0.000000001`                                 | Invariant decimal string is passed to CLI            |
| `-Count`              | `10`                                          | Number of transactions; use `1` for exact acceptance |
| `-BaseP2pPort`        | `19300`                                       | Node P2P ports `19300/19310/19320/19330`             |
| `-BaseRpcPort`        | `19301`                                       | Node RPC ports `19301/19311/19321/19331`             |
| `-RootSyncTimeoutSec` | `60`                                          | Convergence timeout; acceptance uses `120`           |
| `-SkipBuild`          | off                                           | Reuse existing release binaries                      |

Artifacts: `.kanari-runlogs\four-node-tx-load-<timestamp>\` with `prepare*.log`,
`node*.out.log`, `node*.err.log`, `tx-load.out.log`, `tx-load.err.log`.

### `run-local-four-node-rpc-load.ps1`

Creates a temporary four-node devnet and runs `run-production-rpc-load.ps1`
against all four RPC endpoints. This is a lightweight RPC/DoS smoke runner;
it does not submit signed transactions.

```powershell
./scripts/run-local-four-node-rpc-load.ps1 -Requests 100 -Concurrency 8 -SkipBuild
```

| Parameter                       | Default           | Notes                                             |
| ------------------------------- | ----------------- | ------------------------------------------------- |
| `-Requests`                     | `100000`          | Total requests; use `100` for smoke               |
| `-Concurrency`                  | `1024`            | In-flight requests; use `8` for smoke             |
| `-TimeoutSec`                   | `5`               | Per-request timeout                               |
| `-BaseP2pPort` / `-BaseRpcPort` | `19100` / `19101` | Four-node port bases                              |
| `-IncludeMalformed`             | off               | Inject a malformed body every 17th request        |
| `-IncludeOversized`             | off               | Inject an oversized body every 23rd request       |
| `-MinRps`                       | `0`               | Passed as a gate only when greater than zero      |
| `-MaxP99Ms`                     | `0`               | Passed as a gate only when greater than zero      |
| `-MaxClientRejectedPercent`     | `0`               | Gate                                              |
| `-MaxEndpointImbalancePercent`  | `0`               | Gate                                              |
| `-KeystorePath` / `-Password`   | dev defaults      | Inherited by the load wrapper                     |
| `-SkipBuild`                    | off               | Build both `kanari-node` and `kanari-rpc-loadgen` |

Artifacts: `.kanari-runlogs\four-node-rpc-load-<timestamp>\` plus the shared
loadgen logs under `.kanari-runlogs\rpc-loadgen\`.

### `run-local-four-node-chaos.ps1`

Four-node RPC load exercise with a live process failure:

1. Baseline load across all four nodes (malformed and oversized bodies on).
2. Stop node 4 and run load across nodes 1-3.
3. Restart node 4, wait for RPC readiness and state convergence.

```powershell
./scripts/run-local-four-node-chaos.ps1 -Requests 100 -Concurrency 8 -SkipBuild
```

| Parameter                       | Default           | Notes                                    |
| ------------------------------- | ----------------- | ---------------------------------------- |
| `-Requests`                     | `20000`           | Use `100` for smoke                      |
| `-Concurrency`                  | `256`             | Use `8` for smoke                        |
| `-TimeoutSec`                   | `5`               | Per-request timeout                      |
| `-BaseP2pPort` / `-BaseRpcPort` | `19700` / `19701` | Four-node port bases                     |
| `-SyncTimeoutSec`               | `90`              | Convergence timeout after node 4 returns |
| `-MinRps`                       | `0`               | Optional load gate                       |
| `-MaxP99Ms`                     | `0`               | Optional load gate                       |
| `-MaxEndpointImbalancePercent`  | `10`              | Default imbalance gate                   |
| `-KeystorePath` / `-Password`   | dev defaults      | Inherited by the load wrapper            |
| `-SkipBuild`                    | off               | Reuse existing release binaries          |

Success marker: `Four-node chaos exercise complete`. Artifacts:
`.kanari-runlogs\four-node-chaos-<timestamp>\`.

### `run-local-four-node-10k-load.ps1`

Convenience wrapper around the parallel transaction chaos runner for
10k-style runs (default `2` senders × `5000` transactions). It does not run its
own devnet; the child runner does.

```powershell
./scripts/run-local-four-node-10k-load.ps1 `
  -Password "@Password12345678" `
  -CountPerSender 10 `
  -AutoCoinFanout `
  -SkipBuild
```

| Parameter group  | Parameters and defaults                                                                                                                                                                         |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Identity         | `-Senders` (two dev wallets), `-Recipient` (dev wallet B), `-SelfRecipient` (off), `-Password` (`KANARI_LOAD_PASSWORD`), `-KeystorePath` (dev default)                                          |
| Load             | `-CountPerSender` `5000`, `-Amount` `0.000000001`, `-TempWalletCount` `0`                                                                                                                       |
| Funding          | `-FaucetCoinsPerSender` `16`, `-FanoutBatchSize` `64`, `-LaneTimeoutSec` `900`, `-AutoCoinFanout` (off)                                                                                         |
| P2P              | `-P2pChannelCapacity` `8192`, `-MaxConcurrentSyncMessages` `512`                                                                                                                                |
| Faults           | `-ChaosRounds` `0`, `-CrashDuringLoadRounds` `0`, `-CrashDuringLoadPattern` `follower`, `-CrashDuringLoadFirstDelaySec` `10`, `-CrashDuringLoadRestartDelaySec` `1`, `-RecoveryAuditRounds` `0` |
| Profiling        | `-ProfileIntervalSec` `0`, `-StartLinuxPerfRecorders`, `-PerfSampleHz` `99`, `-PerfDurationSec` `30`                                                                                            |
| Sync/ports/build | `-RootSyncTimeoutSec` `300`, `-BaseP2pPort` `21400`, `-BaseRpcPort` `21401`, `-BuildProfile` `release`, `-SkipBuild`                                                                            |

The default fanout of `16` is only sufficient for very small counts. For the
5000 default, use `-AutoCoinFanout` (raises fanout to `10032` with the default
reserve) or pass a sufficiently large `-FaucetCoinsPerSender`.

### `run-local-four-node-parallel-tx-chaos.ps1`

The main transaction chaos runner. It creates a temporary devnet, optionally
copies/creates wallets, funds and fans out native coins, starts one parallel
CLI lane per sender, injects P2P faults, kills/restarts validators, waits for
root convergence, and audits native supply on every node.

```powershell
$env:KANARI_KEYSTORE_PATH = "C:\Users\TaroPukpui\.kanari\kanari_config\kanari.keystore"
$env:KANARI_LOAD_PASSWORD = "@Password12345678"

./scripts/run-local-four-node-parallel-tx-chaos.ps1 `
  -Senders '0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146' `
  -Recipient '0x7e669cf309e9761beb2083d12564895ccea1389f2851a479edcc0d1ad19e4350' `
  -Password '@Password12345678' `
  -CountPerSender 1 `
  -FundSenders `
  -AutoCoinFanout `
  -ChaosRounds 0 `
  -SkipBuild `
  -RootSyncTimeoutSec 120
```

| Parameter group     | Parameters and defaults                                                                                                                                                                                                                                                                                    |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Required identity   | `-Senders` (required, comma-separated addresses accepted), `-Recipient` (required)                                                                                                                                                                                                                         |
| Wallet/password     | `-Password` (`KANARI_LOAD_PASSWORD`), `-KeystorePath` (`KANARI_KEYSTORE_PATH`), `-TempWalletCount` `0`                                                                                                                                                                                                     |
| Transaction load    | `-CountPerSender` `100`, `-Amount` `0.000000001`, `-SelfRecipient` (off; recipient becomes the sender address)                                                                                                                                                                                             |
| Funding             | `-FundSenders` (off), `-FaucetAmount` `0.0001`, `-FaucetCoinsPerSender` `2`, `-CoinReserveBuffer` `32`, `-FanoutBatchSize` `64`, `-AutoCoinFanout` (off)                                                                                                                                                   |
| Timeouts            | `-LaneTimeoutSec` `900`, `-FundingCommitTimeoutSec` `180`, `-LoadCommitTimeoutSec` `180`, `-RootSyncTimeoutSec` `120`                                                                                                                                                                                      |
| P2P fault injection | `-P2pNamespace` (auto), `-P2pChannelCapacity` `1024`, `-MaxConcurrentSyncMessages` `128`, `-P2pPublishDelayMs` `0`, `-P2pDuplicatePublishes` `0`, `-P2pReorderBestEffort` (off)                                                                                                                            |
| Crash/recovery      | `-ChaosRounds` `2`, `-CrashDuringLoadRounds` `0`, `-CrashDuringLoadPattern` `follower`, `-CrashDuringLoadNodes` `@()`, `-CrashDuringLoadFirstDelaySec` `10`, `-CrashDuringLoadRestartDelaySec` `1`, `-RecoveryAuditRounds` `0`, `-RecoveryRestartDelaySec` `3`, `-ProtectedRpcNodes` `@()` (auto-selected) |
| Profiling           | `-ProfileIntervalSec` `0`, `-StartLinuxPerfRecorders`, `-PerfSampleHz` `99`, `-PerfDurationSec` `30`                                                                                                                                                                                                       |
| RPC probes          | `-RpcAdversarialRequests` `0`, `-RpcAdversarialConcurrency` `32`, `-IncludeOversizedRpcAdversarial` (off)                                                                                                                                                                                                  |
| Ports/build         | `-BaseP2pPort` `19500`, `-BaseRpcPort` `19501`, `-BuildProfile` `release`, `-SkipBuild` (off)                                                                                                                                                                                                              |

Important behavior:

- The keystore is copied into the run directory; per-lane copies are also
  created and removed in `finally`.
- `-FundSenders` runs faucet and native fanout before load starts; preflight
  verifies each sender holds at least the required number of KANARI coins.
- P2P faults are injected only after clean bootstrap/funding convergence;
  validators are restarted one at a time.
- `-ProtectedRpcNodes` controls which RPC endpoints client lanes use. Chaos
  targets are the remaining nodes. Do not protect all four nodes when
  `-ChaosRounds` is greater than zero.
- Success markers include `PRE-FLIGHT`, `LANE`, `NATIVE SUPPLY SYNCED`,
  `SYNCED`, and `Four-node parallel tx chaos complete`.

Artifacts: `.kanari-runlogs\four-node-parallel-tx-chaos-<timestamp>\` with
`lane*.tx.*.log`, `tx-lane-metrics.csv`, `profile-samples.csv`,
`profile-summary.json`, `flamegraph-targets.csv`, funding/fanout logs, and
per-node logs.

### `test-rejoin-admission.ps1`

End-to-end scenario for the rejoin catch-up admission gate:

1. Starts four validators with real peer-to-authority mappings.
2. Stops node 4 and submits transactions through the active nodes.
3. Captures a fresh signed submit payload through a local proxy and
   intentionally prevents it from committing on healthy peers.
4. Restarts node 4 from an empty data dir with sync concurrency `1` and a
   scoped `100 ms` publish delay.
5. Waits for the trusted-peer height-gap checkpoint log, then probes raw RPC
   until the node reports it is catching up.
6. Verifies the same signed transaction is accepted after catch-up and all
   four nodes converge to the same root.

This script does not build anything. `target\release\kanari-node.exe` and
`target\release\kanari.exe` must already exist.

```powershell
./scripts/test-rejoin-admission.ps1 -Password "@Password12345678" -RpcTimeoutSec 5
```

| Parameter                              | Default             | Notes                                                                                  |
| -------------------------------------- | ------------------- | -------------------------------------------------------------------------------------- |
| `-From` / `-To`                        | dev wallets         | Transaction sender/recipient                                                           |
| `-KeystorePath`                        | dev default path    | Must contain `-From`                                                                   |
| `-Password`                            | `@Password12345678` | CLI password boundary                                                                  |
| `-Amount`                              | `0.000000001`       | Invariant decimal string                                                               |
| `-Count`                               | `5`                 | Initial transaction batch                                                              |
| `-SyncAdmissionLag`                    | `5`                 | Sets `KANARI_SYNC_ADMISSION_LAG` for the scenario                                      |
| `-BaseP2pPort` / `-BaseRpcPort`        | `19600` / `19601`   | Four-node port bases                                                                   |
| `-CaptureProxyPort`                    | `19690`             | Local submit-capture proxy                                                             |
| `-RpcTimeoutSec` / `-RawRpcTimeoutSec` | `3` / `2`           | Probe timeouts; declared range for the former is `5`–`300`, so pass `-RpcTimeoutSec 5` |
| `-SyncTimeoutSec`                      | `180`               | Convergence timeout                                                                    |

Do not set a global `KANARI_CHAOS_P2P_PUBLISH_DELAY_MS` on nodes 1-3; this
scenario applies its timing only to node 4's process spawn. The expected early
reject is JSON-RPC code `-32002` with a message like:

```text
Transaction error: Submission failed: Node is catching up with network (local_height=..., observed_height=35); retry after sync
```

Success marker: `SCENARIO PASS: rejoin admission gate rejected the early tx and
accepted it after sync`. Artifacts:
`.kanari-runlogs\rejoin-admission-<timestamp>\scenario.log`,
`scenario-result.txt`, `node*.out.log`, `node*.err.log`,
`node4.rejoin.out.log`, and `node4.rejoin.err.log`. Exit code is `1` on
`SCENARIO FAIL`.

## RPC load and benchmark scripts

### `run-production-rpc-load.ps1`

Runs `target\release\kanari-rpc-loadgen.exe` against one or more live JSON-RPC
endpoints. It does not start nodes. Zero-valued gates are disabled.

```powershell
./scripts/run-production-rpc-load.ps1 `
  -RpcUrl 'http://127.0.0.1:6767' `
  -Requests 1000 `
  -Concurrency 32 `
  -SkipBuild
```

| Parameter                      | Default   | Notes                                           |
| ------------------------------ | --------- | ----------------------------------------------- |
| `-RpcUrl`                      | required  | One or more URLs; repeat for multiple endpoints |
| `-Requests`                    | `100000`  | Total requests                                  |
| `-Concurrency`                 | `1024`    | In-flight requests                              |
| `-TimeoutSec`                  | `5`       | Per-request timeout                             |
| `-IncludeMalformed`            | off       | `--malformed-every 17`                          |
| `-IncludeOversized`            | off       | `--oversized-every 23`                          |
| `-OversizedBytes`              | `1048576` | Size of oversized bodies                        |
| `-FailOnRateLimit`             | off       | Treat HTTP 429 as failure                       |
| `-MinRps`                      | `0`       | Minimum RPS gate                                |
| `-MaxP99Ms`                    | `0`       | Maximum p99 latency gate                        |
| `-MaxClientRejectedPercent`    | `0`       | Maximum 4xx percentage gate                     |
| `-MaxEndpointImbalancePercent` | `0`       | Maximum endpoint imbalance gate                 |
| `-SkipBuild`                   | off       | Builds `kanari-rpc-loadgen` if omitted          |

Exit code is the load generator's exit code. Logs:
`.kanari-runlogs\rpc-loadgen\loadgen.out.log` and `loadgen.err.log`; build logs
go to `.kanari-runlogs\rpc-loadgen-build\`.

### `run-production-benchmark-profile.ps1`

Thin, machine-labeled wrapper over `run-production-rpc-load.ps1`. It prints
hostname, OS, and CPU count before invoking the load runner with the same
gates.

```powershell
./scripts/run-production-benchmark-profile.ps1 `
  -RpcUrl 'http://127.0.0.1:6767' `
  -Requests 1000 `
  -Concurrency 32 `
  -MinRps 0 `
  -MaxP99Ms 0 `
  -MaxClientRejectedPercent 0 `
  -MaxEndpointImbalancePercent 0 `
  -SkipBuild
```

| Parameter                                 | Default  | Notes                   |
| ----------------------------------------- | -------- | ----------------------- |
| `-RpcUrl`                                 | required | One or more endpoints   |
| `-Requests`                               | `200000` | Total requests          |
| `-Concurrency`                            | `2048`   | In-flight requests      |
| `-TimeoutSec`                             | `5`      | Per-request timeout     |
| `-MinRps`                                 | `10000`  | Default benchmark gate  |
| `-MaxP99Ms`                               | `300`    | Default benchmark gate  |
| `-MaxClientRejectedPercent`               | `10`     | Default benchmark gate  |
| `-MaxEndpointImbalancePercent`            | `5`      | Default benchmark gate  |
| `-IncludeMalformed` / `-IncludeOversized` | off      | Forwarded to the loader |
| `-FailOnRateLimit`                        | off      | Forwarded to the loader |
| `-SkipBuild`                              | off      | Forwarded to the loader |

### `run-rpc-load-dos.ps1`

Pure-PowerShell JSON-RPC load/DoS probe using `Start-Job` workers. It sends
`kanari_health` / `kanari_getStats` bodies and can mix in malformed and
oversized requests. It intentionally does not fail on HTTP 429 or ordinary 4xx
rejections; it fails on 5xx, network errors, incomplete result collection, or
an unreachable endpoint.

```powershell
./scripts/run-rpc-load-dos.ps1 `
  -RpcUrl 'http://127.0.0.1:6767' `
  -Requests 100 `
  -Concurrency 8
```

| Parameter           | Default                 | Notes                           |
| ------------------- | ----------------------- | ------------------------------- |
| `-RpcUrl`           | `http://127.0.0.1:6767` | Single endpoint                 |
| `-Requests`         | `500`                   | Total requests                  |
| `-Concurrency`      | `32`                    | Worker jobs                     |
| `-TimeoutSec`       | `5`                     | Per-request timeout             |
| `-IncludeMalformed` | off                     | Broken JSON every 17th request  |
| `-IncludeOversized` | off                     | 1 MiB `blob` every 23rd request |

Output includes `completed/...`, `success`, `rate_limited`,
`client_rejected`, `server_errors`, `network_errors`, and nearest-rank
`p50/p95/p99` latency. Exit codes: `0` pass, `1` unstable/incomplete,
`2` endpoint unreachable.

### `run-persistent-rocksdb-benchmark-profile.ps1`

Runs `kanari-benchmarks` in `--mode production` with RocksDB state and verbose
RocksDB flush/compaction profiles. The four `KANARI_BENCH_*` variables are set
only for the child process and restored in `finally`. The benchmark uses a
temporary RocksDB directory.

```powershell
./scripts/run-persistent-rocksdb-benchmark-profile.ps1 `
  -Transactions 10000 `
  -Senders 100 `
  -Runs 1 `
  -SkipBuild
```

| Parameter       | Default                 | Notes                                           |
| --------------- | ----------------------- | ----------------------------------------------- |
| `-Transactions` | `10000`                 | Range `10000`–`1000000`                         |
| `-Senders`      | same as `-Transactions` | Range `1`–`1000000`                             |
| `-Runs`         | `1`                     | Range `1`–`10`                                  |
| `-SkipBuild`    | off                     | Builds `kanari-benchmarks --release` if omitted |

### `run-full-tx-benchmark-profile.ps1`

Runs `run-local-four-node-parallel-tx-chaos.ps1` once per `-TxTargets` value.
It creates temporary wallets, enables funding, uses `-AutoCoinFanout`, and
writes a CSV summary with one row per target. Port bases advance by 100 per
target (starting at `25000` P2P / `25001` RPC).

```powershell
./scripts/run-full-tx-benchmark-profile.ps1 `
  -Password "@Password12345678" `
  -TxTargets 1000,10000 `
  -Wallets 60 `
  -CrashDuringLoadRestartDelaySec 1 `
  -SkipBuild
```

| Parameter group | Parameters and defaults                                                                                                                                                                                             |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Targets         | `-TxTargets` `@(1000, 10000)`, `-Wallets` `60` (must be ≥ `-BaseSenders.Count`)                                                                                                                                     |
| Wallets/amount  | `-Password` (required), `-KeystorePath` (`%USERPROFILE%\.kanari\kanari_config\kanari.keystore`), `-Recipient` (dev wallet B), `-BaseSenders` (two dev wallets), `-Amount` `0.000000001`                             |
| Fanout          | `-FanoutBatchSize` `64`, `-CoinReserveBuffer` `32`                                                                                                                                                                  |
| P2P             | `-P2pChannelCapacity` `8192`, `-MaxConcurrentSyncMessages` `512`                                                                                                                                                    |
| Faults          | `-ChaosRounds` `0`, `-CrashDuringLoadRounds` `0`, `-CrashDuringLoadPattern` `follower`, `-CrashDuringLoadFirstDelaySec` `10`, `-CrashDuringLoadRestartDelaySec` `0` (override with `1`), `-RecoveryAuditRounds` `1` |
| Profiling       | `-ProfileIntervalSec` `5`, `-StartLinuxPerfRecorders`, `-PerfSampleHz` `99`, `-PerfDurationSec` `30`                                                                                                                |
| Sync/build      | `-RootSyncTimeoutSec` `300`, `-BuildProfile` `release`, `-SkipBuild` (off)                                                                                                                                          |

Summary CSV:
`.kanari-runlogs\full-tx-benchmark-profile-<timestamp>.csv` with columns
`timestamp,target_txs,wallets,count_per_sender,requested_txs,base_p2p_port,base_rpc_port,status`.
A failing target writes `failed` and rethrows.

The wrapper's declared default for `-CrashDuringLoadRestartDelaySec` is `0`,
but the child chaos runner accepts only `1`–`3600`. Always pass
`-CrashDuringLoadRestartDelaySec 1` (or a larger value) as shown above.

## Security, fuzz, and campaign scripts

These scripts call native `cargo` commands. On Windows PowerShell 5.1 a
non-zero native exit code does not automatically become a terminating error;
check `$LASTEXITCODE` when composing them in automation.

### `run-security-audit.ps1`

Runs `cargo audit` and workspace clippy with `-D warnings`. `-NoFetch` skips
the advisory database fetch.

```powershell
./scripts/run-security-audit.ps1 -NoFetch
```

| Parameter  | Default | Notes                              |
| ---------- | ------- | ---------------------------------- |
| `-NoFetch` | off     | Adds `--no-fetch` to `cargo audit` |

The script has three documented inline advisory ignores (hickory-proto mDNS
entries and RSA public-key-only usage). `RUSTSEC-2026-0592` is **not** ignored;
it is resolved by pinning `url` to `2.5.9` in the lockfiles.

### `run-fuzz-campaign.ps1`

Recurring campaign over scheduler, changeset, DAG conflict, arbitrary
compressed input, RPC adversarial input, object/gas object, and SMT tests.
Clippy runs once at start unless skipped. `-IncludeIgnoredLongRuns` adds the
two long-running ignored tests.

```powershell
./scripts/run-fuzz-campaign.ps1 -Hours 1 -Workers 2
```

| Parameter                 | Default | Notes                                   |
| ------------------------- | ------- | --------------------------------------- |
| `-Hours`                  | `8`     | Range `1`–`168`                         |
| `-Workers`                | `1`     | `--test-threads` value                  |
| `-IncludeIgnoredLongRuns` | off     | Adds the two `--ignored` long-run tests |
| `-SkipClippy`             | off     | Skips the initial clippy pass           |

### `run-crypto-fuzz-campaign.ps1`

Crypto-focused campaign. Clippy and `cargo audit -q` run once unless skipped,
then crypto proptest/key/signature/compatibility tests repeat until the
deadline. `-Minutes` takes precedence over `-Hours`.

```powershell
./scripts/run-crypto-fuzz-campaign.ps1 -Minutes 10 -Cases 64 -SkipAudit
```

| Parameter     | Default | Notes                                     |
| ------------- | ------- | ----------------------------------------- |
| `-Hours`      | `8`     | Used when `-Minutes` is `0`               |
| `-Minutes`    | `0`     | Non-zero value overrides `-Hours`         |
| `-Cases`      | `4096`  | Sets `KANARI_CRYPTO_PROPTEST_CASES`       |
| `-Workers`    | `1`     | Test thread count                         |
| `-SkipClippy` | off     | Skips `cargo clippy -p kanari-crypto ...` |
| `-SkipAudit`  | off     | Skips `cargo audit -q`                    |

### `run-adversarial-soak.ps1`

Runs bounded P2P decompression and Byzantine native-block tests plus the RPC
adversarial test on every iteration. With `-RpcUrl`, also runs
`run-rpc-load-dos.ps1` (always with malformed bodies) against each endpoint.
No RPC probe is run by default.

```powershell
./scripts/run-adversarial-soak.ps1 -Minutes 10
```

| Parameter              | Default | Notes                                            |
| ---------------------- | ------- | ------------------------------------------------ |
| `-Hours`               | `8`     | Used when `-Minutes` is `0`                      |
| `-Minutes`             | `0`     | Non-zero value overrides `-Hours`                |
| `-Workers`             | `1`     | Test thread count                                |
| `-RpcUrl`              | `@()`   | Optional live endpoints for the PowerShell probe |
| `-RpcRequests`         | `500`   | Requests per endpoint                            |
| `-RpcConcurrency`      | `32`    | Probe workers                                    |
| `-IncludeOversizedRpc` | off     | Adds oversized bodies to the probe               |

### `run-chaos-network-campaign.ps1`

Repeatedly invokes `run-local-four-node-parallel-tx-chaos.ps1` until the
deadline. It writes an atomic JSON heartbeat so
`watch-chaos-network-campaign.ps1` can distinguish a healthy run from a dead
or stuck parent. It selects usable port sets by skipping Windows excluded and
currently listening ranges.

```powershell
./scripts/run-chaos-network-campaign.ps1 `
  -Minutes 10 `
  -Senders '0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146' `
  -Recipient '0x7e669cf309e9761beb2083d12564895ccea1389f2851a479edcc0d1ad19e4350' `
  -Password '@Password12345678' `
  -SkipBuild
```

| Parameter group | Parameters and defaults                                                                                                                                                                                                                      |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Duration        | `-Hours` `4`, `-Minutes` `0` (non-zero wins)                                                                                                                                                                                                 |
| Identity        | `-Senders` (required), `-Recipient` (required), `-Password` (required), `-KeystorePath` (`%USERPROFILE%\.kanari\kanari_config\kanari.keystore`)                                                                                              |
| Load            | `-CountPerSender` `250`, `-Amount` `0.000000001`, `-FaucetCoinsPerSender` `0` (selects safe minimum), `-FanoutBatchSize` `64`, `-CoinReserveBuffer` `32`, `-LaneTimeoutSec` `900`, `-FundingCommitTimeoutSec` `180`, `-AutoCoinFanout` (off) |
| Chaos           | `-ChaosRoundsPerIteration` `4`, `-CrashDuringLoadRoundsPerIteration` `0`, `-CrashDuringLoadPattern` `follower`, `-CrashDuringLoadFirstDelaySec` `10`, `-CrashDuringLoadRestartDelaySec` `1`, `-RecoveryAuditRoundsPerIteration` `1`          |
| P2P             | `-P2pPublishDelayMs` `250`, `-P2pDuplicatePublishes` `2`, `-P2pReorderBestEffort` (off), `-P2pChannelCapacity` `8192`, `-MaxConcurrentSyncMessages` `512`                                                                                    |
| Profiling       | `-ProfileIntervalSec` `15`, `-StartLinuxPerfRecorders`, `-PerfSampleHz` `99`, `-PerfDurationSec` `30`                                                                                                                                        |
| Ports           | `-BaseP2pPort` `23000`, `-BaseRpcPort` `23001` (advanced by 100 per iteration with wraparound/skipping)                                                                                                                                      |
| RPC probes      | `-RpcAdversarialRequestsPerNode` `0`, `-RpcAdversarialConcurrency` `32`, `-IncludeOversizedRpcAdversarial` (off)                                                                                                                             |
| Watch/build     | `-StatusPath` (default `.kanari-runlogs\chaos-campaign-status.json`), `-BuildProfile` `release`, `-SkipBuild` (off)                                                                                                                          |

Status JSON contains `version`, `pid`, `state` (`starting`/`running`/`completed`/`failed`), `phase`, `started_at`, `deadline`, `heartbeat_at`, `iteration`, `last_exit_code`, and `terminal_error`.

### `watch-chaos-network-campaign.ps1`

One-shot watcher for the campaign heartbeat. It exits `0` when the campaign
state is `completed`; it throws on `failed`, a missing process, or a stale
heartbeat.

```powershell
./scripts/watch-chaos-network-campaign.ps1
./scripts/watch-chaos-network-campaign.ps1 -StatusPath .\.kanari-runlogs\chaos-campaign-status.json -MaxHeartbeatAgeSec 300
```

| Parameter             | Default                                      | Notes                                   |
| --------------------- | -------------------------------------------- | --------------------------------------- |
| `-StatusPath`         | `.kanari-runlogs\chaos-campaign-status.json` | Must match the campaign's `-StatusPath` |
| `-MaxHeartbeatAgeSec` | `180`                                        | Range `15`–`3600`                       |

### `run-long-run-security-campaign.ps1`

Public campaign entry point that combines `run-chaos-network-campaign.ps1`
(80% of `-Minutes`) and `run-adversarial-soak.ps1` (remaining 20%). It uses
fixed bases `24000`/`24001`, enforces the coin-fanout invariant up front, and
creates `.kanari-runlogs\security-campaign-<timestamp>\`.

```powershell
./scripts/run-long-run-security-campaign.ps1 `
  -Minutes 30 `
  -Senders '0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146' `
  -Recipient '0x7e669cf309e9761beb2083d12564895ccea1389f2851a479edcc0d1ad19e4350' `
  -Password '@Password12345678' `
  -SkipBuild
```

| Parameter group | Parameters and defaults                                                                                                                                       |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Duration        | `-Minutes` `30` (range `1`–`10080`); each phase is clamped to at least 1 minute                                                                               |
| Identity        | `-Senders` (required), `-Recipient` (required), `-Password` (`KANARI_LOAD_PASSWORD`), `-KeystorePath` (`%USERPROFILE%\.kanari\kanari_config\kanari.keystore`) |
| Load/fanout     | `-CountPerSender` `250`, `-FaucetCoinsPerSender` `0` → `2*CountPerSender+2`, `-FanoutBatchSize` `64`                                                          |
| Chaos           | `-CrashDuringLoadPattern` `two-node`, `-CrashDuringLoadRoundsPerIteration` `2`, `-ChaosRoundsPerIteration` `4`, `-RecoveryAuditRoundsPerIteration` `2`        |
| P2P/RPC         | `-P2pPublishDelayMs` `0`, `-P2pDuplicatePublishes` `0`, `-RpcConcurrency` `64`, `-RpcRequests` `1000`, `-IncludeOversizedRpc` (off)                           |
| Build           | `-BuildProfile` `release`, `-SkipBuild` (off)                                                                                                                 |

## Low-level lane wrapper

### `run-kanari-stress-lane.ps1`

Runs exactly one `kanari client stress-test` lane. The parallel chaos runner
uses this script for each sender. Run it manually only when debugging a single
lane against an already-running RPC node.

```powershell
$env:KANARI_LOAD_PASSWORD = "@Password12345678"

./scripts/run-kanari-stress-lane.ps1 `
  -KanariExe .\target\release\kanari.exe `
  -KeystorePath "C:\Users\TaroPukpui\.kanari\kanari_config\kanari.keystore" `
  -Sender '0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146' `
  -Recipient '0x7e669cf309e9761beb2083d12564895ccea1389f2851a479edcc0d1ad19e4350' `
  -Amount 0.000000001 `
  -Count 10 `
  -Rpc 'http://127.0.0.1:19501'
```

| Parameter                | Default  | Notes                                             |
| ------------------------ | -------- | ------------------------------------------------- |
| `-KanariExe`             | required | Path to `kanari.exe`                              |
| `-KeystorePath`          | required | Sets `KANARI_KEYSTORE_PATH` for the child         |
| `-Sender` / `-Recipient` | required | Addresses passed as `--from` / `--to`             |
| `-Amount`                | required | Formatted with `Format-KanariAmount`              |
| `-Count`                 | required | Transaction count                                 |
| `-Rpc`                   | required | Primary endpoint                                  |
| `-RpcFallbackCsv`        | `''`     | Comma-separated transport-only failover endpoints |
| `-CommitTimeoutSec`      | `60`     | Range `20`–`7200`                                 |

`KANARI_LOAD_PASSWORD` must be set; the script throws if it is empty. Exit code
is the CLI's exit code. This wrapper does not start nodes or clean up
processes.

## Artifacts and logs

The devnet, chaos, and rejoin runners create timestamped directories under
`.kanari-runlogs\`:

| Prefix                             | Script                                                                          |
| ---------------------------------- | ------------------------------------------------------------------------------- |
| `four-node-tx-load-`               | `run-local-four-node-tx-load.ps1`                                               |
| `four-node-rpc-load-`              | `run-local-four-node-rpc-load.ps1`                                              |
| `four-node-chaos-`                 | `run-local-four-node-chaos.ps1`                                                 |
| `four-node-parallel-tx-chaos-`     | `run-local-four-node-parallel-tx-chaos.ps1`, `run-local-four-node-10k-load.ps1` |
| `rejoin-admission-`                | `test-rejoin-admission.ps1`                                                     |
| `rpc-loadgen-build`, `rpc-loadgen` | `run-production-rpc-load.ps1`                                                   |
| `chaos-campaign-status.json`       | `run-chaos-network-campaign.ps1`, watcher                                       |
| `full-tx-benchmark-profile-*.csv`  | `run-full-tx-benchmark-profile.ps1`                                             |
| `security-campaign-`               | `run-long-run-security-campaign.ps1`                                            |

Common log file names:

- `prepare.out.log`, `prepare.err.log`, `prepare-genesis.out.log`,
  `prepare-genesis.err.log`: key generation and genesis export.
- `node<N>-peer-id.out`, `node<N>-peer-id.err.log`: peer ID derivation.
- `node<N>.out.log`, `node<N>.err.log`: validator stdout/stderr. Suffixes such
  as `node4.rejoin.*`, `node2.restart1.*`, and `node1.p2p-fault.*` mark
  restarts.
- `*.build.out.log`, `*.build.err.log`: cargo build output.
- `tx-load.out.log`, `tx-load.err.log`: CLI transaction load.
- `lane<N>.tx.out.log`, `lane<N>.tx.err.log`, `tx-lane-metrics.csv`:
  parallel sender lanes and measured wall-clock TPS.
- `profile-samples.csv`, `profile-summary.json`, `flamegraph-targets.csv`:
  optional process/RocksDB profiling.
- `scenario.log`, `scenario-result.txt`: rejoin scenario timeline and summary.

## Troubleshooting

| Symptom                                                                  | Action                                                                                                                                                                      |
| ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Four-node ports are still in use after stale-process cleanup: ...`      | Stop the owning process, or pass different `-BaseP2pPort` / `-BaseRpcPort` values.                                                                                          |
| `FaucetCoinsPerSender=... is too low for CountPerSender=...`             | Use `-AutoCoinFanout`, or set `-FaucetCoinsPerSender` to at least `(2 * CountPerSender) + CoinReserveBuffer`.                                                               |
| `Fanout preview adjusted: ... (adaptive setup retry)`                    | Informational: the runner reduced the batch to fit the Move gas budget. Check `fanout-*.log` only if it then fails.                                                         |
| `CLI did not exit within <n>s (possible hang)`                           | Inspect the printed `*.out.log` / `*.err.log`. For rejoin, do not set a global P2P publish delay on nodes 1-3.                                                              |
| `only x/4 RPC endpoints became ready` or `node process exited early`     | Inspect `node*.err.log` and `prepare*.log`. Confirm release binaries exist.                                                                                                 |
| `state root did not converge within <n>s`                                | Increase `-RootSyncTimeoutSec`, inspect `Show-KanariStats` output and node logs. Do not weaken root/checkpoint checks.                                                      |
| `Native supply audit failed` / `untracked_supply` non-zero               | Treat as a real supply regression; inspect `kanari_getFungibleAsset` results and the failing node's log.                                                                    |
| Rejoin first probe returns `rpc_code=0` when rejection is expected       | Re-run after confirming stale node4 cleanup. The gate needs the checkpoint-derived height gap; the scenario waits for that log line plus a tick.                            |
| Rejoin error is `already executed` instead of a retryable catch-up error | The signed payload committed on healthy peers; verify the capture proxy is dropping the submit and that no stale node is serving it.                                        |
| `NativeCommandError` appears in a four-node runner                       | Check that the runner dot-sources `lib/four-node-common.ps1` and uses `Start-KanariLoggedProcess` / `Invoke-KanariCliBounded` rather than `& exe 2>&1` or `*>` redirection. |
| Campaign heartbeat stale                                                 | Run `watch-chaos-network-campaign.ps1`; inspect `terminal_error` and the campaign's last phase.                                                                             |
