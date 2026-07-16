# Kanari External Security Audit Scope

## Scope

The external review must cover the Rust workspace commit supplied to the auditor,
with priority on:

1. `crates/kanari-core`: Mysticeti vertex validation, checkpoint finalization,
   state-root construction, replay protection, and checkpoint/snapshot recovery.
2. `crates/kanari-node`: P2P framing, decompression limits, peer selection,
   checkpoint/DAG synchronization, and validator backup/restore.
3. `move-execution/v1/kanari-move-runtime-v1` and
   `crates/kanari-system-natives`: Move object ownership, gas, dynamic fields,
   supply invariants, and persistent-storage failures.
4. `crates/kanari-crypto`: key storage, consensus identity, signatures, and
   network encryption boundaries.

## Required Adversarial Evidence

- Four-validator Byzantine scenarios: invalid signatures, equivocation,
  replay, invalid parents, malformed native blocks, and conflicting checkpoints
  must never advance a checkpoint or mutate Move state.
- DoS budgets for compressed P2P payloads, malformed payloads, DAG-parent
  closure size, checkpoint sync responses, mempool lanes, and Move gas.
- Property-based fuzzing for P2P decompression and untrusted Mysticeti native
  blocks. Coverage-guided fuzzing should additionally run on Linux with ASAN.
- Crash/restart tests for the state-commit/checkpoint-metadata window and
  snapshot export/restore.

## Reproducible Commands

```powershell
cargo test -p kanari-core --lib -- --test-threads=1
cargo test -p kanari-node
cargo test -p kanari-move-runtime-v1 -- --test-threads=1
cargo clippy -p kanari-system-natives -p kanari-move-runtime-v1 -p kanari-core -p kanari-node -p kanari-rpc-server --all-targets -- -D warnings
```

## Deliverables From an Independent Auditor

Publish the reviewed commit hash, methodology, tooling versions, test duration,
findings with severity, proof-of-concept steps, remediation commits, and a
retest statement. This document is an audit scope and readiness checklist; it
is not a claim that an independent audit has already occurred.
