# Quick Start Guide - Kanari Indexer

Get started with the Kanari Indexer in 5 minutes!

## Installation

The indexer is already part of the Kanari SDK workspace. No additional installation needed!

## Basic Usage

### 1. Create an Indexer

```rust
use kanari_indexer::{Indexer, IndexerConfig};
use std::path::PathBuf;

let config = IndexerConfig {
    db_path: PathBuf::from("my_indexer.db"),
    in_memory: false,
    batch_size: 100,
};

let indexer = Indexer::new(config)?;
```

### 2. Index Blocks

```rust
// Index a single block
indexer.index_block(&block)?;

// Or sync from a node
indexer.sync_to_latest(|height| {
    // Your function to fetch blocks
    fetch_block_from_node(height)
})?;
```

### 3. Query Data

```rust
// Get transactions by sender
let txs = indexer.db()
    .get_transactions_by_sender("0x123...", 10)?;

// Get owner balances
let balances = indexer.db()
    .get_all_owner_balances("0x123...")?;

// Get statistics
let stats = indexer.get_statistics()?;
println!("{}", stats);
```

## Run Examples

```bash
# Basic usage example
cargo run -p kanari-indexer --example basic_usage

# Integration example
cargo run -p kanari-indexer --example integration_example

# Query examples
cargo run -p kanari-indexer --example query_examples
```

## Common Operations

### Indexing

```rust
// Single block
indexer.index_block(&block)?;

// Batch of blocks
indexer.index_blocks_batch(&blocks)?;

// Sync to latest
let count = indexer.sync_to_latest(fetch_fn)?;
```

### Queries

```rust
// Blocks
let block = indexer.db().get_block_by_height(100)?;
let latest = indexer.db().get_latest_height()?;

// Transactions
let tx = indexer.db().get_transaction_by_hash(&hash)?;
let txs = indexer.db().get_transactions_by_block(100)?;

// Events
let events = indexer.db().get_events_by_transaction(&tx_hash)?;

// Coins & Balances
let coins = indexer.db().get_coins_by_owner(addr)?;
let balance = indexer.db().get_owner_balance(addr, "KANARI")?;
```

### Analytics

```rust
let stats = indexer.get_statistics()?;
println!("Blocks: {}", stats.block_count);
println!("Transactions: {}", stats.transaction_count);
println!("Events: {}", stats.event_count);
```

## Testing

```bash
# Run all tests
cargo test -p kanari-indexer

# Run with output
cargo test -p kanari-indexer -- --nocapture
```

## Database Location

By default, the database is stored at the path specified in `IndexerConfig.db_path`.

For production:

```rust
let config = IndexerConfig {
    db_path: PathBuf::from("/var/lib/kanari/indexer.db"),
    in_memory: false,
    batch_size: 100,
};
```

For testing:

```rust
let config = IndexerConfig {
    db_path: PathBuf::from("test.db"),
    in_memory: true,  // Uses in-memory database
    batch_size: 10,
};
```

## Performance Tips

1. **Use batch operations** for better throughput
2. **Enable WAL mode** (enabled by default)
3. **Adjust cache size** if you have more memory
4. **Use prepared queries** for repeated operations
5. **Monitor statistics** regularly

## Need Help?

- Check the [README.md](README.md) for detailed documentation
- See [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) for architecture details
- Review the examples in the `examples/` directory
- Check the inline code documentation

## Next Steps

1. ✅ Set up your indexer
2. ✅ Start indexing blocks
3. ✅ Query and analyze data
4. 🚀 Build your application on top of the indexed data!

Happy indexing! 🎉
