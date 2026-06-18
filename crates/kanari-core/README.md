# Kanari Core

`kanari-core` is the transaction execution and checkpoint state layer used by
`kanari-node`.

It connects:

- signed Kanari transactions;
- the Move runtime;
- persistent blockchain and transaction indexes;
- Sparse Merkle Tree state roots;
- Mysticeti DAG metadata;
- checkpoint production and synchronization.

This crate is not a standalone consensus demo. Normal applications should use
the Kanari RPC API or SDK instead of constructing `DagEngine` directly.

## Current Design

Kanari uses transaction-driven checkpoints.

```text
RPC or P2P transaction
        |
        v
signature, replay, and sequence validation
        |
        v
verified mempool
        |
        v
deterministic Move execution
        |
        v
Mysticeti-backed DAG vertex
        |
        v
checkpoint + state root + transaction indexes
```

A checkpoint is created only when at least one accepted transaction is waiting
in the mempool. An idle node does not increment blockchain height.

See [CHECKPOINT_DESIGN_INVARIANTS.md](CHECKPOINT_DESIGN_INVARIANTS.md) before
changing checkpoint production, DAG synchronization, transaction counting, or
state-root behavior.

## Important Invariants

- Empty mempools do not create checkpoints.
- Empty mempools do not increment blockchain height.
- Network DAG vertices update consensus metadata only.
- Receiving a DAG vertex does not directly execute its transactions.
- Lagging nodes recover blockchain state through checkpoint synchronization.
- Transaction counts represent unique committed signed transactions.
- Move system prologues, clock updates, and peer messages are not transactions.
- Nodes with the same committed transaction history must produce the same state
  root.

These rules are intentional product behavior.

## Main Components

### `BlockchainEngine`

The main execution and state API.

Responsibilities include:

- loading in-memory or persistent state;
- validating and accepting signed transaction batches;
- deterministic parallel Move execution;
- checkpoint production;
- checkpoint replay and synchronization;
- transaction history and hash indexes;
- account, block, transaction, and network statistics.

Constructors:

```rust
use kanari_core::BlockchainEngine;

let persistent = BlockchainEngine::new_dir("./kanari-db")?;
let default_path = BlockchainEngine::new()?;
let test_engine = BlockchainEngine::new_in_memory()?;
```

Production checkpoint creation also requires configured authorities and an
explicit consensus signing key.

### `DagEngine`

Internal Mysticeti integration used by `BlockchainEngine`.

It:

- proposes local DAG vertices from verified mempool transactions;
- tracks known local and network vertices;
- persists DAG metadata;
- rejects zero-transaction local production.

Network vertices are not finalized directly into blockchain checkpoints.

### `Blockchain`

Stores retained checkpoint history and query indexes:

- committed checkpoints;
- executed transaction hashes;
- transaction-to-checkpoint locations;
- unique committed transaction count.

### Move Runtime

`kanari-move-runtime-v1` executes Move transactions and maintains logical state.
State roots are generated through the workspace `smt` crate.

## Transaction Flow

1. A signed transaction arrives through RPC or P2P.
2. The engine verifies its signature and immutable transaction hash.
3. Replay, duplicate, sender sequence, and mempool limits are checked.
4. The transaction enters the verified mempool.
5. The node allows a short gossip window for other authorities.
6. Pending transactions are sorted deterministically.
7. Move execution prepares the next state and state root.
8. Mysticeti supplies DAG vertex metadata.
9. The checkpoint, state, transaction payloads, and indexes are persisted.
10. The checkpoint is broadcast so lagging nodes can synchronize.

Calling read-only RPC methods must not create transactions or checkpoints.

## Persistence

Persistent mode separates checkpoint metadata from transaction payload storage.
Transaction hashes and locations are indexed so RPC and explorer queries do not
need to scan the entire blockchain history for normal lookups.

On startup, the engine:

- loads checkpoint and Move state;
- hydrates retained transaction payloads;
- rebuilds in-memory replay and location indexes;
- repairs recoverable metadata from persisted DAG state when required.

## Public Types

The crate exports:

- `BlockchainEngine`
- `BlockchainStats`
- `BlockData`
- `FullBlockData`
- `Checkpoint`
- `CheckpointSyncData`
- `CheckpointProductionInfo`
- `DagVertex`
- `DagProductionPolicy`
- `PersistentDagState`
- `ConsensusRuntimeProtocol`

`DagEngine` is exposed for internal workspace integration, but application code
should normally use `BlockchainEngine`, RPC, or an SDK.

## Validation and Safety

The engine enforces:

- transaction signature verification;
- per-sender sequence ordering;
- duplicate and replay rejection;
- checkpoint sequence validation;
- deterministic transaction ordering;
- state-root verification during checkpoint sync;
- optional strict persistence and supply invariant guards;
- explicit consensus signing keys.

Do not weaken these checks to improve benchmark results.

## Build and Test

```powershell
cargo check -p kanari-core -p kanari-node -p kanari-rpc-server
cargo test -p kanari-core
cargo test -p kanari-node sync::tests
```

Build an optimized node with:

```powershell
cargo build -p kanari-node --release
```

After changing consensus or synchronization, verify a four-node network:

1. Start all authorities.
2. Confirm idle heights remain unchanged.
3. Submit one signed transaction through one node.
4. Confirm every node converges on height, unique transaction count, and state
   root.
5. Restart all nodes without resetting data.
6. Confirm restart does not create an additional transaction or checkpoint.

## Performance

Performance depends on transaction type, sender conflicts, Move execution,
storage, CPU, memory, build profile, and batch size.

Use `kanari-benchmarks` for measurements. Do not place fixed TPS or latency
claims in this README unless they identify the exact workload, hardware, build,
and measurement method.

## Dependencies

Key workspace dependencies:

- `kanari-move-runtime-v1`
- `kanari-types`
- `kanari-crypto`
- `smt`
- `mysticeti-consensus`
- `mysticeti-dag`

## License

Copyright (c) KanariNetwork, Inc.

SPDX-License-Identifier: Apache-2.0

## References

1. **Narwhal and Tusk**: DAG-based Mempool and Efficient BFT Consensus
   - [arXiv:2105.11827](https://arxiv.org/abs/2105.11827)

2. **Mysticeti**: Reaching the Latency Limits with Uncertified DAGs
   - [NDSS 2025](https://www.ndss-symposium.org/ndss-paper/mysticeti-reaching-the-latency-limits-with-uncertified-dags/)

3. **Sui Consensus**: Real-world DAG consensus implementation
   - [docs.sui.io/learn/architecture/consensus](https://docs.sui.io/learn/architecture/consensus)
