// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Shared RocksDB instance for Move VM state storage
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

/// Delegate to the centralized DB opener implemented in `kanari-db-common`.
pub fn get_or_open_db(path_opt: Option<PathBuf>) -> Result<Arc<rocksdb::DB>> {
    kanari_db_common::open_or_get_db(path_opt)
}
