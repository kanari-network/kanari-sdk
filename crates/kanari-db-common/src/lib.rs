// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use rocksdb::{DB, Options};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};

static OPEN_DATABASES: Lazy<Mutex<HashMap<PathBuf, Weak<DB>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn bytes_from_mb(value: u64) -> usize {
    usize::try_from(value.saturating_mul(1024 * 1024)).unwrap_or(usize::MAX)
}

/// Open one shared RocksDB instance per normalized path. Different paths may be
/// open in the same process (needed by snapshot import/verification), while
/// repeated opens of one path reuse the existing `Arc<DB>`.
pub fn open_or_get_db(path_opt: Option<PathBuf>) -> Result<Arc<DB>> {
    // Determine path
    let mut path = if let Some(p) = path_opt {
        p
    } else if let Ok(dir) = std::env::var("KANARI_DB") {
        let mut pb = PathBuf::from(dir);
        if pb.is_dir() {
            pb.push("kanari_db");
        }
        pb
    } else {
        let mut pb = if cfg!(miri) {
            PathBuf::from(".")
        } else {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
        };
        pb.push(".kanari");
        pb.push("kanari-db");
        std::fs::create_dir_all(&pb).context("Failed to create kanari-db directory")?;
        pb.push("kanari_db");
        pb
    };
    if path.is_relative() {
        path = std::env::current_dir()
            .context("Failed to resolve current directory for RocksDB")?
            .join(path);
    }

    {
        let mut registry = OPEN_DATABASES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        registry.retain(|_, db| db.strong_count() > 0);
        if let Some(db) = registry.get(&path).and_then(Weak::upgrade) {
            return Ok(db);
        }
    }

    std::fs::create_dir_all(path.parent().unwrap_or_else(|| std::path::Path::new(".")))
        .context("Failed to create RocksDB parent directory")?;

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);

    // Optimize for high throughput (100k+ TPS target)
    // 1. Increase parallelism for background flushes/compactions
    let parallelism = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4);
    opts.increase_parallelism(parallelism);
    opts.set_max_background_jobs(std::cmp::max(4, parallelism));

    // 2. Optimize BlockBasedTable (Block Cache & Bloom Filter)
    let mut block_opts = rocksdb::BlockBasedOptions::default();
    // Keep memory sizing operator-configurable for long-running validators.
    let block_cache_mb = env_u64("KANARI_DB_BLOCK_CACHE_MB", 512);
    block_opts.set_block_cache(&rocksdb::Cache::new_lru_cache(bytes_from_mb(
        block_cache_mb,
    )));
    // Bloom filter: 10 bits per key
    block_opts.set_bloom_filter(10.0, false);
    // Cache index and filter blocks in block cache to save memory/IO
    block_opts.set_cache_index_and_filter_blocks(true);
    block_opts.set_pin_l0_filter_and_index_blocks_in_cache(true);
    opts.set_block_based_table_factory(&block_opts);

    // 3. MemTable & Compaction Tuning
    // 64MB MemTable size
    let write_buffer_mb = env_u64("KANARI_DB_WRITE_BUFFER_MB", 64);
    opts.set_write_buffer_size(bytes_from_mb(write_buffer_mb));
    // Keep up to 4 memtables in memory before blocking
    opts.set_max_write_buffer_number(4);
    // Target file size for L1 (same as MemTable)
    opts.set_target_file_size_base(64 * 1024 * 1024);
    // Level multiplier (default 10)
    opts.set_max_bytes_for_level_base(256 * 1024 * 1024);

    // 4. Compression
    // LZ4 is good balance of speed/compression for bottom levels
    opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

    // 5. Bytes per sync (smoother IO)
    opts.set_bytes_per_sync(1024 * 1024); // 1MB
    opts.set_wal_bytes_per_sync(1024 * 1024);

    // Bound file descriptors, WAL growth, and old RocksDB logs. Periodic
    // compaction reclaims obsolete versions without deleting canonical state.
    let max_open_files = env_u64("KANARI_DB_MAX_OPEN_FILES", 4096).min(i32::MAX as u64) as i32;
    opts.set_max_open_files(max_open_files);
    opts.set_max_total_wal_size(env_u64("KANARI_DB_MAX_WAL_MB", 1024).saturating_mul(1024 * 1024));
    opts.set_keep_log_file_num(
        usize::try_from(env_u64("KANARI_DB_KEEP_LOG_FILES", 10)).unwrap_or(usize::MAX),
    );
    opts.set_periodic_compaction_seconds(env_u64(
        "KANARI_DB_PERIODIC_COMPACTION_SECS",
        7 * 24 * 60 * 60,
    ));

    // Serialize the final check/open step so two startup threads cannot race to
    // open RocksDB twice for the same directory.
    let mut registry = OPEN_DATABASES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(db) = registry.get(&path).and_then(Weak::upgrade) {
        return Ok(db);
    }
    let db = DB::open(&opts, &path).context("Failed to open RocksDB for kanari")?;
    let arc = Arc::new(db);
    registry.insert(path, Arc::downgrade(&arc));
    Ok(arc)
}
