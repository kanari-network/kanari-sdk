// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Database schema definitions and initialization

use rusqlite::{Connection, Result as SqliteResult};

fn table_has_column(conn: &Connection, table: &str, column: &str) -> SqliteResult<bool> {
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&pragma)?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for entry in columns {
        if entry? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ensure_column_exists(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> SqliteResult<()> {
    if !table_has_column(conn, table, column)? {
        let alter = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
        conn.execute(&alter, [])?;
    }
    Ok(())
}

fn migrate_transactions_nonce_schema(conn: &Connection) -> SqliteResult<()> {
    if !table_has_column(conn, "transactions", "sequence_number")? {
        return Ok(());
    }

    ensure_column_exists(conn, "transactions", "nonce", "INTEGER")?;
    conn.execute(
        "UPDATE transactions SET nonce = COALESCE(nonce, sequence_number) WHERE nonce IS NULL",
        [],
    )?;

    conn.execute_batch(
        "
        DROP INDEX IF EXISTS idx_transactions_hash;
        DROP INDEX IF EXISTS idx_transactions_sender;
        DROP INDEX IF EXISTS idx_transactions_block;
        DROP INDEX IF EXISTS idx_transactions_type;

        ALTER TABLE transactions RENAME TO transactions_legacy_nonce_migration;

        CREATE TABLE transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tx_hash TEXT NOT NULL UNIQUE,
            block_height INTEGER NOT NULL,
            sender TEXT NOT NULL,
            tx_type TEXT NOT NULL,
            nonce INTEGER NOT NULL,
            gas_limit INTEGER NOT NULL,
            gas_price INTEGER NOT NULL,
            gas_used INTEGER DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'success',
            signature TEXT NOT NULL,
            raw_data BLOB,
            timestamp INTEGER NOT NULL,
            created_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY (block_height) REFERENCES blocks(height) ON DELETE CASCADE
        );

        INSERT INTO transactions (
            id, tx_hash, block_height, sender, tx_type, nonce, gas_limit, gas_price,
            gas_used, status, signature, raw_data, timestamp, created_at
        )
        SELECT
            id, tx_hash, block_height, sender, tx_type, COALESCE(nonce, sequence_number),
            gas_limit, gas_price, gas_used, status, signature, raw_data, timestamp, created_at
        FROM transactions_legacy_nonce_migration;

        DROP TABLE transactions_legacy_nonce_migration;

        CREATE INDEX IF NOT EXISTS idx_transactions_hash ON transactions(tx_hash);
        CREATE INDEX IF NOT EXISTS idx_transactions_sender ON transactions(sender);
        CREATE INDEX IF NOT EXISTS idx_transactions_block ON transactions(block_height);
        CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(tx_type);
        ",
    )?;

    Ok(())
}

/// Initialize the database schema
pub fn initialize_schema(conn: &Connection) -> SqliteResult<()> {
    // Create blocks table
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS blocks (
            height INTEGER PRIMARY KEY,
            hash TEXT NOT NULL UNIQUE,
            prev_hash TEXT NOT NULL,
            state_root TEXT NOT NULL,
            merkle_root TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            tx_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks(hash);
        CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp);

        -- Create transactions table
        CREATE TABLE IF NOT EXISTS transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tx_hash TEXT NOT NULL UNIQUE,
            block_height INTEGER NOT NULL,
            sender TEXT NOT NULL,
            tx_type TEXT NOT NULL,
            nonce INTEGER NOT NULL,
            gas_limit INTEGER NOT NULL,
            gas_price INTEGER NOT NULL,
            gas_used INTEGER DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'success',
            signature TEXT NOT NULL,
            raw_data BLOB,
            timestamp INTEGER NOT NULL,
            created_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY (block_height) REFERENCES blocks(height) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_transactions_hash ON transactions(tx_hash);
        CREATE INDEX IF NOT EXISTS idx_transactions_sender ON transactions(sender);
        CREATE INDEX IF NOT EXISTS idx_transactions_block ON transactions(block_height);
        CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(tx_type);

        -- Create transaction arguments table (for ExecuteFunction args)
        CREATE TABLE IF NOT EXISTS transaction_args (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tx_hash TEXT NOT NULL,
            arg_index INTEGER NOT NULL,
            arg_value BLOB,
            FOREIGN KEY (tx_hash) REFERENCES transactions(tx_hash) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_transaction_args_tx ON transaction_args(tx_hash);

        -- Create events table
        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_key TEXT NOT NULL,
            sequence_number INTEGER NOT NULL,
            type_tag TEXT NOT NULL,
            event_data BLOB,
            tx_hash TEXT NOT NULL,
            block_height INTEGER NOT NULL,
            created_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY (tx_hash) REFERENCES transactions(tx_hash) ON DELETE CASCADE,
            FOREIGN KEY (block_height) REFERENCES blocks(height) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_events_key ON events(event_key);
        CREATE INDEX IF NOT EXISTS idx_events_tx ON events(tx_hash);
        CREATE INDEX IF NOT EXISTS idx_events_block ON events(block_height);
        CREATE INDEX IF NOT EXISTS idx_events_type ON events(type_tag);

        -- Create coins table
        CREATE TABLE IF NOT EXISTS coins (
            id TEXT PRIMARY KEY,
            owner TEXT NOT NULL,
            coin_type TEXT NOT NULL,
            balance INTEGER NOT NULL DEFAULT 0,
            is_frozen INTEGER NOT NULL DEFAULT 0,
            created_tx_hash TEXT,
            last_updated_tx_hash TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_coins_owner ON coins(owner);
        CREATE INDEX IF NOT EXISTS idx_coins_type ON coins(coin_type);

        -- Create owner balances table (aggregated view)
        CREATE TABLE IF NOT EXISTS owner_balances (
            address TEXT NOT NULL,
            coin_type TEXT NOT NULL,
            total_balance INTEGER NOT NULL DEFAULT 0,
            coin_count INTEGER NOT NULL DEFAULT 0,
            last_updated TEXT DEFAULT (datetime('now')),
            PRIMARY KEY (address, coin_type)
        );

        CREATE INDEX IF NOT EXISTS idx_owner_balances_address ON owner_balances(address);
        CREATE INDEX IF NOT EXISTS idx_owner_balances_coin_type ON owner_balances(coin_type);

        -- Create indexer metadata table
        CREATE TABLE IF NOT EXISTS indexer_metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT DEFAULT (datetime('now'))
        );

        -- Insert initial metadata
        INSERT OR IGNORE INTO indexer_metadata (key, value) VALUES ('version', '1');
        INSERT OR IGNORE INTO indexer_metadata (key, value) VALUES ('last_indexed_height', '0');
        ",
    )?;

    migrate_transactions_nonce_schema(conn)?;

    Ok(())
}
