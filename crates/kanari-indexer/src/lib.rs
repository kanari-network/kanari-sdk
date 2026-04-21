// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Kanari Blockchain Indexer using SQLite
//!
//! This module provides a comprehensive indexing solution for the Kanari blockchain,
//! storing transactions, blocks, events, coins, and account balances in a SQLite database
//! for efficient querying and analysis.

mod db;
mod indexer;
mod models;
mod schema;

pub use db::IndexerDB;
pub use indexer::{Indexer, IndexerConfig};
pub use models::*;
