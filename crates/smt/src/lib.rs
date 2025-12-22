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

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use rocksdb::{DB, Options};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

static GLOBAL_DBS: Lazy<Mutex<HashMap<String, Arc<DB>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn open_or_get_db(path_opt: Option<PathBuf>) -> Result<Arc<DB>> {
    // Determine path
    let path = if let Some(p) = path_opt {
        p
    } else if let Ok(dir) = std::env::var("KANARI_DB") {
        let mut pb = PathBuf::from(dir);
        if pb.is_dir() {
            pb.push("kanari_db");
        }
        pb
    } else {
        let mut pb = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        pb.push(".kanari");
        pb.push("kanari-db");
        std::fs::create_dir_all(&pb).context("Failed to create kanari-db directory")?;
        pb.push("smt_db");
        pb
    };

    std::fs::create_dir_all(path.parent().unwrap_or_else(|| std::path::Path::new(".")))
        .context("Failed to create RocksDB parent directory")?;
    let path_str = path.to_string_lossy().to_string();

    // Serialize DB opens for the same path to avoid concurrent open attempts
    // which can result in RocksDB lock file errors when tests run in
    // parallel. Hold the GLOBAL_DBS mutex while opening and inserting.
    let mut map = GLOBAL_DBS.lock().unwrap();
    if let Some(db) = map.get(&path_str) {
        return Ok(db.clone());
    }

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, &path).context("Failed to open RocksDB for SMT")?;
    let arc = Arc::new(db);

    map.insert(path_str, arc.clone());
    Ok(arc)
}

mod hash;
mod sparse_merkle;
pub use sparse_merkle::SparseMerkleTree;
pub use sparse_merkle::default_hashes;

pub use hash::*;
