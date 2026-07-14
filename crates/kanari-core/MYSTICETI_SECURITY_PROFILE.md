# Mysticeti Security Profile

## Audited Source Baseline

Kanari currently vendors Mysticeti at Git commit:

```text
87925e0cc41bab65551048ea37d928cc46b94839
```

This identifier is the local source baseline, not a claim of an independent
security audit or formal verification. Any update to the vendored source must
record the new commit and rerun all consensus, Byzantine-sync, replay, and
state-root convergence tests before release.

## Consensus Signature Profile

The current Mysticeti wire format uses a fixed 64-byte Ed25519 block signature
and a 32-byte Ed25519 public key. Kanari requires explicit per-authority keys,
verifies remote DAG vertices, rejects a local key that does not match the
authority registry, and never permits dummy consensus crypto in node startup.

This profile is **not post-quantum**. Kanari transaction crypto supports PQ and
hybrid algorithms, but that does not make Mysticeti block authentication PQ.

## Required PQ Migration (Protocol Version 2)

Post-quantum consensus must be shipped as an explicit protocol upgrade, not as
an in-place type substitution. The upgrade must include:

1. A versioned block and WAL encoding with domain-separated signatures.
2. A hybrid Ed25519 + ML-DSA authority registry and proof-of-possession.
3. Verification requiring both signature components during the transition.
4. Mixed-version peer rejection before participating in quorum.
5. WAL/state migration with rollback and restore tests.
6. Bandwidth, block-size, verification-latency, and denial-of-service budgets.
7. Four-authority Byzantine tests where malformed, replayed, oversized, and
   equivocated hybrid blocks never commit.

Until all seven items pass review, releases must report consensus as Ed25519,
not quantum-safe.

## Kanari Integration Invariants

- Empty Mysticeti progress never creates an application checkpoint.
- Only a committed sub-DAG containing accepted transactions mutates Move state.
- Remote vertices are signature-checked and do not directly apply state.
- Checkpoint synchronization verifies sequence, replay protection, transaction
  signatures, and state root before mutation.
- Same committed transaction ordering must produce the same state root across
  authorities.
- A divergent peer is quarantined and cannot be selected as a sync source.
- Persistent storage is required by default on devnet, testnet, and mainnet.

## Release Evidence

At minimum run:

```powershell
cargo test -p kanari-core
cargo test -p kanari-node
cargo clippy -p kanari-core -p kanari-node --all-targets -- -D warnings
```

An external audit, fuzzing campaign, or multi-month soak test must publish its
tool version, corpus/configuration, duration, commit hash, findings, and fixes.
Absence of a crash in unit tests is not evidence that such an audit occurred.
