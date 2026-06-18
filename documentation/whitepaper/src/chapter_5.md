## 5. Infrastructure & Networking

### 5.1 Rust Runtime

Kanari's node and core services are written in Rust to keep concurrency, storage control, and networking behavior explicit.

### 5.2 Networking Model

A running node combines several responsibilities:

- peer-to-peer communication between authorities
- DAG metadata exchange
- mempool transaction propagation
- JSON-RPC access for clients and tooling

The network layer must allow validators to sync without turning synchronization into fake blockchain progress.

### 5.3 Storage Model

Storage efficiency is a major part of correctness and performance. The current direction emphasizes:

- compact checkpoint metadata
- incremental state-root maintenance where possible
- separation between summary records and heavier transaction payloads

This reduces unnecessary recomputation and keeps persistence work closer to the actual transaction delta.

### 5.4 State Roots and Divergence Detection

State roots are not cosmetic metrics. They are how operators verify that nodes which claim the same checkpoint height also agree on the same committed state.

If two nodes report different roots at the same checkpoint height, that indicates divergence and should trigger investigation or resync.

### 5.5 Deployment Notes

Kanari supports single-node and multi-node developer setups. In multi-node environments, validators should be restarted from consistent binaries and data expectations so explorers and RPC queries are interpreting the same runtime behavior.
