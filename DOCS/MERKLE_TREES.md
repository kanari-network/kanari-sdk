# Merkle Tree Implementation

## Overview

Kanari blockchain uses **two types of Merkle trees** for different purposes:

### 1. Sparse Merkle Tree (SMT) - State Storage

- **Location**: `crates/smt/`
- **Purpose**: Account state verification and proofs
- **Used for**:
  - Account balances
  - Module storage
  - Object ownership
  - State root computation
- **Implementation**: Full SMT with 256-bit keyspace
- **Storage**: Persistent in RocksDB

### 2. Transaction Merkle Tree - Block Verification

- **Location**: `crates/smt/src/sparse_merkle.rs`
- **Purpose**: Light client transaction verification
- **Used for**:
  - Block header merkle root
  - Transaction inclusion proofs
  - Light client sync
- **Implementation**: Binary merkle tree (like Bitcoin/Ethereum)
- **Storage**: In-memory, reconstructed per block

## Why Two Different Trees?

| Feature              | SMT (State)                 | Binary Tree (Transactions)|
|----------------------|-----------------------------|---------------------------|
| **Persistence**      | Persisted in DB             | Computed on-demand        |
| **Proof Type**       | Membership + Non-membership | Inclusion only            |
| **Size**             | Sparse (256-bit keyspace)   | Dense (transaction count) |
| **Update Frequency** | Per transaction execution   | Per block production      |
| **Primary Use**      | State verification          | Light client sync         |


## References

- Bitcoin merkle trees: [Bitcoin Wiki](https://en.bitcoin.it/wiki/Protocol_documentation#Merkle_Trees)
- Ethereum merkle trees: [Ethereum Yellow Paper](https://ethereum.github.io/yellowpaper/paper.pdf)
- Sparse Merkle Trees: [Jellyfish Merkle Tree](https://developers.diem.com/papers/jellyfish-merkle-tree/2021-01-14.pdf)
- Blake3: [BLAKE3 Specification](https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf)
