# Kanari Core

Core blockchain engine and consensus implementation for Kanari Network.

## Features

### 🔗 DAG-Based Blockchain

- **DAG Mode**: High-throughput DAG-based consensus (Narwhal & Bullshark)
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
│  • DagConsensus (Bullshark)             │
│  • Leader election                      │
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

## Performance

|    Metric   |       DAG Mode      |
|-------------|---------------------|
| Throughput  | ~10,000+ TPS        |
| Latency     | ~100-500ms          |
| Parallelism | High (N validators) |
| CPU Usage   | ~80%+ (optimized)   |

## DAG Consensus

Implements Narwhal & Bullshark consensus protocol:

### Data Availability

- Vertices created in parallel by multiple authorities
- Each vertex references 2f+1 parents
- Efficient storage with HashMap indexing

### Ordering

- Leader-based ordering (round-robin)
- 3-round commit protocol
- Automatic checkpoint creation

### Execution

- Parallel transaction execution
- Per-sender sequence enforcement
- State management with snapshots

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

- `kanari-move-runtime` - Move VM integration
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

2. **Bullshark**: DAG BFT Protocols Made Practical
   - [arXiv:2201.05677](https://arxiv.org/abs/2201.05677)

3. **Sui Consensus**: Real-world DAG consensus implementation
   - [docs.sui.io/learn/architecture/consensus](https://docs.sui.io/learn/architecture/consensus)
