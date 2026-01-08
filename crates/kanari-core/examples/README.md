# Kanari Core Examples

This directory contains examples demonstrating various features of the Kanari blockchain core.

## Available Examples

### 1. DAG Consensus Demo

**File**: [dag_consensus_demo.rs](dag_consensus_demo.rs)

Demonstrates the DAG-based consensus mechanism (Narwhal & Bullshark style) for high-throughput, low-latency transaction processing.

#### What it shows

- Creating a DAG engine with multiple authorities
- Submitting transactions to the DAG
- Producing DAG vertices (equivalent to blocks)
- Automatic checkpoint creation
- Parallel transaction execution
- Byzantine fault tolerance

#### Run the example

```bash
cargo run --package kanari-core --example dag_consensus_demo
```

#### Expected output

```
=== Kanari DAG Consensus Example ===

1. Creating blockchain engine...
   ✓ Engine created

2. Setting up authorities...
   ✓ 4 authorities configured

3. Creating DAG engine...
   ✓ DAG engine created
   ✓ Authority ID: 0xAUTH1

4. Generating test transactions...
   ✓ Generated 10 transactions

5. Submitting transactions...
   ✓ Transaction 1: a1b2c3d4...
   ✓ Transaction 2: e5f6g7h8...
   ...

6. Producing DAG vertices...

   Round 1:
   ✓ Vertex created:
     - ID: 1a2b3c4d5e6f...
     - Round: 1
     - Transactions: 10
     - Executed: 10
     - Failed: 0

   ⭐ Checkpoint #1 created!
     - Vertices committed: 1
     - Transactions committed: 10

7. Checking blockchain state...
   - Mode: DAG
   - Height: 1
   - Total transactions: 10
   - Latest checkpoint: #1
   - Checkpoint transactions: 10

8. DAG Consensus details...
   - Current round: 1
   - Number of authorities: 4
   - Checkpoints: 1

9. Parallel execution test...
   DAG consensus allows multiple authorities to create
   vertices simultaneously, leading to:
   ✓ Higher throughput (10,000+ TPS)
   ✓ Lower latency (100-500ms)
   ✓ Better resource utilization

=== DAG Consensus Example Complete ===
```

#### Key Features Demonstrated

1. **Dual-Mode Blockchain**
   - Switches between Linear Chain and DAG mode
   - Backward compatible

2. **DAG Structure**
   - Vertices with multiple parents
   - Round-based organization
   - Quorum requirements (2f+1)

3. **Consensus Protocol**
   - Bullshark-style leader election
   - 3-round commit protocol
   - Byzantine fault tolerance

4. **Parallel Execution**
   - Transactions from different senders run in parallel
   - Transactions from same sender run sequentially
   - Utilizes all CPU cores

5. **Checkpointing**
   - Automatic checkpoint creation
   - Ordered transaction finalization
   - State root computation

## Related Documentation

- [DAG_CONSENSUS.md](../DAG_CONSENSUS.md) - Complete DAG consensus guide
- [DAG_ARCHITECTURE.md](../DAG_ARCHITECTURE.md) - Architecture diagrams and flow
- [DAG_IMPLEMENTATION_SUMMARY.md](../DAG_IMPLEMENTATION_SUMMARY.md) - Implementation details
- [DAG_COMPLETION_REPORT.md](../DAG_COMPLETION_REPORT.md) - Project completion report

## Running Tests

```bash
# Run all DAG-related tests
cargo test --package kanari-core --lib dag

# Run example as test
cargo test --package kanari-core --example dag_consensus_demo
```

## Building Examples

```bash
# Build all examples
cargo build --package kanari-core --examples

# Build specific example
cargo build --package kanari-core --example dag_consensus_demo
```

## Performance Comparison

| Mode | Throughput | Latency | Parallelism |
|------|------------|---------|-------------|
| Linear Chain | ~1,000 TPS | ~2-3s | Limited |
| **DAG** | **~10,000+ TPS** | **~100-500ms** | **High** |

## Code Structure

```rust
// 1. Create base engine
let engine = Arc::new(BlockchainEngine::new()?);

// 2. Setup authorities (validators)
let authorities = vec!["auth1".to_string(), ...];

// 3. Create DAG engine
let dag_engine = DagEngine::new(
    engine.clone(),
    "auth1".to_string(),
    authorities,
)?;

// 4. Submit transactions
dag_engine.engine().submit_transaction(tx)?;

// 5. Produce vertex
let dag_info = dag_engine.produce_vertex()?;

// 6. Check checkpoint
if let Some(checkpoint) = dag_info.checkpoint {
    // Checkpoint created!
}
```

## Requirements

- Rust 1.75 or higher
- All Kanari dependencies installed
- Sufficient CPU cores for parallel execution (4+ recommended)

## License

Copyright (c) KanariNetwork, Inc.  
SPDX-License-Identifier: Apache-2.0
