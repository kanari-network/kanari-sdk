// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Minimal merkle map implementation used as a Sparse Merkle Tree substitute
//! for runtime state. This is intentionally pragmatic: keys are hashed and the
//! Merkle root is computed over sorted key-value hashes. It supports batched
//! writes persisted to RocksDB for high-throughput commits.
//!
//! IMPORTANT LIMITATIONS (by design)
//! - Non-membership proofs are NOT supported. This is NOT a full Sparse Merkle
//!   Tree with implicit zero/default leaves; it only produces membership proofs
//!   that show "key K has value V". Systems that require proofs of absence
//!   must use a different structure or maintain additional bookkeeping.
//! - No versioning / snapshots: this SMT exposes only the latest state.
//!   Historical roots must be managed externally (e.g. by keeping multiple
//!   DB instances or taking RocksDB snapshots at the application layer).
//!
//! These constraints simplify the implementation and are acceptable for use
//! cases that only need compact membership proofs for the current state.

pub use kanari_db_common::open_or_get_db;

mod hash;
mod sparse_merkle;
pub use sparse_merkle::SparseMerkleTree;
pub use sparse_merkle::default_hashes;

pub use hash::*;
mod merkle;
pub use merkle::{
    CompressedMerkleProof, batch_verify_merkle_proofs, compute_merkle_root,
    generate_merkle_multiproof, generate_merkle_proof, verify_merkle_proof,
};
