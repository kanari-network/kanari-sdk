# Kanari

Kanari is a real-time payment network designed for instant transactions across all industries.

It allows developers to build fee-free payment applications — such as e-commerce checkout, cross-border remittances, digital content monetization, and in-game purchases — with sub-second finality and no user fees.

---

## What can you build with Kanari?

- 🛍️ E-commerce payment processing
- 💸 Cross-border remittances  
- 🎮 In-game payment systems (UID top-up)
- 📱 Digital content monetization (pay-per-article)
- 🔁 Real-time peer-to-peer transfers
- 🏦 Financial service applications

---

## Why Kanari?

Traditional systems:

- ❌ Slow settlement (seconds to days)
- ❌ High transaction fees (2-10% + fixed costs)
- ❌ Complex integration requirements
- ❌ No verifiable state

Kanari:

- ⚡ Instant execution (~10 ms)
- 🔒 Secure finality (~300 ms)
- 💸 No user fees (businesses cover infrastructure costs)
- 🧩 Simple integration with existing systems
- 🛡️ **Post-quantum security** (Hybrid PQC signatures)

---

## 🛡️ Quantum-Resistant Infrastructure

Unlike traditional networks, Kanari is built for the post-quantum era:

- **Hybrid Signature Scheme:** Combines standard Ed25519/K256 with NIST-approved **Dilithium3**.
- **Pragmatic Security:** Ensures 100% compatibility with current blockchain explorers while being resistant to Shor's algorithm.
- **Future-Proof:** Seamless upgrade path from classical to quantum-safe cryptography without breaking existing integrations.

---

## How it works

1. Transaction is submitted
2. Executed instantly by a small node set (~10 ms)
3. Propagated across the network (DAG)
4. Finalized by Byzantine quorum (~300 ms)

Result:

- Instant user experience
- Strong consistency and correctness
- Universal payment infrastructure

---

## Example: Universal Payment Scenarios

**E-commerce**:

1. Customer checks out with email address
2. Payment processes instantly
3. Order confirmed within ~300 ms

**Remittance**:

1. User sends money internationally  
2. Recipient receives funds instantly
3. Near-zero fees vs traditional 5-10% charges

**Gaming**:

1. Player enters UID
2. Payment is submitted
3. Balance updates instantly
4. Transaction finalized within ~300 ms

**Banking & Enterprise**:

- **Interoperability:** Hybrid keys allow seamless integration with existing banking HSMs (Hardware Security Modules) while providing an upgrade path to Quantum-Safe standards.
- **Compliance Ready:** Built-in audit trails and verifiable state for regulatory requirements.

No waiting. No user fees.

---

## Developer Quick Start

### Prerequisites

- Rust and Cargo (stable channel recommended)
- Clang, LLVM, CMake (for RocksDB)
- Libssl-dev, pkg-config

### Build CLI

```powershell
cargo build -p kanari
```

### Run CLI

```powershell
# List wallets (first run will bootstrap genesis)
cargo run -p kanari -- keytool list
```

### Move commands

```powershell
# Create new Move package
cargo run -p kanari -- move new my_token

# Test Move package
cargo run -p kanari -- move test ./my_token
```

**Note:** On first run, the CLI performs a Rust-side genesis that mints initial supply. To reset state, remove `~/.kanari/kanari-db/` and rerun.

---

## Testing

```powershell
# Run all tests
cargo test

# Run specific crate tests
cargo test -p kanari-types
```

---

## Project Structure

- `crates/kanari` — CLI binary and bootstrap logic
- `crates/kanari-types` — domain types (accounts, balances, TransferRecord)
- `crates/kanari-move-runtime` — Move VM integration (execution, validation, persistence)
- `crates/kanari-crypto` — key management, signing, crypto utilities (**Standardized on Hybrid Signatures**)
- `crates/kanari-frameworks/packages/kanari-system` — Move packages (on-chain modules)
- `crates/kanari-core` — blockchain engine and DAG consensus
- `crates/centauri` — consensus implementation
- `third_party/move` — bundled Move toolchain (path dependencies)

---

## Local State

- **RocksDB path**: `~/.kanari/kanari-db/`
- **State storage**: Serialized `MoveVMState` under key `"state"`
- **Reset state**: Delete the DB directory and restart

---

## Architecture (Advanced)

Kanari is a **Modular Execution Client** inspired by the efficiency of **Reth**, reimagined for the **Move VM**:

- **Parallel Execution:** Transactions are executed in parallel, achieving **52,000+ TPS**.
- **Event-driven (Blockless):** No block dependency for instant user feedback.
- **SDK-First:** Every component is designed as a crate for developers to extend or integrate.
- **DAG-based propagation:** Narwhal & Bullshark consensus protocol for high throughput.
- **Byzantine quorum consensus:** 2f+1 finality guarantee.
- **Sparse Merkle Tree:** Efficient state verification and proofs.

Transactions are executed instantly and finalized asynchronously through DAG consensus.

For detailed architecture documentation:

- [Kanari Core README](crates/kanari-core/README.md)
- [DAG Architecture](crates/centauri/DAG_ARCHITECTURE.md)
- [System ER Diagram](DOCS/SYSTEM_ER.md)

---

## Key Files for Development

- `crates/kanari/src/main.rs` — CLI entry point and bootstrap
- `crates/kanari-move-runtime/src/move_runtime.rs` — Move VM integration
- `crates/kanari-types/src/transfer.rs` — TransferRecord validation
- `crates/kanari-frameworks/packages/kanari-system` — Move modules

---

## Documentation

### Core Documentation

- [Move CLI Guide](crates/kanari/MOVE_CLI_GUIDE.md) — Complete Move development guide
- [Kanari Core](crates/kanari-core/README.md) — Engine and consensus details
- [Whitepaper](documentation/whitepaper/) — Technical whitepaper
- [Developer Book](documentation/book/) — Comprehensive docs

### Centauri Consensus (Advanced)

Kanari uses **DAG-based consensus** (Narwhal & Bullshark protocol) to achieve instant execution and sub-second finality.

#### Why DAG Consensus?

Traditional blockchain consensus:

- ❌ Sequential block production (slow)
- ❌ Single leader bottleneck
- ❌ High latency (seconds to minutes)

Kanari's DAG approach:

- ✅ Parallel vertex creation (multiple authorities)
- ✅ No single point of failure
- ✅ Sub-second finality (~300ms)
- ✅ High throughput (**52,000+ TPS**)

#### How it works

1. Transaction is submitted
2. Executed instantly by a small node set (~10 ms)
3. Propagated across the network (DAG)
4. Finalized by Byzantine quorum (~300 ms)

Result: Instant user experience with strong consistency and verifiable state.

#### Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                     Application Layer                            │
│  (Move VM, Smart Contracts, Transactions)                        │
└────────────────┬─────────────────────────────────────────────────┘
                 │
┌────────────────▼─────────────────────────────────────────────────┐
│                   DAG Execution Layer                            │
│                                                                  │
│  ┌───────────────────────────────────────────────────────┐       │
│  │              DagEngine                                │       │
│  │  • produce_vertex()                                   │       │
│  │  • Parallel transaction execution                     │       │
│  │  • State management                                   │       │
│  └───────────────────┬───────────────────────────────────┘       │
└──────────────────────┼───────────────────────────────────────────┘
                       │
┌──────────────────────▼───────────────────────────────────────────┐
│                 Consensus Layer (Bullshark)                      │
│                                                                  │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐                  │
│  │  Ordering  │  │  Quorum    │  │  Leader    │                  │
│  │  Protocol  │  │  Check     │  │  Election  │                  │
│  │  (3 rounds)│  │  (2f+1)    │  │  (Round-   │                  │
│  │            │  │            │  │   Robin)   │                  │
│  └────────────┘  └────────────┘  └────────────┘                  │
│                                                                  │
│  ┌────────────────────────────────────────────────────┐          │
│  │         DagConsensus                               │          │
│  │  • create_vertex()                                 │          │
│  │  • add_vertex()                                    │          │
│  │  • try_commit() → Checkpoint                       │          │
│  └────────────────────┬───────────────────────────────┘          │
└───────────────────────┼──────────────────────────────────────────┘
                        │
┌───────────────────────▼──────────────────────────────────────────┐
│              Data Availability Layer                             │
│                                                                  │
│  ┌────────────────────────────────────────────────────┐          │
│  │              DagStore                              │          │
│  │  • Store vertices (HashMap<VertexId, DagVertex>)   │          │
│  │  • Index by round                                  │          │
│  │  • Index by authority                              │          │
│  │  • Maintain pending vertices queue                 │          │
│  └────────────────────────────────────────────────────┘          │
└──────────────────────────────────────────────────────────────────┘
```

#### Parallel Execution Model

```
┌───────────────────────────────────────────────────────────┐
│                    Transaction Pool                       │
│  [Tx1 Tx2 Tx3 Tx4 Tx5 Tx6 Tx7 Tx8 Tx9 Tx10]               │
└─────────┬─────────────────────────────────────────────────┘
          │
          │ Group by sender
          ▼
┌─────────────────────────────────────────────────────────────┐
│  Sender A: [Tx1, Tx4, Tx7]  (sequential execution)          │
│  Sender B: [Tx2, Tx5, Tx8]  (sequential execution)          │
│  Sender C: [Tx3, Tx6, Tx9]  (sequential execution)          │
│  Sender D: [Tx10]           (sequential execution)          │
└────┬──────────┬──────────┬───────────┬──────────────────────┘
     │          │          │           │
     │          │          │           │
┌────▼────┐ ┌───▼─────┐ ┌──▼──────┐ ┌──▼──────┐
│Worker 1 │ │Worker 2 │ │Worker 3 │ │Worker 4 │  (Parallel)
│Runtime 1│ │Runtime 2│ │Runtime 3│ │Runtime 4│
└────┬────┘ └───┬─────┘ └──┬──────┘ └──┬──────┘
     │          │          │           │
     └──────────┴──────────┴───────────┘
                │
                ▼
         ┌──────────────┐
         │  ChangeSet   │
         │  Aggregation │
         └──────┬───────┘
                │
                ▼
         ┌──────────────┐
         │ Apply to     │
         │ State        │
         └──────────────┘
```

#### Security Model: Byzantine Fault Tolerance

```
Total Authorities: n = 4
Maximum Faulty: f = (n-1)/3 = 1
Quorum Required: 2f+1 = 3

For commit to happen:
• Leader vertex needs ≥ 3 supporters
• Even if 1 authority is malicious, 3 honest
  authorities form quorum
```

#### SMT and Light Client Integration

Centauri uses the `smt` crate to provide cryptographic proofs for light clients:

- **State Proofs (SMT)**: The `state_root` in each checkpoint represents the root of a Sparse Merkle Tree. Light clients can verify account state using `StateProof`.
- **Transaction Proofs**: The `tx_root` is computed from actual transaction hashes using binary Merkle trees. Light clients verify transaction inclusion using `TransactionProof`.

#### Production Status

- **Security Audit**: ✅ All 22 critical vulnerabilities fixed and verified
- **Test Coverage**: ✅ 107/107 tests passing with comprehensive fuzz testing
- **Performance**: ✅ 52,000+ TPS with sub-300ms finality
- **Production Ready**: ✅ Ready for mainnet deployment

---

## Vision

Kanari is built for real-time interactive systems like games, where speed, usability, and reliability are critical.

It bridges the gap between traditional game backend systems and verifiable distributed infrastructure.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Copyright (c) KanariNetwork, Inc.  
SPDX-License-Identifier: Apache-2.0

---

## Need help?

- **Issues**: [GitHub Issues](https://github.com/kanari-network/kanari-sdk/issues)

- **Docs**: [docs.kanari.network](https://docs.kanarinetwork.site)
