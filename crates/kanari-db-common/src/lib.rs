// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use rocksdb::{DB, Options};
use std::path::PathBuf;
use std::sync::Arc;

static GLOBAL_DB: OnceCell<Arc<DB>> = OnceCell::new();
static GLOBAL_DB_PATH: OnceCell<PathBuf> = OnceCell::new();

/// Open (once) a RocksDB instance at the given path (or default) and return `Arc<DB>`.
/// Subsequent calls will return the same Arc. If a different path is provided after
/// the DB was opened, an error is returned to avoid multiple opens to different paths.
pub fn open_or_get_db(path_opt: Option<PathBuf>) -> Result<Arc<DB>> {
    if let Some(db) = GLOBAL_DB.get() {
        // Already opened. If a path was provided, ensure it matches the existing one.
        if let (Some(p), Some(existing)) = (path_opt.as_ref(), GLOBAL_DB_PATH.get())
            && existing != p
        {
            anyhow::bail!("RocksDB already opened with a different path");
        }
        return Ok(db.clone());
    }

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
    // 512MB Block Cache
    block_opts.set_block_cache(&rocksdb::Cache::new_lru_cache(512 * 1024 * 1024));
    // Bloom filter: 10 bits per key
    block_opts.set_bloom_filter(10.0, false);
    // Cache index and filter blocks in block cache to save memory/IO
    block_opts.set_cache_index_and_filter_blocks(true);
    block_opts.set_pin_l0_filter_and_index_blocks_in_cache(true);
    opts.set_block_based_table_factory(&block_opts);

    // 3. MemTable & Compaction Tuning
    // 64MB MemTable size
    opts.set_write_buffer_size(64 * 1024 * 1024);
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

    let db = DB::open(&opts, &path).context("Failed to open RocksDB for kanari")?;

    GLOBAL_DB_PATH.set(path.clone()).ok();
    let arc = Arc::new(db);
    GLOBAL_DB
        .set(arc.clone())
        .map_err(|_| anyhow::anyhow!("Failed to set global RocksDB"))?;
    Ok(arc)
}
