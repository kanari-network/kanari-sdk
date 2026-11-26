# Kanari External Security Audit Scope

## Scope

The external review must cover the Rust workspace commit supplied to the auditor,
with priority on:

1. `crates/kanari-core`: Mysticeti vertex validation, checkpoint finalization,
   state-root construction, replay protection, and checkpoint/snapshot recovery.
2. `crates/kanari-node`: P2P framing, decompression limits, peer selection,
   checkpoint/DAG synchronization, and validator backup/restore.
3. `move-execution/v2/kanari-move-runtime-v2` and
   `crates/kanari-system-natives`: Move object ownership, gas, dynamic fields,
   supply invariants, persistent-storage failures, and framework module loading.
4. `crates/kanari-crypto`: key storage, consensus identity, signatures, and
   network encryption boundaries.
5. Framework upgrade control in `kanari-move-runtime-v2`: expected-hash pinning,
   dependency verification, module compatibility, and rollback handling. The
   retired `kanari-move-runtime-v1` is disconnected from the workspace (source
   retained for reference only) and out of scope.

## Required Adversarial Evidence

- Four-validator Byzantine scenarios: invalid signatures, equivocation,
  replay, invalid parents, malformed native blocks, and conflicting checkpoints
  must never advance a checkpoint or mutate Move state.
- Multi-hour four-validator network chaos: kill/restart validators, delayed
  P2P publish, duplicate gossip, transaction pressure, and post-restart root
  convergence.
- DoS budgets for compressed P2P payloads, malformed payloads, DAG-parent
  closure size, checkpoint sync responses, mempool lanes, and Move gas.
- Property-based fuzzing for P2P decompression, conflict-aware speculative
  scheduling, retry paths, and untrusted Mysticeti native blocks.
  Coverage-guided fuzzing should additionally run on Linux with ASAN.
- Production-hardware RPC/load benchmark with commit hash, hardware profile,
  validator count, RPC limits, achieved RPS, p99 latency, rejection rate, and
  final state-root convergence.
- Crash/restart tests for the state-commit/checkpoint-metadata window and
  snapshot export/restore.
- Framework upgrade evidence: changed on-disk bytecode must fail startup when
  `KANARI_FRAMEWORK_EXPECTED_HASH_0X1`, `_0X2`, or `_0X3` is pinned to the old
  hash; validators must not silently accept a downgrade or load divergent
  module sets. See `SECURITY/FRAMEWORK_UPGRADE_POLICY.md`.
- Release evidence: signed artifacts, SBOM, and provenance must verify to the
  tagged commit. See `SECURITY/RELEASE_SIGNING_POLICY.md`.

## Reproducible Commands

```powershell
cargo test -p kanari-core --lib -- --test-threads=1
cargo test -p kanari-node
cargo test -p kanari-move-runtime-v2 -- --test-threads=1
cargo test -p kanari-core long_run_byzantine_native_blocks_cannot_advance_checkpoint -- --ignored --test-threads=1
cargo test -p kanari-node long_run_malformed_compressed_payloads_are_bounded -- --ignored --test-threads=1
cargo clippy -p kanari-system-natives -p kanari-move-runtime-v2 -p kanari-core -p kanari-node -p kanari-rpc-server --all-targets -- -D warnings
.\scripts\run-fuzz-campaign.ps1 -Hours 8 -Workers 1 -IncludeIgnoredLongRuns
.\scripts\run-chaos-network-campaign.ps1 -Hours 6 -Password '<secret>' -Senders @('<4+ wallet addresses>') -Recipient '<recipient>'
.\scripts\run-production-benchmark-profile.ps1 -RpcUrl @('<gateway-or-node-rpc-urls>') -Requests 200000 -Concurrency 2048
```

## Deliverables From an Independent Auditor

Publish the reviewed commit hash, methodology, tooling versions, test duration,
findings with severity, proof-of-concept steps, remediation commits, and a
retest statement. This document is an audit scope and readiness checklist; it
is not a claim that an independent audit has already occurred.
