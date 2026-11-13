// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Database operations for the indexer

use crate::models::*;
use crate::schema;
use anyhow::{Context, Result};
use chrono::Utc;
use kanari_types::block::Block;
use kanari_types::transaction::{SignedTransaction, Transaction};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use tracing::{debug, info};

type EventBatch<'a> = (Option<&'a str>, &'a [kanari_types::event::Event]);

/// Main database interface for the indexer
pub struct IndexerDB {
    conn: Connection,
}

impl IndexerDB {
    /// Create a new indexer database connection
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(&db_path).context("Failed to open SQLite database")?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch(
            "
            PRAGMA foreign_keys=ON;
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA cache_size=-64000;  -- 64MB cache
            PRAGMA temp_store=MEMORY;
            PRAGMA mmap_size=268435456;  -- 256MB memory mapping
            ",
        )
        .context("Failed to configure SQLite pragmas")?;

        // Initialize schema
        schema::initialize_schema(&conn).context("Failed to initialize database schema")?;

        info!("Indexer database initialized at {:?}", db_path.as_ref());

        Ok(Self { conn })
    }

    /// Create an in-memory database (useful for testing)
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("Failed to create in-memory database")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .context("Failed to enable SQLite foreign keys")?;

        schema::initialize_schema(&conn).context("Failed to initialize database schema")?;

        info!("In-memory indexer database initialized");

        Ok(Self { conn })
    }

    // =========================================================================
    // Block Operations
    // =========================================================================

    /// Insert a block into the database
    pub fn insert_block(&self, block: &Block) -> Result<()> {
        let block_hash = hex::encode(block.hash());
        let prev_hash = hex::encode(&block.header.prev_hash);
        let state_root = hex::encode(&block.header.state_root);
        let merkle_root = hex::encode(&block.header.merkle_root);

        self.conn
            .execute(
                "INSERT OR REPLACE INTO blocks
             (height, hash, prev_hash, state_root, merkle_root, timestamp, tx_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    block.header.height as i64,
                    block_hash,
                    prev_hash,
                    state_root,
                    merkle_root,
                    block.header.timestamp as i64,
                    block.header.tx_count as i64,
                ],
            )
            .context("Failed to insert block")?;

        debug!("Inserted block at height {}", block.header.height);

        Ok(())
    }

    /// Get a block by height
    pub fn get_block_by_height(&self, height: u64) -> Result<Option<IndexedBlock>> {
        let mut stmt = self
            .conn
            .prepare("SELECT height, hash, prev_hash, state_root, merkle_root, timestamp, tx_count, created_at FROM blocks WHERE height = ?1")?;

        let block = stmt
            .query_row(params![height as i64], |row| {
                Ok(IndexedBlock {
                    height: row.get::<_, i64>(0)? as u64,
                    hash: row.get(1)?,
                    prev_hash: row.get(2)?,
                    state_root: row.get(3)?,
                    merkle_root: row.get(4)?,
                    timestamp: row.get::<_, i64>(5)? as u64,
                    tx_count: row.get::<_, i64>(6)? as usize,
                    created_at: row.get(7)?,
                })
            })
            .optional()?;

        Ok(block)
    }

    /// Get the latest block height
    pub fn get_latest_height(&self) -> Result<u64> {
        let height: Option<i64> = self
            .conn
            .query_row("SELECT MAX(height) FROM blocks", [], |row| row.get(0))
            .optional()?
            .flatten();

        Ok(height.unwrap_or(0) as u64)
    }

    // =========================================================================
    // Transaction Operations
    // =========================================================================

    /// Insert transactions from a block
    pub fn insert_transactions(
        &self,
        block_height: u64,
        transactions: &[SignedTransaction],
    ) -> Result<()> {
        let mut insert_tx_stmt = self.conn.prepare_cached(
            "INSERT OR REPLACE INTO transactions
                 (tx_hash, block_height, sender, tx_type, nonce, gas_limit, gas_price, status, signature, raw_data, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )?;
        let mut insert_arg_stmt = self.conn.prepare_cached(
            "INSERT INTO transaction_args (tx_hash, arg_index, arg_value) VALUES (?1, ?2, ?3)",
        )?;

        for signed_tx in transactions {
            let tx_hash = hex::encode(signed_tx.hash());
            let sender = signed_tx.transaction.sender().to_string();
            let tx_type = signed_tx.transaction.tx_type_label();

            let signature = hex::encode(&signed_tx.signature);
            let raw_data = bcs::to_bytes(&signed_tx.transaction).ok();

            insert_tx_stmt
                .execute(params![
                    &tx_hash,
                    block_height as i64,
                    sender,
                    tx_type,
                    signed_tx.transaction.nonce() as i64,
                    signed_tx.transaction.gas_limit() as i64,
                    signed_tx.transaction.gas_price() as i64,
                    "success",
                    signature,
                    raw_data,
                    Utc::now().timestamp_millis(),
                ])
                .context("Failed to insert transaction")?;

            // Insert transaction arguments for ExecuteFunction
            if let Transaction::ExecuteFunction { args, .. } = &signed_tx.transaction {
                for (idx, arg) in args.iter().enumerate() {
                    insert_arg_stmt
                        .execute(params![&tx_hash, idx as u32, arg])
                        .context("Failed to insert transaction argument")?;
                }
            }
        }

        debug!("Inserted {} transactions for block {}", transactions.len(), block_height);

        Ok(())
    }

    /// Get a transaction by hash
    pub fn get_transaction_by_hash(&self, tx_hash: &str) -> Result<Option<IndexedTransaction>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tx_hash, block_height, sender, tx_type, nonce,
                    gas_limit, gas_price, gas_used, status, signature, raw_data, timestamp, created_at
             FROM transactions WHERE tx_hash = ?1"
        )?;

        let tx = stmt
            .query_row(params![tx_hash], |row| {
                Ok(IndexedTransaction {
                    id: row.get(0)?,
                    tx_hash: row.get(1)?,
                    block_height: row.get::<_, i64>(2)? as u64,
                    sender: row.get(3)?,
                    tx_type: row.get(4)?,
                    nonce: row.get::<_, i64>(5)? as u64,
                    gas_limit: row.get::<_, i64>(6)? as u64,
                    gas_price: row.get::<_, i64>(7)? as u64,
                    gas_used: row.get::<_, i64>(8)? as u64,
                    status: row.get(9)?,
                    signature: row.get(10)?,
                    raw_data: row.get(11)?,
                    timestamp: row.get::<_, i64>(12)? as u64,
                    created_at: row.get(13)?,
                })
            })
            .optional()?;

        Ok(tx)
    }

    /// Get transactions by block height
    pub fn get_transactions_by_block(&self, block_height: u64) -> Result<Vec<IndexedTransaction>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tx_hash, block_height, sender, tx_type, nonce,
                    gas_limit, gas_price, gas_used, status, signature, raw_data, timestamp, created_at
             FROM transactions WHERE block_height = ?1 ORDER BY id"
        )?;

        let transactions = stmt
            .query_map(params![block_height as i64], |row| {
                Ok(IndexedTransaction {
                    id: row.get(0)?,
                    tx_hash: row.get(1)?,
                    block_height: row.get::<_, i64>(2)? as u64,
                    sender: row.get(3)?,
                    tx_type: row.get(4)?,
                    nonce: row.get::<_, i64>(5)? as u64,
                    gas_limit: row.get::<_, i64>(6)? as u64,
                    gas_price: row.get::<_, i64>(7)? as u64,
                    gas_used: row.get::<_, i64>(8)? as u64,
                    status: row.get(9)?,
                    signature: row.get(10)?,
                    raw_data: row.get(11)?,
                    timestamp: row.get::<_, i64>(12)? as u64,
                    created_at: row.get(13)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(transactions)
    }

    /// Get transactions by sender address
    pub fn get_transactions_by_sender(
        &self,
        sender: &str,
        limit: u32,
    ) -> Result<Vec<IndexedTransaction>> {
        let mut stmt = self.conn.prepare(
        "SELECT id, tx_hash, block_height, sender, tx_type, nonce,
                    gas_limit, gas_price, gas_used, status, signature, raw_data, timestamp, created_at
             FROM transactions WHERE sender = ?1 ORDER BY id DESC LIMIT ?2"
        )?;

        let transactions = stmt
            .query_map(params![sender, limit], |row| {
                Ok(IndexedTransaction {
                    id: row.get(0)?,
                    tx_hash: row.get(1)?,
                    block_height: row.get::<_, i64>(2)? as u64,
                    sender: row.get(3)?,
                    tx_type: row.get(4)?,
                    nonce: row.get::<_, i64>(5)? as u64,
                    gas_limit: row.get::<_, i64>(6)? as u64,
                    gas_price: row.get::<_, i64>(7)? as u64,
                    gas_used: row.get::<_, i64>(8)? as u64,
                    status: row.get(9)?,
                    signature: row.get(10)?,
                    raw_data: row.get(11)?,
                    timestamp: row.get::<_, i64>(12)? as u64,
                    created_at: row.get(13)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(transactions)
    }

    // =========================================================================
    // Event Operations
    // =========================================================================

    /// Insert events from a block
    pub fn insert_events(
        &self,
        block_height: u64,
        tx_hash_events: &[EventBatch<'_>],
    ) -> Result<()> {
        let mut insert_event_stmt = self.conn.prepare_cached(
            "INSERT INTO events (event_key, sequence_number, type_tag, event_data, tx_hash, block_height)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;

        for &(tx_hash, events) in tx_hash_events {
            for event in events {
                let event_key = hex::encode(&event.key);
                let event_data = if event.event_data.is_empty() {
                    None
                } else {
                    Some(event.event_data.as_slice())
                };

                insert_event_stmt
                    .execute(params![
                        event_key,
                        event.sequence_number as i64,
                        &event.type_tag,
                        event_data,
                        tx_hash,
                        block_height as i64,
                    ])
                    .context("Failed to insert event")?;
            }
        }

        debug!("Inserted events for block {}", block_height);

        Ok(())
    }

    /// Get events by transaction hash
    pub fn get_events_by_transaction(&self, tx_hash: &str) -> Result<Vec<IndexedEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, event_key, sequence_number, type_tag, event_data, tx_hash, block_height, created_at
             FROM events WHERE tx_hash = ?1 ORDER BY id"
        )?;

        let events = stmt
            .query_map(params![tx_hash], |row| {
                Ok(IndexedEvent {
                    id: row.get(0)?,
                    event_key: row.get(1)?,
                    sequence_number: row.get::<_, i64>(2)? as u64,
                    type_tag: row.get(3)?,
                    event_data: row.get(4)?,
                    tx_hash: row.get(5)?,
                    block_height: row.get::<_, i64>(6)? as u64,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// Get events by event key
    pub fn get_events_by_key(&self, event_key: &str, limit: u32) -> Result<Vec<IndexedEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, event_key, sequence_number, type_tag, event_data, tx_hash, block_height, created_at
             FROM events WHERE event_key = ?1 ORDER BY id DESC LIMIT ?2"
        )?;

        let events = stmt
            .query_map(params![event_key, limit], |row| {
                Ok(IndexedEvent {
                    id: row.get(0)?,
                    event_key: row.get(1)?,
                    sequence_number: row.get::<_, i64>(2)? as u64,
                    type_tag: row.get(3)?,
                    event_data: row.get(4)?,
                    tx_hash: row.get(5)?,
                    block_height: row.get::<_, i64>(6)? as u64,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// Get recent events by their fully-qualified Move type tag.
    pub fn get_events_by_type_tag(&self, type_tag: &str, limit: u32) -> Result<Vec<IndexedEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, event_key, sequence_number, type_tag, event_data, tx_hash, block_height, created_at
             FROM events WHERE type_tag = ?1 ORDER BY id DESC LIMIT ?2",
        )?;

        let events = stmt
            .query_map(params![type_tag, limit], |row| {
                Ok(IndexedEvent {
                    id: row.get(0)?,
                    event_key: row.get(1)?,
                    sequence_number: row.get::<_, i64>(2)? as u64,
                    type_tag: row.get(3)?,
                    event_data: row.get(4)?,
                    tx_hash: row.get(5)?,
                    block_height: row.get::<_, i64>(6)? as u64,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    // =========================================================================
    // Object Locked Supply Change Event Operations
    // =========================================================================

    /// Insert or update an object locked supply change event
    pub fn upsert_object_locked_supply_event(
        &self,
        event: &IndexedObjectLockedSupplyEvent,
    ) -> Result<()> {
        let mut stmt = self.conn.prepare_cached(
            "INSERT INTO object_locked_supply_events
             (tx_hash, block_height, object_id, token_type, amount, action, action_label, actor, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        stmt.execute(params![
            event.tx_hash,
            event.block_height as i64,
            event.object_id,
            event.token_type,
            event.amount as i64,
            event.action as i64,
            event.action_label,
            event.actor,
            event.timestamp as i64,
        ])
        .context("Failed to insert object locked supply event")?;
        Ok(())
    }

    /// Get object locked supply change events by transaction hash
    pub fn get_object_locked_supply_events_by_tx(
        &self,
        tx_hash: &str,
    ) -> Result<Vec<IndexedObjectLockedSupplyEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tx_hash, block_height, object_id, token_type, amount, action, action_label, actor, timestamp, created_at
             FROM object_locked_supply_events WHERE tx_hash = ?1 ORDER BY id",
        )?;
        let events = stmt
            .query_map(params![tx_hash], |row| {
                Ok(IndexedObjectLockedSupplyEvent {
                    id: row.get(0)?,
                    tx_hash: row.get(1)?,
                    block_height: row.get::<_, i64>(2)? as u64,
                    object_id: row.get(3)?,
                    token_type: row.get(4)?,
                    amount: row.get::<_, i64>(5)? as u64,
                    action: row.get::<_, i64>(6)? as u8,
                    action_label: row.get(7)?,
                    actor: row.get(8)?,
                    timestamp: row.get::<_, i64>(9)? as u64,
                    created_at: row.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    /// Get object locked supply change events by object id
    pub fn get_object_locked_supply_events_by_object(
        &self,
        object_id: &str,
        limit: u32,
    ) -> Result<Vec<IndexedObjectLockedSupplyEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tx_hash, block_height, object_id, token_type, amount, action, action_label, actor, timestamp, created_at
             FROM object_locked_supply_events WHERE object_id = ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let events = stmt
            .query_map(params![object_id, limit], |row| {
                Ok(IndexedObjectLockedSupplyEvent {
                    id: row.get(0)?,
                    tx_hash: row.get(1)?,
                    block_height: row.get::<_, i64>(2)? as u64,
                    object_id: row.get(3)?,
                    token_type: row.get(4)?,
                    amount: row.get::<_, i64>(5)? as u64,
                    action: row.get::<_, i64>(6)? as u8,
                    action_label: row.get(7)?,
                    actor: row.get(8)?,
                    timestamp: row.get::<_, i64>(9)? as u64,
                    created_at: row.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    /// Get aggregated locked amount per token type
    pub fn get_locked_supply_summary(&self) -> Result<Vec<(String, u64, u64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT token_type,
                    SUM(CASE WHEN action = 1 THEN amount ELSE 0 END) as locked_amount,
                    SUM(CASE WHEN action = 2 OR action = 3 THEN amount ELSE 0 END) as released_amount
             FROM object_locked_supply_events
             GROUP BY token_type",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)? as u64,
                    row.get::<_, i64>(2)? as u64,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // =========================================================================
    // KRC-1155 Event Operations
    // =========================================================================

    /// Insert a KRC-1155 event (Minted / Burned / Transferred)
    pub fn upsert_krc_1155_event(&self, event: &IndexedKrc1155Event) -> Result<()> {
        let mut stmt = self.conn.prepare_cached(
            "INSERT INTO krc_1155_events
             (tx_hash, block_height, token_type, action, amount, sender, recipient, owner, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        stmt.execute(params![
            event.tx_hash,
            event.block_height as i64,
            event.token_type,
            event.action.as_str(),
            event.amount as i64,
            event.sender,
            event.recipient,
            event.owner,
            event.timestamp as i64,
        ])
        .context("Failed to insert KRC-1155 event")?;
        Ok(())
    }

    /// Get KRC-1155 events by token type
    pub fn get_krc_1155_events_by_token(
        &self,
        token_type: &str,
        limit: u32,
    ) -> Result<Vec<IndexedKrc1155Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tx_hash, block_height, token_type, action, amount, sender, recipient, owner, timestamp, created_at
             FROM krc_1155_events WHERE token_type = ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let events = stmt
            .query_map(params![token_type, limit], |row| {
                Ok(IndexedKrc1155Event {
                    id: row.get(0)?,
                    tx_hash: row.get(1)?,
                    block_height: row.get::<_, i64>(2)? as u64,
                    token_type: row.get(3)?,
                    action: match row.get::<_, String>(4)?.as_str() {
                        "minted" => Krc1155Action::Minted,
                        "burned" => Krc1155Action::Burned,
                        "transferred" => Krc1155Action::Transferred,
                        _ => Krc1155Action::Unknown,
                    },
                    amount: row.get::<_, i64>(5)? as u64,
                    sender: row.get(6)?,
                    recipient: row.get(7)?,
                    owner: row.get(8)?,
                    timestamp: row.get::<_, i64>(9)? as u64,
                    created_at: row.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    /// Get KRC-1155 events involving an address (as sender, recipient, or owner)
    pub fn get_krc_1155_events_by_address(
        &self,
        address: &str,
        limit: u32,
    ) -> Result<Vec<IndexedKrc1155Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tx_hash, block_height, token_type, action, amount, sender, recipient, owner, timestamp, created_at
             FROM krc_1155_events WHERE sender = ?1 OR recipient = ?1 OR owner = ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let events = stmt
            .query_map(params![address, limit], |row| {
                Ok(IndexedKrc1155Event {
                    id: row.get(0)?,
                    tx_hash: row.get(1)?,
                    block_height: row.get::<_, i64>(2)? as u64,
                    token_type: row.get(3)?,
                    action: match row.get::<_, String>(4)?.as_str() {
                        "minted" => Krc1155Action::Minted,
                        "burned" => Krc1155Action::Burned,
                        "transferred" => Krc1155Action::Transferred,
                        _ => Krc1155Action::Unknown,
                    },
                    amount: row.get::<_, i64>(5)? as u64,
                    sender: row.get(6)?,
                    recipient: row.get(7)?,
                    owner: row.get(8)?,
                    timestamp: row.get::<_, i64>(9)? as u64,
                    created_at: row.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    // =========================================================================
    // KRC-721 Event Operations
    // =========================================================================

    /// Insert a KRC-721 event (MintLog)
    pub fn upsert_krc_721_event(&self, event: &IndexedKrc721Event) -> Result<()> {
        let mut stmt = self.conn.prepare_cached(
            "INSERT INTO krc_721_events
             (tx_hash, block_height, object_id, creator, collection_id, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        stmt.execute(params![
            event.tx_hash,
            event.block_height as i64,
            event.object_id,
            event.creator,
            event.collection_id,
            event.timestamp as i64,
        ])
        .context("Failed to insert KRC-721 event")?;
        Ok(())
    }

    /// Get KRC-721 events by collection id
    pub fn get_krc_721_events_by_collection(
        &self,
        collection_id: &str,
        limit: u32,
    ) -> Result<Vec<IndexedKrc721Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tx_hash, block_height, object_id, creator, collection_id, timestamp, created_at
             FROM krc_721_events WHERE collection_id = ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let events = stmt
            .query_map(params![collection_id, limit], |row| {
                Ok(IndexedKrc721Event {
                    id: row.get(0)?,
                    tx_hash: row.get(1)?,
                    block_height: row.get::<_, i64>(2)? as u64,
                    object_id: row.get(3)?,
                    creator: row.get(4)?,
                    collection_id: row.get(5)?,
                    timestamp: row.get::<_, i64>(6)? as u64,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    /// Get KRC-721 events by creator address
    pub fn get_krc_721_events_by_creator(
        &self,
        creator: &str,
        limit: u32,
    ) -> Result<Vec<IndexedKrc721Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tx_hash, block_height, object_id, creator, collection_id, timestamp, created_at
             FROM krc_721_events WHERE creator = ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let events = stmt
            .query_map(params![creator, limit], |row| {
                Ok(IndexedKrc721Event {
                    id: row.get(0)?,
                    tx_hash: row.get(1)?,
                    block_height: row.get::<_, i64>(2)? as u64,
                    object_id: row.get(3)?,
                    creator: row.get(4)?,
                    collection_id: row.get(5)?,
                    timestamp: row.get::<_, i64>(6)? as u64,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    // =========================================================================
    // Coin Operations
    // =========================================================================

    /// Insert or update a coin
    pub fn upsert_coin(&self, coin: &IndexedCoin) -> Result<()> {
        let previous_owner_and_type: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT owner, coin_type FROM coins WHERE id = ?1",
                params![&coin.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        let mut stmt = self.conn.prepare_cached(
            "INSERT INTO coins (id, owner, coin_type, balance, is_frozen, created_tx_hash, last_updated_tx_hash, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                owner = excluded.owner,
                coin_type = excluded.coin_type,
                balance = excluded.balance,
                is_frozen = excluded.is_frozen,
                last_updated_tx_hash = excluded.last_updated_tx_hash,
                updated_at = excluded.updated_at",
        )?;
        stmt.execute(params![
            &coin.id,
            &coin.owner,
            &coin.coin_type,
            coin.balance as i64,
            coin.is_frozen as i32,
            &coin.created_tx_hash,
            &coin.last_updated_tx_hash,
            coin.created_at,
            coin.updated_at,
        ])
        .context("Failed to upsert coin")?;

        self.refresh_owner_balance(&coin.owner, &coin.coin_type)?;

        if let Some((previous_owner, previous_coin_type)) = previous_owner_and_type
            && (previous_owner != coin.owner || previous_coin_type != coin.coin_type)
        {
            self.refresh_owner_balance(&previous_owner, &previous_coin_type)?;
        }

        Ok(())
    }

    /// Recompute owner balance aggregation from canonical coin rows.
    fn refresh_owner_balance(&self, address: &str, coin_type: &str) -> Result<()> {
        let (total_balance, coin_count): (i64, i64) = self.conn.query_row(
            "SELECT COALESCE(SUM(balance), 0), COUNT(*)
             FROM coins
             WHERE owner = ?1 AND coin_type = ?2",
            params![address, coin_type],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        if coin_count == 0 {
            self.conn.execute(
                "DELETE FROM owner_balances WHERE address = ?1 AND coin_type = ?2",
                params![address, coin_type],
            )?;
            return Ok(());
        }

        let mut stmt = self.conn.prepare_cached(
            "INSERT INTO owner_balances (address, coin_type, total_balance, coin_count, last_updated)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(address, coin_type) DO UPDATE SET
                total_balance = excluded.total_balance,
                coin_count = excluded.coin_count,
                last_updated = datetime('now')",
        )?;
        stmt.execute(params![address, coin_type, total_balance, coin_count])
            .context("Failed to refresh owner balance")?;

        Ok(())
    }

    /// Get coin by ID
    pub fn get_coin_by_id(&self, coin_id: &str) -> Result<Option<IndexedCoin>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, owner, coin_type, balance, is_frozen, created_tx_hash, last_updated_tx_hash, created_at, updated_at
             FROM coins WHERE id = ?1"
        )?;

        let coin = stmt
            .query_row(params![coin_id], |row| {
                Ok(IndexedCoin {
                    id: row.get(0)?,
                    owner: row.get(1)?,
                    coin_type: row.get(2)?,
                    balance: row.get::<_, i64>(3)? as u64,
                    is_frozen: row.get::<_, i32>(4)? != 0,
                    created_tx_hash: row.get(5)?,
                    last_updated_tx_hash: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .optional()?;

        Ok(coin)
    }

    /// Get coins by owner
    pub fn get_coins_by_owner(&self, owner: &str) -> Result<Vec<IndexedCoin>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, owner, coin_type, balance, is_frozen, created_tx_hash, last_updated_tx_hash, created_at, updated_at
             FROM coins WHERE owner = ?1 ORDER BY coin_type"
        )?;

        let coins = stmt
            .query_map(params![owner], |row| {
                Ok(IndexedCoin {
                    id: row.get(0)?,
                    owner: row.get(1)?,
                    coin_type: row.get(2)?,
                    balance: row.get::<_, i64>(3)? as u64,
                    is_frozen: row.get::<_, i32>(4)? != 0,
                    created_tx_hash: row.get(5)?,
                    last_updated_tx_hash: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(coins)
    }

    /// Get aggregated balance for an owner and coin type.
    pub fn get_owner_balance(
        &self,
        address: &str,
        coin_type: &str,
    ) -> Result<Option<OwnerBalance>> {
        let mut stmt = self.conn.prepare(
            "SELECT address, coin_type, total_balance, coin_count, last_updated
             FROM owner_balances WHERE address = ?1 AND coin_type = ?2",
        )?;

        let balance = stmt
            .query_row(params![address, coin_type], |row| {
                Ok(OwnerBalance {
                    address: row.get(0)?,
                    coin_type: row.get(1)?,
                    total_balance: row.get::<_, i64>(2)? as u64,
                    coin_count: row.get::<_, i32>(3)? as u32,
                    last_updated: row.get(4)?,
                })
            })
            .optional()?;

        Ok(balance)
    }

    /// Get all aggregated balances for an owner.
    pub fn get_all_owner_balances(&self, address: &str) -> Result<Vec<OwnerBalance>> {
        let mut stmt = self.conn.prepare(
            "SELECT address, coin_type, total_balance, coin_count, last_updated
             FROM owner_balances WHERE address = ?1 ORDER BY coin_type",
        )?;

        let balances = stmt
            .query_map(params![address], |row| {
                Ok(OwnerBalance {
                    address: row.get(0)?,
                    coin_type: row.get(1)?,
                    total_balance: row.get::<_, i64>(2)? as u64,
                    coin_count: row.get::<_, i32>(3)? as u32,
                    last_updated: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(balances)
    }
    // =========================================================================
    // Metadata Operations
    // =========================================================================

    /// Get metadata value by key
    pub fn get_metadata(&self, key: &str) -> Result<Option<String>> {
        let value: Option<String> = self
            .conn
            .query_row("SELECT value FROM indexer_metadata WHERE key = ?1", params![key], |row| {
                row.get(0)
            })
            .optional()?;

        Ok(value)
    }

    /// Set metadata value
    pub fn set_metadata(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO indexer_metadata (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
            params![key, value],
        )
        .context("Failed to set metadata")?;

        Ok(())
    }

    /// Update last indexed height
    pub fn update_last_indexed_height(&self, height: u64) -> Result<()> {
        self.set_metadata("last_indexed_height", &height.to_string())
    }

    /// Remove all indexed chain-derived data so the index can be rebuilt cleanly.
    pub fn clear_all_indexed_data(&self) -> Result<()> {
        self.conn
            .execute_batch(
                "
                DELETE FROM transaction_args;
                DELETE FROM object_locked_supply_events;
                DELETE FROM krc_1155_events;
                DELETE FROM krc_721_events;
                DELETE FROM events;
                DELETE FROM transactions;
                DELETE FROM blocks;
                DELETE FROM coins;
                DELETE FROM owner_balances;
                ",
            )
            .context("Failed to clear indexed data")?;
        self.update_last_indexed_height(0)?;
        Ok(())
    }

    /// Get last indexed height
    pub fn get_last_indexed_height(&self) -> Result<u64> {
        let value = self.get_metadata("last_indexed_height")?.unwrap_or_else(|| "0".to_string());
        value.parse::<u64>().map_err(|e| anyhow::anyhow!("Failed to parse height: {}", e))
    }

    // =========================================================================
    // Statistics & Analytics
    // =========================================================================

    /// Get total number of blocks
    pub fn get_block_count(&self) -> Result<u64> {
        let count: i64 =
            self.conn.query_row("SELECT COUNT(*) FROM blocks", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Get total number of transactions
    pub fn get_transaction_count(&self) -> Result<u64> {
        let count: i64 =
            self.conn.query_row("SELECT COUNT(*) FROM transactions", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Get total number of events
    pub fn get_event_count(&self) -> Result<u64> {
        let count: i64 =
            self.conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Get transaction statistics by type
    pub fn get_transaction_stats(&self) -> Result<Vec<(String, u64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT tx_type, COUNT(*) FROM transactions GROUP BY tx_type ORDER BY COUNT(*) DESC",
        )?;

        let stats = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get::<_, i64>(1)? as u64)))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(stats)
    }

    /// Get top addresses by transaction count
    pub fn get_top_addresses(&self, limit: u32) -> Result<Vec<(String, u64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT sender, COUNT(*) as tx_count FROM transactions GROUP BY sender ORDER BY tx_count DESC LIMIT ?1"
        )?;

        let addresses = stmt
            .query_map(params![limit], |row| Ok((row.get(0)?, row.get::<_, i64>(1)? as u64)))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(addresses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_in_memory_db() {
        let db = IndexerDB::new_in_memory().unwrap();
        assert_eq!(db.get_latest_height().unwrap(), 0);
    }

    #[test]
    fn test_metadata_operations() {
        let db = IndexerDB::new_in_memory().unwrap();

        db.set_metadata("test_key", "test_value").unwrap();
        let value = db.get_metadata("test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[test]
    fn test_owner_balance_recomputed_on_coin_update() {
        let db = IndexerDB::new_in_memory().unwrap();

        let now = Utc::now();
        let coin = IndexedCoin {
            id: "coin-1".to_string(),
            owner: "0x1".to_string(),
            coin_type: "0x2::kanari::KANARI".to_string(),
            balance: 10,
            is_frozen: false,
            created_tx_hash: None,
            last_updated_tx_hash: None,
            created_at: now,
            updated_at: now,
        };
        db.upsert_coin(&coin).unwrap();

        let mut updated_coin = coin.clone();
        updated_coin.balance = 25;
        updated_coin.updated_at = Utc::now();
        db.upsert_coin(&updated_coin).unwrap();

        let owner_balance = db.get_owner_balance("0x1", "0x2::kanari::KANARI").unwrap().unwrap();
        assert_eq!(owner_balance.total_balance, 25);
        assert_eq!(owner_balance.coin_count, 1);
    }

    #[test]
    fn test_object_locked_supply_event_insert_and_query() {
        let db = IndexerDB::new_in_memory().unwrap();

        // Insert a block row to satisfy FK constraints
        db.conn.execute(
            "INSERT INTO blocks (height, hash, prev_hash, state_root, merkle_root, timestamp, tx_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![10i64, "blockhash10", "prevhash", "stateroot", "merkleroot", 1000i64, 0i64],
        ).unwrap();

        let event = IndexedObjectLockedSupplyEvent {
            id: 0,
            tx_hash: None,
            block_height: 10,
            object_id: "0xbbbb".to_string(),
            token_type: "0x2::kanari::KANARI".to_string(),
            amount: 100,
            action: 1, // locked
            action_label: "locked".to_string(),
            actor: "0x1111".to_string(),
            timestamp: 1000,
            created_at: Utc::now(),
        };

        db.upsert_object_locked_supply_event(&event).unwrap();

        // Query by tx hash (None won't match)
        let by_tx = db.get_object_locked_supply_events_by_tx("0xtx1").unwrap();
        assert_eq!(by_tx.len(), 0);

        // Query by object id
        let by_obj = db.get_object_locked_supply_events_by_object("0xbbbb", 10).unwrap();
        assert_eq!(by_obj.len(), 1);
        assert_eq!(by_obj[0].tx_hash, None);
        assert_eq!(by_obj[0].amount, 100);
    }

    #[test]
    fn test_object_locked_supply_event_release_and_refund() {
        let db = IndexerDB::new_in_memory().unwrap();

        // Insert block rows to satisfy FK constraints
        db.conn.execute(
            "INSERT INTO blocks (height, hash, prev_hash, state_root, merkle_root, timestamp, tx_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![10i64, "blockhash10", "prevhash", "stateroot", "merkleroot", 1000i64, 0i64],
        ).unwrap();
        db.conn.execute(
            "INSERT INTO blocks (height, hash, prev_hash, state_root, merkle_root, timestamp, tx_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![20i64, "blockhash20", "prevhash", "stateroot", "merkleroot", 2000i64, 0i64],
        ).unwrap();
        db.conn.execute(
            "INSERT INTO blocks (height, hash, prev_hash, state_root, merkle_root, timestamp, tx_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![30i64, "blockhash30", "prevhash", "stateroot", "merkleroot", 3000i64, 0i64],
        ).unwrap();

        // Insert a lock event
        let lock_event = IndexedObjectLockedSupplyEvent {
            id: 0,
            tx_hash: None,
            block_height: 10,
            object_id: "0xbbbb".to_string(),
            token_type: "0x2::kanari::KANARI".to_string(),
            amount: 100,
            action: 1,
            action_label: "locked".to_string(),
            actor: "0x1111".to_string(),
            timestamp: 1000,
            created_at: Utc::now(),
        };
        db.upsert_object_locked_supply_event(&lock_event).unwrap();

        // Insert a release event
        let release_event = IndexedObjectLockedSupplyEvent {
            id: 0,
            tx_hash: None,
            block_height: 20,
            object_id: "0xbbbb".to_string(),
            token_type: "0x2::kanari::KANARI".to_string(),
            amount: 100,
            action: 2,
            action_label: "released".to_string(),
            actor: "0x2222".to_string(),
            timestamp: 2000,
            created_at: Utc::now(),
        };
        db.upsert_object_locked_supply_event(&release_event).unwrap();

        // Insert a refund event
        let refund_event = IndexedObjectLockedSupplyEvent {
            id: 0,
            tx_hash: None,
            block_height: 30,
            object_id: "0xcccc".to_string(),
            token_type: "0x2::kanari::KANARI".to_string(),
            amount: 50,
            action: 3,
            action_label: "refunded".to_string(),
            actor: "0x1111".to_string(),
            timestamp: 3000,
            created_at: Utc::now(),
        };
        db.upsert_object_locked_supply_event(&refund_event).unwrap();

        // Check summary
        let summary = db.get_locked_supply_summary().unwrap();
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].0, "0x2::kanari::KANARI");
        assert_eq!(summary[0].1, 100); // locked
        assert_eq!(summary[0].2, 150); // released + refunded

        // Check events by object
        let bbbb_events = db.get_object_locked_supply_events_by_object("0xbbbb", 10).unwrap();
        assert_eq!(bbbb_events.len(), 2);
        assert_eq!(bbbb_events[0].action, 2); // most recent first (release)
        assert_eq!(bbbb_events[1].action, 1); // lock

        let cccc_events = db.get_object_locked_supply_events_by_object("0xcccc", 10).unwrap();
        assert_eq!(cccc_events.len(), 1);
        assert_eq!(cccc_events[0].action, 3); // refunded
    }
}
