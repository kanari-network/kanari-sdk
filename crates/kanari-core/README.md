# Kanari Architecture

Real-time transaction network for in-game payments and asset systems.

## What is Kanari?

Kanari is a real-time transaction network designed for game economies.

It allows developers to build:

- Instant in-game payments (UID top-up)
- Real-time asset trading
- Game economy systems

Transactions execute instantly (~10 ms) and finalize securely within ~300 ms.

## Why Kanari?

Traditional systems:

- ❌ Slow settlement (seconds)
- ❌ Complex backend
- ❌ No verifiable state

Kanari:

- ✅ Instant execution
- ✅ Sub-second finality
- ✅ No gas fees
- ✅ Simple integration

---

## How it works

1. Transaction is submitted
2. Executed instantly by a small node set (~10 ms)
3. Propagated across the network (DAG)
4. Finalized by Byzantine quorum (~300 ms)

Result:

- Instant user experience
- Strong consistency

## Example: In-game payment

1. Player enters UID
2. Payment is submitted
3. Balance updates instantly
4. Transaction is finalized within 300 ms

No waiting. No gas fees.

---

## Quick Start

### DAG Mode

```rust
use kanari_core::engine::{BlockchainEngine, DagEngine};
use std::sync::Arc;

// Create base engine
let engine = Arc::new(BlockchainEngine::new()?);

// Setup authorities
let authorities = vec![
    "auth1".to_string(),
    "auth2".to_string(),
    "auth3".to_string(),
    "auth4".to_string(),
];

// Create DAG engine
let dag_engine = DagEngine::new(
    engine.clone(),
    "auth1".to_string(),
    authorities,
)?;

// Submit transactions
dag_engine.engine().submit_transaction(signed_tx)?;

// Produce vertex
let dag_info = dag_engine.produce_vertex()?;

// Check checkpoint
if let Some(checkpoint) = dag_info.checkpoint {
    println!("Checkpoint #{} created!", checkpoint.sequence);
}
```

---

## Features

### 🔗 DAG-Based Network

- **DAG Mode**: High-throughput DAG-based consensus (Mysticeti-style)
- Parallel transaction processing
- Byzantine fault tolerance

### ⚡ High Performance

- **10,000+ TPS** throughput in DAG mode
- **100-500ms** latency
- Parallel transaction execution
- Multi-core CPU utilization

### 🛡️ Byzantine Fault Tolerance

- Tolerates f = (n-1)/3 Byzantine failures
- 2f+1 quorum requirements
- Signature verification on all transactions
- Proven consensus algorithms

### 🔧 Move VM Integration

- Execute Move smart contracts
- Parallel execution for non-conflicting transactions
- Gas metering and limits
- State management with Sparse Merkle Trees

---

## Architecture

```rust
┌─────────────────────────────────────────┐
│       Application Layer                 │
│  (Move VM, Smart Contracts, TXs)        │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│       Execution Layer                   │
│  • DagEngine / BlockchainEngine         │
│  • Parallel TX execution                │
│  • State management                     │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│       Consensus Layer                   │
│  • DagConsensus (Mysticeti)             │
│  • Multi-leader selection               │
│  • Checkpoint creation                  │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│       Data Availability Layer            │
│  • DagStore                              │
│  • Vertex storage & indexing             │
│  • Merkle proofs                         │
└──────────────────────────────────────────┘
```

## Modules

### blockchain

Core blockchain data structures and operations:

- `Block`, `BlockHeader`, `Transaction`, `SignedTransaction`
- `Blockchain` - DAG-based blockchain state
- `DagVertex`, `Checkpoint` - DAG structures
- `DagStore`, `DagConsensus` - DAG consensus protocol
- Merkle tree implementation

### engine

Blockchain execution engine:

- `BlockchainEngine` - main engine with Move VM
- `DagEngine` - DAG consensus engine
- `produce_vertex()` - DAG vertex production
- Parallel transaction execution

---

## Performance

|    Metric   |       DAG Mode      |
|-------------|---------------------|
| Throughput  | ~50,000+ TPS        |
| Latency     | ~100-300ms          |
| Parallelism | High (N validators) |
| CPU Usage   | ~80%+ (optimized)   |

## DAG Consensus

Implements a Mysticeti-style DAG consensus protocol:

### Data Availability

- Vertices created in parallel by multiple authorities
- Each vertex references 2f+1 parents
- Efficient storage with HashMap indexing

### Ordering

- Deterministic multi-leader ordering
- 3-round decision protocol
- Automatic checkpoint creation

### Execution

- Parallel transaction execution
- Per-sender sequence enforcement
- State management with snapshots

---

## Documentation

- [DAG_CONSENSUS.md](DAG_CONSENSUS.md) - Complete DAG consensus guide
- [DAG_ARCHITECTURE.md](DAG_ARCHITECTURE.md) - Architecture diagrams
- [DAG_IMPLEMENTATION_SUMMARY.md](DAG_IMPLEMENTATION_SUMMARY.md) - Implementation details
- [DAG_COMPLETION_REPORT.md](DAG_COMPLETION_REPORT.md) - Project completion report
- [examples/README.md](examples/README.md) - Examples guide

## Examples

Run the DAG consensus demo:

```bash
cargo run --package kanari-core --example dag_consensus_demo
```

See [examples/](examples/) directory for more.

## Testing

```bash
# Run all tests
cargo test --package kanari-core

# Run DAG-specific tests
cargo test --package kanari-core --lib dag

# Run with output
cargo test --package kanari-core -- --nocapture
```

## Dependencies

- `kanari-move-runtime-v1` - Move VM integration
- `kanari-crypto` - Cryptographic primitives (Blake3, Ed25519, etc.)
- `kanari-types` - Common type definitions
- `smt` - Sparse Merkle Tree implementation
- `move-core-types` - Move language types

## Features Flags

Currently no feature flags. All features are enabled by default.

## Building

```bash
# Check compilation
cargo check --package kanari-core

# Build
cargo build --package kanari-core

# Build with optimizations
cargo build --package kanari-core --release
```

## Benchmarks

Run benchmarks with:

```bash
cargo bench --package kanari-core
```

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) in the workspace root.

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
