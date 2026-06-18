## 2 Architecture & Mysticeti Consensus

### 2.1 DAG Ordering in Kanari

Kanari uses a Directed Acyclic Graph to exchange ordering metadata between authorities. DAG traffic helps validators agree on the order of work, but it does not directly define blockchain state.

That distinction is intentional:

- DAG sync must not create synthetic checkpoints
- network liveness must not inflate blockchain height
- state roots must come from executed transaction effects

### 2.2 Mysticeti-Style Authority Flow

The current implementation follows a Mysticeti-oriented authority model.

At a high level:

1. Authorities receive and propagate transactions.
2. Validators exchange DAG vertices and dependency references.
3. Ordered transaction batches move into execution.
4. A checkpoint is finalized only if there are pending transactions to commit.

This keeps consensus progress and state progress aligned without forcing empty blocks during quiet periods.

### 2.3 Checkpoint Invariants

Kanari currently treats the following rules as architectural invariants:

- no pending transactions means no new checkpoint
- DAG metadata propagation alone must not advance chain height
- nodes catching up from peers should replay committed work instead of inventing local progress
- state roots at the same checkpoint height should converge across honest nodes

These rules make explorer output and operator debugging much easier to trust.

### 2.4 Security Model

Kanari assumes partial faults and Byzantine behavior are possible. Safety depends on deterministic execution and validator agreement on the ordered transaction set.

Operationally this means:

- root mismatches at the same height are treated as real divergence signals
- lagging nodes must resync from committed history
- checkpoint persistence must reflect executed state, not transient consensus chatter

### 2.5 Component Overview

The main runtime components are:

- `kanari-core`: execution orchestration, checkpoint production, persistence, DAG integration
- `kanari-node`: runtime startup, networking, synchronization, RPC bootstrapping
- `kanari-rpc-api`: public method surface and shared RPC types
- `kanari-rpc-server`: JSON-RPC handlers and query endpoints
- `kanari-move-runtime-v1`: Move execution and state application
- `crates/smt`: sparse Merkle tree support for state-root maintenance

This separation lets Kanari optimize execution and storage without weakening the consensus model.
