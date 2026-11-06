// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Main indexer implementation

use crate::db::IndexerDB;
use crate::models::{IndexedCoin, IndexedKrc721Event, IndexedKrc1155Event, Krc1155Action};
use anyhow::{Context, Result};
use chrono::Utc;
use kanari_types::address::Address as KanariAddress;
use kanari_types::event::Event;
use kanari_types::gas_coin::GasModule;
use kanari_types::transaction::Transaction;
use kanari_types::{block::Block, gas_coin::GAS_COIN};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

/// Type tag suffix for KRC-1155 events
const KRC_1155_MODULE: &str = "::kanari_system::krc_1155::";

/// Type tag suffix for KRC-721 events
const KRC_721_MODULE: &str = "::kanari_system::krc_721::";

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
    fn extract_native_transfer(transaction: &Transaction) -> Option<(String, u64)> {
        let Transaction::ExecuteFunction {
            module,
            function,
            args,
            ..
        } = transaction
        else {
            return None;
        };

        if module.as_str() != GasModule::module_path()
            || function != GasModule::function_names().transfer
            || args.len() < 3
        {
            return None;
        }

        let amount = bcs::from_bytes::<u64>(&args[1]).ok()?;
        let recipient = KanariAddress::try_from(args[2].as_slice())
            .ok()?
            .to_hex_literal();
        Some((recipient, amount))
    }

    /// Parse an `ObjectLockedSupplyChanged<CoinType>` Move event from raw BCS bytes.
    ///
    /// Move struct layout:
    ///   object_id: address   (32 bytes)
    ///   amount:    u64       (8 bytes LE)
    ///   action:    u8        (1 byte)   1=locked, 2=released, 3=refunded
    /// Parse a KRC-1155 event from raw BCS bytes.
    ///
    /// Move struct layout for Minted/Burned/Transferred:
    ///   token_type: address (32 bytes)
    ///   amount:    u64       (8 bytes LE)
    ///   action:    u8        (1 byte)   1=minted, 2=burned, 3=transferred
    ///   sender:    address   (32 bytes)  // only for Transferred
    ///   recipient: address   (32 bytes)  // for Minted/Transferred
    ///   owner:     address   (32 bytes)  // for Burned
    ///   timestamp: u64       (8 bytes LE)
    /// Total: varies by action (81-145 bytes)
    fn parse_krc_1155_event(
        event_data: &[u8],
        tx_hash: Option<&str>,
        block_height: u64,
    ) -> Option<IndexedKrc1155Event> {
        if event_data.len() < 81 {
            return None;
        }

        let token_type = KanariAddress::try_from(&event_data[0..32])
            .ok()?
            .to_hex_literal();
        let amount = u64::from_le_bytes(event_data[32..40].try_into().ok()?);
        let action_byte = event_data[40];

        let action = match action_byte {
            1 => Krc1155Action::Minted,
            2 => Krc1155Action::Burned,
            3 => Krc1155Action::Transferred,
            _ => Krc1155Action::Unknown,
        };

        // Parse based on action type
        let (sender, recipient, owner) = match action_byte {
            1 => {
                // Minted: recipient only (no sender, owner)
                let recipient = KanariAddress::try_from(&event_data[41..73])
                    .ok()?
                    .to_hex_literal();
                (None, Some(recipient), None)
            }
            2 => {
                // Burned: owner only
                let owner = KanariAddress::try_from(&event_data[41..73])
                    .ok()?
                    .to_hex_literal();
                (None, None, Some(owner))
            }
            3 => {
                // Transferred: sender + recipient
                let sender = KanariAddress::try_from(&event_data[41..73])
                    .ok()?
                    .to_hex_literal();
                let recipient = KanariAddress::try_from(&event_data[73..105])
                    .ok()?
                    .to_hex_literal();
                (Some(sender), Some(recipient), None)
            }
            _ => (None, None, None),
        };

        // Timestamp is at different offsets based on action
        let (timestamp_offset, has_timestamp) = match action_byte {
            1 => (73, true), // Minted: token_id(32) + amount(8) + action(1) + recipient(32) = 73
            2 => (73, true), // Burned: token_id(32) + amount(8) + action(1) + owner(32) = 73
            3 => (105, true), // Transferred: token_id(32) + amount(8) + action(1) + sender(32) + recipient(32) = 105
            _ => (41, false), // Unknown: only token_type(32) + amount(8) + action(1) = 41 bytes known
        };

        if event_data.len() < timestamp_offset {
            return None;
        }

        let timestamp = if has_timestamp {
            if event_data.len() < timestamp_offset + 8 {
                return None;
            }
            u64::from_le_bytes(
                event_data[timestamp_offset..timestamp_offset + 8]
                    .try_into()
                    .ok()?,
            )
        } else {
            0
        };

        Some(IndexedKrc1155Event {
            id: 0,
            tx_hash: tx_hash.map(|s| s.to_string()),
            block_height,
            token_type,
            action: action.clone(),
            amount,
            sender,
            recipient,
            owner,
            timestamp,
            created_at: Utc::now(),
        })
    }

    /// Parse a KRC-721 MintLog event from raw BCS bytes.
    ///
    /// Move struct layout for MintLog:
    ///   object_id: address   (32 bytes)
    ///   creator:   address   (32 bytes)
    ///   collection_id: address (32 bytes)
    /// Total: 96 bytes
    fn parse_krc_721_mint_event(
        event_data: &[u8],
        tx_hash: Option<&str>,
        block_height: u64,
    ) -> Option<IndexedKrc721Event> {
        if event_data.len() < 96 {
            return None;
        }

        let object_id = KanariAddress::try_from(&event_data[0..32])
            .ok()?
            .to_hex_literal();
        let creator = KanariAddress::try_from(&event_data[32..64])
            .ok()?
            .to_hex_literal();
        let collection_id = KanariAddress::try_from(&event_data[64..96])
            .ok()?
            .to_hex_literal();

        Some(IndexedKrc721Event {
            id: 0,
            tx_hash: tx_hash.map(|s| s.to_string()),
            block_height,
            object_id,
            creator,
            collection_id,
            timestamp: 0, // KRC-721 MintLog doesn't have timestamp in event data
            created_at: Utc::now(),
        })
    }

    /// Detect and index KRC-1155 events from a block's event list.
    fn index_krc_1155_events(
        &self,
        block_height: u64,
        tx_hash: Option<&str>,
        events: &[Event],
    ) -> Result<usize> {
        let mut count = 0u64;
        for event in events {
            if !event.type_tag.contains(KRC_1155_MODULE) {
                continue;
            }
            if let Some(parsed) =
                Self::parse_krc_1155_event(&event.event_data, tx_hash, block_height)
            {
                self.db.upsert_krc_1155_event(&parsed).context(format!(
                    "Failed to insert KRC-1155 event for tx {:?}",
                    tx_hash
                ))?;
                count += 1;
            }
        }
        Ok(count as usize)
    }

    /// Detect and index KRC-721 events from a block's event list.
    fn index_krc_721_events(
        &self,
        block_height: u64,
        tx_hash: Option<&str>,
        events: &[Event],
    ) -> Result<usize> {
        let mut count = 0u64;
        for event in events {
            if !event.type_tag.contains(KRC_721_MODULE) {
                continue;
            }
            if let Some(parsed) =
                Self::parse_krc_721_mint_event(&event.event_data, tx_hash, block_height)
            {
                self.db.upsert_krc_721_event(&parsed).context(format!(
                    "Failed to insert KRC-721 event for tx {:?}",
                    tx_hash
                ))?;
                count += 1;
            }
        }
        Ok(count as usize)
    }

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
                if let Some((recipient, amount)) =
                    Self::extract_native_transfer(&signed_tx.transaction)
                {
                    // Create coin record for transfer
                    let coin = IndexedCoin {
                        id: format!("{}-{}", tx_hash, 0),
                        owner: recipient,
                        coin_type: GAS_COIN.to_string(),
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
            }
        }

        // Events are block-level data and must be indexed even when a block
        // has no transaction rows (for example, synthetic/replayed blocks).
        // Keeping this outside the transaction branch also guarantees KRC events
        // are not silently dropped.
        if !block.events.is_empty() {
            self.db
                .insert_events(height, &[(None, block.events.as_slice())])
                .context(format!("Failed to insert events for block {}", height))?;

            // Also index structured KRC events
            let tx_hash = None;
            self.index_krc_1155_events(height, tx_hash, &block.events)?;
            self.index_krc_721_events(height, tx_hash, &block.events)?;
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
        warn!(
            "Reindex rebuilds the index from genesis to avoid stale derived rows after schema or logic changes"
        );
        self.db.clear_all_indexed_data()?;
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

    #[test]
    fn test_parse_krc_1155_event() {
        // Test Minted (action=1): token_type(32) + amount(8) + action(1) + recipient(32) + timestamp(8) = 81 bytes
        let mut minted_data = Vec::new();
        minted_data.extend(std::iter::repeat(0x11u8).take(32)); // token_type
        minted_data.extend_from_slice(&100u64.to_le_bytes()); // amount
        minted_data.push(1u8); // action = Minted
        minted_data.extend(std::iter::repeat(0x22u8).take(32)); // recipient
        minted_data.extend_from_slice(&1000u64.to_le_bytes()); // timestamp
        assert_eq!(minted_data.len(), 81);

        let parsed = Indexer::parse_krc_1155_event(&minted_data, Some("0xtx1"), 42).unwrap();
        assert_eq!(
            parsed.token_type,
            "0x1111111111111111111111111111111111111111111111111111111111111111"
        );
        assert_eq!(parsed.amount, 100);
        assert_eq!(parsed.action, Krc1155Action::Minted);
        assert_eq!(
            parsed.recipient,
            Some("0x2222222222222222222222222222222222222222222222222222222222222222".to_string())
        );
        assert_eq!(parsed.timestamp, 1000);
        assert_eq!(parsed.block_height, 42);
        assert_eq!(parsed.tx_hash, Some("0xtx1".to_string()));

        // Test action = 2 (Burned): token_type(32) + amount(8) + action(1) + owner(32) + timestamp(8) = 81 bytes
        let mut burned_data = Vec::new();
        burned_data.extend(std::iter::repeat(0x11u8).take(32)); // token_type
        burned_data.extend_from_slice(&50u64.to_le_bytes()); // amount
        burned_data.push(2u8); // action = Burned
        burned_data.extend(std::iter::repeat(0x33u8).take(32)); // owner
        burned_data.extend_from_slice(&2000u64.to_le_bytes()); // timestamp
        assert_eq!(burned_data.len(), 81);

        let parsed2 = Indexer::parse_krc_1155_event(&burned_data, None, 0).unwrap();
        assert_eq!(parsed2.action, Krc1155Action::Burned);
        assert_eq!(parsed2.amount, 50);
        assert_eq!(
            parsed2.owner,
            Some("0x3333333333333333333333333333333333333333333333333333333333333333".to_string())
        );
        assert_eq!(parsed2.timestamp, 2000);

        // Test action = 3 (Transferred): token_type(32) + amount(8) + action(1) + sender(32) + recipient(32) + timestamp(8) = 113 bytes
        let mut transferred_data = Vec::new();
        transferred_data.extend(std::iter::repeat(0x11u8).take(32)); // token_type
        transferred_data.extend_from_slice(&25u64.to_le_bytes()); // amount
        transferred_data.push(3u8); // action = Transferred
        transferred_data.extend(std::iter::repeat(0x22u8).take(32)); // sender
        transferred_data.extend(std::iter::repeat(0x44u8).take(32)); // recipient
        transferred_data.extend_from_slice(&3000u64.to_le_bytes()); // timestamp
        assert_eq!(transferred_data.len(), 113);

        let parsed3 = Indexer::parse_krc_1155_event(&transferred_data, None, 0).unwrap();
        assert_eq!(parsed3.action, Krc1155Action::Transferred);
        assert_eq!(parsed3.amount, 25);
        assert_eq!(
            parsed3.sender,
            Some("0x2222222222222222222222222222222222222222222222222222222222222222".to_string())
        );
        assert_eq!(
            parsed3.recipient,
            Some("0x4444444444444444444444444444444444444444444444444444444444444444".to_string())
        );
        assert_eq!(parsed3.timestamp, 3000);

        // Test unknown action
        let mut unknown_data = minted_data.clone();
        unknown_data[40] = 99;
        let parsed4 = Indexer::parse_krc_1155_event(&unknown_data, None, 0).unwrap();
        assert_eq!(parsed4.action, Krc1155Action::Unknown);

        // Test too-short data (less than 81 bytes)
        let short_data = &minted_data[0..80];
        assert!(Indexer::parse_krc_1155_event(short_data, None, 0).is_none());

        // Test empty data
        assert!(Indexer::parse_krc_1155_event(&[], None, 0).is_none());
    }
}
