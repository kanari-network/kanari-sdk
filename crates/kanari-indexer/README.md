# Kanari Indexer

A high-performance blockchain indexer for the Kanari Network using SQLite.

## Overview

The Kanari Indexer provides efficient storage and querying capabilities for blockchain data, including:

- **Blocks**: Complete block information with headers and metadata
- **Transactions**: All transaction types (Transfer, ExecuteFunction, PublishModule, Burn)
- **Events**: Move VM events emitted during transaction execution
- **Coins**: Individual coin objects with ownership tracking
- **Owner Balances**: Aggregated balance information per owner address
- **Analytics**: Statistics and queries for blockchain analysis

## Features

✅ **SQLite Backend**: Lightweight, embedded database with excellent performance  
✅ **Comprehensive Indexing**: Blocks, transactions, events, coins, and balances  
✅ **Efficient Queries**: Optimized indexes for common query patterns  
✅ **Batch Operations**: Support for bulk indexing operations  
✅ **Sync Capabilities**: Incremental sync from any block height  
✅ **Statistics & Analytics**: Built-in analytics and reporting  
✅ **Type Safety**: Full Rust type safety with proper error handling  

## Architecture

```basic
kanari-indexer/
├── src/
│   ├── lib.rs          # Main library exports
│   ├── schema.rs       # Database schema definitions
│   ├── models.rs       # Data models for indexed entities
│   ├── db.rs           # Database operations layer
│   └── indexer.rs      # Main indexer logic
├── Cargo.toml
└── README.md
```

## Usage

### Basic Setup

```rust
use kanari_indexer::{Indexer, IndexerConfig};
use std::path::PathBuf;

// Create indexer configuration
let config = IndexerConfig {
    db_path: PathBuf::from("kanari_indexer.db"),
    in_memory: false,
    batch_size: 100,
};

// Initialize indexer
let indexer = Indexer::new(config)?;
```

### Indexing Blocks

```rust
use kanari_types::block::Block;

// Index a single block
let block: Block = get_block_from_node();
indexer.index_block(&block)?;

// Index multiple blocks in batch
let blocks: Vec<Block> = get_blocks_batch();
indexer.index_blocks_batch(&blocks)?;
```

### Syncing from Node

```rust
// Sync from current position to latest block
let synced_count = indexer.sync_to_latest(|height| {
    // Your function to fetch block by height
    fetch_block_from_node(height)
})?;

println!("Synced {} blocks", synced_count);
```

### Querying Data

```rust
use kanari_indexer::IndexedTransaction;

// Get block by height
if let Some(block) = indexer.db().get_block_by_height(100)? {
    println!("Block hash: {}", block.hash);
}

// Get transactions by sender
let txs = indexer.db().get_transactions_by_sender("0x123...", 10)?;
for tx in txs {
    println!("TX: {} - Type: {}", tx.tx_hash, tx.tx_type);
}

// Get owner balances
let balances = indexer.db().get_all_owner_balances("0x123...")?;
for balance in balances {
    println!("{}: {} coins", balance.coin_type, balance.total_balance);
}

// Get events by transaction
let events = indexer.db().get_events_by_transaction(&tx_hash)?;
```

### Statistics

```rust
let stats = indexer.get_statistics()?;
println!("{}", stats);

// Output:
// === Indexer Statistics ===
// Blocks indexed: 1234
// Transactions indexed: 5678
// Events indexed: 9012
// Last indexed height: 1233
//
// Transaction types:
//   transfer: 3000
//   execute_function: 2000
//   publish_module: 500
//   burn: 178
```

## Database Schema

### Tables

- **blocks**: Block headers and metadata
- **transactions**: All transaction data
- **transaction_args**: Function call arguments
- **events**: Move VM events
- **coins**: Individual coin objects
- **owner_balances**: Aggregated balances per owner address
- **indexer_metadata**: Internal indexer state

### Indexes

Optimized indexes on:

- Block hash and height
- Transaction hash, sender, and block
- Event keys and types
- Coin owners and types
- Owner addresses

## Configuration

```rust
pub struct IndexerConfig {
    /// Path to SQLite database file
    pub db_path: PathBuf,
    
    /// Use in-memory database (for testing)
    pub in_memory: bool,
    
    /// Batch size for progress reporting
    pub batch_size: u32,
}
```

## Performance Tuning

The indexer uses optimized SQLite settings:

- **WAL Mode**: Write-Ahead Logging for concurrent reads
- **64MB Cache**: In-memory page cache
- **256MB MMAP**: Memory-mapped I/O for large databases
- **Normal Sync**: Balance between durability and performance

## Testing

```bash
# Run tests
cargo test -p kanari-indexer

# Run with verbose output
cargo test -p kanari-indexer -- --nocapture
```

## Examples

See the examples directory for complete usage examples:

- `basic_indexing.rs`: Simple block indexing
- `sync_example.rs`: Syncing from a blockchain node
- `query_examples.rs`: Various query patterns
- `analytics.rs`: Generating statistics and reports

## Error Handling

All operations return `Result<T>` with descriptive error messages using `anyhow`. Common errors:

- Database connection failures
- Schema initialization errors
- Data insertion/conversion errors
- Query execution failures

## Best Practices

1. **Batch Operations**: Use `index_blocks_batch()` for better performance
2. **Incremental Sync**: Use `sync_to_latest()` to avoid reprocessing
3. **Error Recovery**: Use `reindex_from_height()` for recovery scenarios
4. **Regular Backups**: SQLite files can be backed up while running (WAL mode)
5. **Monitoring**: Check statistics regularly to track indexer health

## Future Enhancements

- [ ] WebSocket notifications for new blocks
- [ ] Advanced filtering and search
- [ ] Export to CSV/JSON
- [ ] GraphQL API layer
- [ ] Multi-node support
- [ ] Pruning old data
- [ ] Custom event handlers

## License

Apache-2.0
