// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use rocksdb::{DB, IteratorMode, Options, WriteBatch};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};

static OPEN_DATABASES: Lazy<Mutex<HashMap<PathBuf, Weak<DB>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Upper bound for state column families. All of them are created up front so
/// changing the configured count on a later start cannot hide existing data.
pub const MAX_STATE_CF_SHARDS: usize = 64;
pub const DEFAULT_STATE_CF_SHARDS: usize = 8;

/// Persisted layout marker. It lives in the default CF so it is visible before
/// any state CF routing is chosen, and is excluded from logical-state scans.
pub const STATE_CF_SHARD_COUNT_KEY: &[u8] = b"runtime:state_cf_shard_count_v1";

pub fn state_cf_name(index: usize) -> String {
    format!("state_{index}")
}

fn is_store_internal_key(key: &[u8]) -> bool {
    // `meta:` holds SMT maintenance keys such as `meta:node_index_version`.
    // They belong to the SMT's default-CF namespace just like `n:`/`d:`.
    key == STATE_CF_SHARD_COUNT_KEY
        || key.starts_with(b"n:")
        || key.starts_with(b"d:")
        || key.starts_with(b"meta:")
}

/// Deterministic FNV-1a routing. Unlike a randomly seeded hasher this keeps the
/// same key in the same column family across process restarts.
pub fn shard_index_for_key(key: &[u8], shard_count: usize) -> usize {
    let count = shard_count.max(1);
    let mut hash = 0xcbf29ce484222325u64;
    for byte in key {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    (hash as usize) % count
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|value| value.parse().ok()).unwrap_or(default)
}

fn clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    let (min, max) = if min <= max { (min, max) } else { (max, min) };
    value.clamp(min, max)
}

fn env_i32_clamped(name: &str, default: i32, min: i32, max: i32) -> i32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .map(|value| clamp_i32(value, min, max))
        .unwrap_or_else(|| clamp_i32(default, min, max))
}

fn bytes_from_mb(value: u64) -> usize {
    usize::try_from(bytes_from_mb_u64(value)).unwrap_or(usize::MAX)
}

fn bytes_from_mb_u64(value: u64) -> u64 {
    value.saturating_mul(1024 * 1024)
}

fn resolve_db_path(path_opt: Option<PathBuf>) -> Result<PathBuf> {
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
    Ok(path)
}

fn requested_shard_count() -> usize {
    env_i32_clamped(
        "KANARI_STATE_CF_SHARDS",
        DEFAULT_STATE_CF_SHARDS as i32,
        1,
        MAX_STATE_CF_SHARDS as i32,
    ) as usize
}

fn read_or_init_shard_count(db: &DB, requested: usize) -> Result<usize> {
    let default_cf = db.cf_handle("default").context("Missing default RocksDB column family")?;
    if let Some(bytes) = db.get_cf(default_cf, STATE_CF_SHARD_COUNT_KEY)? {
        anyhow::ensure!(bytes.len() == 4, "Invalid persisted state shard count marker");
        let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        anyhow::ensure!(
            (1..=MAX_STATE_CF_SHARDS).contains(&count),
            "Persisted state shard count {count} is out of range"
        );
        return Ok(count);
    }
    db.put_cf(default_cf, STATE_CF_SHARD_COUNT_KEY, (requested as u32).to_le_bytes())?;
    Ok(requested)
}

/// Move logical keys written by the former single-CF layout into their routed
/// column families. The whole migration and the layout marker share one batch,
/// so a crash cannot expose a half-migrated database to canonical root checks.
fn initialize_sharded_layout(db: &DB, requested: usize) -> Result<usize> {
    let default_cf = db.cf_handle("default").context("Missing default RocksDB column family")?;
    if db.get_cf(default_cf, STATE_CF_SHARD_COUNT_KEY)?.is_some() {
        return read_or_init_shard_count(db, requested);
    }

    let mut batch = WriteBatch::default();
    for item in db.iterator_cf(default_cf, IteratorMode::Start) {
        let (key, value) = item?;
        if is_store_internal_key(&key) {
            continue;
        }
        let index = shard_index_for_key(&key, requested);
        let target_name = state_cf_name(index);
        let target_cf = db
            .cf_handle(&target_name)
            .with_context(|| format!("Missing RocksDB column family {target_name}"))?;
        batch.delete_cf(default_cf, &key);
        batch.put_cf(target_cf, &key, &value);
    }
    batch.put_cf(default_cf, STATE_CF_SHARD_COUNT_KEY, (requested as u32).to_le_bytes());
    db.write(batch)?;
    Ok(requested)
}

/// Open one shared RocksDB instance per normalized path. Different paths may be
/// open in the same process (needed by snapshot import/verification), while
/// repeated opens of one path reuse the existing `Arc<DB>`.
pub fn open_or_get_db(path_opt: Option<PathBuf>) -> Result<Arc<DB>> {
    let (db, _) = open_or_get_db_with_layout(path_opt)?;
    Ok(db)
}

/// Same shared handle as [`open_or_get_db`], plus the persisted number of
/// state column families used for key routing.
pub fn open_or_get_db_with_layout(path_opt: Option<PathBuf>) -> Result<(Arc<DB>, usize)> {
    let path = resolve_db_path(path_opt)?;
    let requested = requested_shard_count();

    let mut registry = OPEN_DATABASES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    registry.retain(|_, db| db.strong_count() > 0);
    if let Some(db) = registry.get(&path).and_then(Weak::upgrade) {
        // A cached handle may be observed while the first opener is still
        // migrating. Use the atomic migrate-or-read path so this caller cannot
        // persist the layout marker before the legacy keys have been moved.
        let count = initialize_sharded_layout(&db, requested)?;
        return Ok((db, count));
    }

    std::fs::create_dir_all(path.parent().unwrap_or_else(|| std::path::Path::new(".")))
        .context("Failed to create RocksDB parent directory")?;

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    opts.enable_statistics();

    // Optimize for high throughput (100k+ TPS target)
    // 1. Increase parallelism for background flushes/compactions
    let parallelism = std::thread::available_parallelism().map(|n| n.get() as i32).unwrap_or(4);
    opts.increase_parallelism(parallelism);
    opts.set_max_background_jobs(std::cmp::max(4, parallelism));

    // 2. Optimize BlockBasedTable (Block Cache & Bloom Filter)
    let mut block_opts = rocksdb::BlockBasedOptions::default();
    // Keep memory sizing operator-configurable for long-running validators.
    let block_cache_mb = env_u64("KANARI_DB_BLOCK_CACHE_MB", 512);
    block_opts.set_block_cache(&rocksdb::Cache::new_lru_cache(bytes_from_mb(block_cache_mb)));
    // Bloom filter: 10 bits per key
    block_opts.set_bloom_filter(10.0, false);
    // Cache index and filter blocks in block cache to save memory/IO
    block_opts.set_cache_index_and_filter_blocks(true);
    block_opts.set_pin_l0_filter_and_index_blocks_in_cache(true);
    opts.set_block_based_table_factory(&block_opts);

    // 3. MemTable & Compaction Tuning
    // Larger memtables and wider L0 thresholds keep large checkpoint commits
    // from bouncing between foreground writes and background compaction.
    let write_buffer_mb = env_u64("KANARI_DB_WRITE_BUFFER_MB", 256);
    opts.set_write_buffer_size(bytes_from_mb(write_buffer_mb));
    let max_write_buffers = env_i32_clamped("KANARI_DB_MAX_WRITE_BUFFERS", 8, 1, 64);
    let min_write_buffers_to_merge =
        env_i32_clamped("KANARI_DB_MIN_WRITE_BUFFERS_TO_MERGE", 2, 1, max_write_buffers);
    let l0_compaction_trigger = env_i32_clamped("KANARI_DB_L0_COMPACTION_TRIGGER", 8, 1, 1024);
    let l0_slowdown_trigger =
        env_i32_clamped("KANARI_DB_L0_SLOWDOWN_TRIGGER", 32, l0_compaction_trigger, 4096);
    let l0_stop_trigger =
        env_i32_clamped("KANARI_DB_L0_STOP_TRIGGER", 64, l0_slowdown_trigger, 8192);
    opts.set_max_write_buffer_number(max_write_buffers);
    opts.set_min_write_buffer_number_to_merge(min_write_buffers_to_merge);
    opts.set_level_zero_file_num_compaction_trigger(l0_compaction_trigger);
    opts.set_level_zero_slowdown_writes_trigger(l0_slowdown_trigger);
    opts.set_level_zero_stop_writes_trigger(l0_stop_trigger);
    opts.set_target_file_size_base(bytes_from_mb_u64(env_u64("KANARI_DB_TARGET_FILE_MB", 256)));
    opts.set_max_bytes_for_level_base(bytes_from_mb_u64(env_u64("KANARI_DB_LEVEL_BASE_MB", 1024)));

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

    // Create the full CF set once. Routing may be reconfigured later, but the
    // data is never hidden behind a missing column family.
    let cf_names: Vec<String> = std::iter::once("default".to_string())
        .chain((0..MAX_STATE_CF_SHARDS).map(state_cf_name))
        .collect();
    let db = DB::open_cf(&opts, &path, &cf_names).map_err(|error| {
        anyhow::anyhow!("Failed to open RocksDB for kanari at {}: {error:?}", path.display())
    })?;
    let arc = Arc::new(db);
    registry.insert(path, Arc::downgrade(&arc));
    let count = initialize_sharded_layout(&arc, requested)?;
    Ok((arc, count))
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_STATE_CF_SHARDS, bytes_from_mb_u64, clamp_i32, env_i32_clamped, shard_index_for_key,
        state_cf_name,
    };

    #[test]
    fn clamp_i32_keeps_rocksdb_tuning_in_safe_range() {
        assert_eq!(clamp_i32(-10, 1, 64), 1);
        assert_eq!(clamp_i32(8, 1, 64), 8);
        assert_eq!(clamp_i32(10_000, 1, 64), 64);
        assert_eq!(clamp_i32(10, 64, 1), 10);
        assert_eq!(clamp_i32(100, 64, 1), 64);
    }

    #[test]
    fn env_i32_clamped_clamps_missing_or_invalid_defaults() {
        let missing_name = "KANARI_TEST_DB_CLAMP_MISSING_DEFAULT";
        let invalid_name = "KANARI_TEST_DB_CLAMP_INVALID_DEFAULT";
        unsafe {
            std::env::remove_var(missing_name);
            std::env::set_var(invalid_name, "not-an-int");
        }

        assert_eq!(env_i32_clamped(missing_name, 64, 1, 8), 8);
        assert_eq!(env_i32_clamped(invalid_name, 0, 1, 8), 1);

        unsafe {
            std::env::remove_var(invalid_name);
        }
    }

    #[test]
    fn bytes_from_mb_u64_saturates_on_overflow() {
        assert_eq!(bytes_from_mb_u64(256), 256 * 1024 * 1024);
        assert_eq!(bytes_from_mb_u64(u64::MAX), u64::MAX);
    }

    #[test]
    fn shard_index_for_key_is_stable_and_bounded() {
        let key = b"object:0xabc";
        let first = shard_index_for_key(key, 8);
        assert_eq!(first, shard_index_for_key(key, 8));
        assert!(first < 8);
        assert!(shard_index_for_key(b"", 1) < 1);
        assert!(shard_index_for_key(key, MAX_STATE_CF_SHARDS) < MAX_STATE_CF_SHARDS);
        assert_eq!(state_cf_name(first), format!("state_{first}"));
    }
}
