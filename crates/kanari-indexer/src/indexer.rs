// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Main indexer implementation

use crate::db::IndexerDB;
use crate::models::IndexedCoin;
use anyhow::{Context, Result};
use chrono::Utc;
use kanari_types::transaction::NativeCall;
use kanari_types::{block::Block, kanari::KANARI_TOKEN_TYPE};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

/// Configuration for the indexer
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    /// Path to the SQLite database file
    pub db_path: PathBuf,
    /// Whether to use in-memory database (for testing)
    pub in_memory: bool,
    /// Batch size for indexing blocks
    pub batch_size: u32,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("kanari_indexer.db"),
            in_memory: false,
            batch_size: 100,
        }
    }
}

/// Main indexer that processes and indexes blockchain data
pub struct Indexer {
    config: IndexerConfig,
    db: IndexerDB,
}

impl Indexer {
    /// Create a new indexer with the given configuration
    pub fn new(config: IndexerConfig) -> Result<Self> {
        let db = if config.in_memory {
            IndexerDB::new_in_memory()?
        } else {
            // Ensure parent directory exists
            if let Some(parent) = config.db_path.parent() {
                std::fs::create_dir_all(parent).context("Failed to create database directory")?;
            }

            IndexerDB::new(&config.db_path)?
        };

        info!("Indexer initialized with config: {:?}", config);

        Ok(Self { config, db })
    }

    /// Get a reference to the database
    pub fn db(&self) -> &IndexerDB {
        &self.db
    }

    /// Index a single block and all its transactions and events
    pub fn index_block(&self, block: &Block) -> Result<()> {
        let height = block.header.height;
        debug!("Indexing block at height {}", height);

        // Insert block
        self.db
            .insert_block(block)
            .context(format!("Failed to insert block at height {}", height))?;

        // Insert transactions
        if !block.transactions.is_empty() {
            self.db
                .insert_transactions(height, &block.transactions)
                .context(format!(
                    "Failed to insert transactions for block {}",
                    height
                ))?;

            // Index coins from transactions
            for signed_tx in &block.transactions {
                let tx_hash = hex::encode(signed_tx.hash());

                // Extract coin information from transaction payload
                if let Some(NativeCall::Transfer { recipient, amount, .. }) =
                    signed_tx.transaction.native_call()
                {
                    // Create coin record for transfer
                    let coin = IndexedCoin {
                        id: format!("{}-{}", tx_hash, 0),
                        owner: recipient,
                        coin_type: KANARI_TOKEN_TYPE.to_string(),
                        balance: amount,
                        is_frozen: false,
                        created_tx_hash: Some(tx_hash.clone()),
                        last_updated_tx_hash: Some(tx_hash.clone()),
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    };

                    self.db
                        .upsert_coin(&coin)
                        .context(format!("Failed to insert coin from tx {}", tx_hash))?;
                }

                if !block.events.is_empty() {
                    self.db
                        .insert_events(height, &[(tx_hash.as_str(), block.events.as_slice())])
                        .context(format!("Failed to insert events for block {}", height))?;
                }
            }
        }

        // Update last indexed height
        self.db
            .update_last_indexed_height(height)
            .context("Failed to update last indexed height")?;

        debug!("Successfully indexed block at height {}", height);

        Ok(())
    }

    /// Index multiple blocks in batch
    pub fn index_blocks_batch(&self, blocks: &[Block]) -> Result<()> {
        let count = blocks.len();
        info!("Indexing batch of {} blocks", count);

        for block in blocks {
            self.index_block(block)?;
        }

        info!("Successfully indexed batch of {} blocks", count);

        Ok(())
    }

    /// Sync from current position to latest block
    pub fn sync_to_latest<F>(&self, get_block_fn: F) -> Result<u64>
    where
        F: Fn(u64) -> Result<Option<Block>>,
    {
        let current_height = self.db.get_last_indexed_height()?;
        info!("Starting sync from height {}", current_height);

        let mut synced_count = 0u64;
        let mut height = current_height + 1;

        loop {
            match get_block_fn(height) {
                Ok(Some(block)) => {
                    self.index_block(&block)?;
                    synced_count += 1;
                    height += 1;

                    if synced_count.is_multiple_of(self.config.batch_size as u64) {
                        info!("Synced {} blocks so far...", synced_count);
                    }
                }
                Ok(None) => {
                    // No more blocks to sync
                    break;
                }
                Err(e) => {
                    error!("Error fetching block at height {}: {}", height, e);
                    return Err(e).context(format!("Failed to fetch block at height {}", height));
                }
            }
        }

        info!("Sync completed. Total blocks synced: {}", synced_count);

        Ok(synced_count)
    }

    /// Reindex from a specific height (useful for recovery or reprocessing)
    pub fn reindex_from_height<F>(&self, from_height: u64, get_block_fn: F) -> Result<u64>
    where
        F: Fn(u64) -> Result<Option<Block>>,
    {
        warn!("Reindexing from height {}", from_height);

        // Clear data from this height onwards
        // Note: This is a simplified approach - in production you might want more sophisticated cleanup

        // Set the last indexed height to one before the reindex point
        if from_height > 0 {
            self.db.update_last_indexed_height(from_height - 1)?;
        } else {
            self.db.update_last_indexed_height(0)?;
        }

        // Now sync from this height
        self.sync_to_latest(get_block_fn)
    }

    /// Get indexer statistics
    pub fn get_statistics(&self) -> Result<IndexerStatistics> {
        let block_count = self.db.get_block_count()?;
        let transaction_count = self.db.get_transaction_count()?;
        let event_count = self.db.get_event_count()?;
        let last_indexed_height = self.db.get_last_indexed_height()?;
        let transaction_stats = self.db.get_transaction_stats()?;

        Ok(IndexerStatistics {
            block_count,
            transaction_count,
            event_count,
            last_indexed_height,
            transaction_stats,
        })
    }
}

/// Statistics about the indexer state
#[derive(Debug, Clone)]
pub struct IndexerStatistics {
    pub block_count: u64,
    pub transaction_count: u64,
    pub event_count: u64,
    pub last_indexed_height: u64,
    pub transaction_stats: Vec<(String, u64)>,
}

impl std::fmt::Display for IndexerStatistics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Indexer Statistics ===")?;
        writeln!(f, "Blocks indexed: {}", self.block_count)?;
        writeln!(f, "Transactions indexed: {}", self.transaction_count)?;
        writeln!(f, "Events indexed: {}", self.event_count)?;
        writeln!(f, "Last indexed height: {}", self.last_indexed_height)?;
        writeln!(f, "\nTransaction types:")?;
        for (tx_type, count) in &self.transaction_stats {
            writeln!(f, "  {}: {}", tx_type, count)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_indexer_in_memory() {
        let config = IndexerConfig {
            in_memory: true,
            ..Default::default()
        };

        let indexer = Indexer::new(config).unwrap();
        assert_eq!(indexer.db().get_latest_height().unwrap(), 0);
    }

    #[test]
    fn test_index_statistics() {
        let config = IndexerConfig {
            in_memory: true,
            ..Default::default()
        };

        let indexer = Indexer::new(config).unwrap();
        let stats = indexer.get_statistics().unwrap();

        assert_eq!(stats.block_count, 0);
        assert_eq!(stats.transaction_count, 0);
        assert_eq!(stats.event_count, 0);
        assert_eq!(stats.last_indexed_height, 0);
    }
}
