use anyhow::Result;
use kanari_move_runtime::{BlockchainEngine, SignedTransaction, Transaction};

fn main() -> Result<()> {
    println!("=== Kanari Blockchain Demo ===\n");

    // Initialize blockchain engine
    let engine = BlockchainEngine::new()?;

    // Show initial stats
    println!("📊 Initial Blockchain State:");
    let stats = engine.get_stats();
    println!("  Height: {}", stats.height);
    println!("  Total Blocks: {}", stats.total_blocks);
    println!("  Total Accounts: {}", stats.total_accounts);
    println!("  Total Supply: {} Kanari", stats.total_supply);
    println!();

    // Get genesis block
    println!("🔗 Genesis Block:");
    if let Some(block) = engine.get_block(0) {
        println!("  Height: {}", block.height);
        println!("  Timestamp: {}", block.timestamp);
        println!("  Hash: {}", block.hash);
        println!("  Transactions: {}", block.tx_count);
    }
    println!();

    // Show system accounts
    println!("👤 System Accounts:");
    for addr in &["0x1", "0x2"] {
        if let Some(account) = engine.get_account_info(addr) {
            println!(
                "  {} - Balance: {}, Seq: {}",
                account.address, account.balance, account.sequence_number
            );
        }
    }
    println!();

    // Submit some test transactions
    println!("📝 Submitting Test Transactions...");

    // Mint coins to test account
    let tx1 = Transaction::new_transfer("0x2".to_string(), "0x1".to_string(), 1000);
    let signed_tx1 = SignedTransaction::new(tx1);
    let tx1_hash = engine.submit_transaction(signed_tx1)?;
    println!(
        "  ✅ Transfer transaction submitted: {}",
        hex::encode(&tx1_hash[..8])
    );

    let tx2 = Transaction::new_transfer("0x2".to_string(), "0x123".to_string(), 500);
    let signed_tx2 = SignedTransaction::new(tx2);
    let tx2_hash = engine.submit_transaction(signed_tx2)?;
    println!(
        "  ✅ Transfer transaction submitted: {}",
        hex::encode(&tx2_hash[..8])
    );
    println!();

    // Check pending transactions
    let stats = engine.get_stats();
    println!("📊 After Submission:");
    println!("  Pending Transactions: {}", stats.pending_transactions);
    println!();

    // Produce a block
    println!("⛏️  Mining Block...");
    let block_info = engine.produce_block()?;
    println!("  ✅ Block #{} produced!", block_info.height);
    println!("     Hash: {}", block_info.hash);
    println!("     Transactions: {}", block_info.tx_count);
    println!("     Executed: {}", block_info.executed);
    println!("     Failed: {}", block_info.failed);
    println!();

    // Check final state
    println!("📊 Final Blockchain State:");
    let stats = engine.get_stats();
    println!("  Height: {}", stats.height);
    println!("  Total Blocks: {}", stats.total_blocks);
    println!("  Total Transactions: {}", stats.total_transactions);
    println!("  Pending Transactions: {}", stats.pending_transactions);
    println!("  Total Accounts: {}", stats.total_accounts);
    println!();

    // Show updated accounts
    println!("👤 Updated Accounts:");
    for addr in &["0x1", "0x2", "0x123"] {
        if let Some(account) = engine.get_account_info(addr) {
            println!(
                "  {} - Balance: {}, Seq: {}, Modules: {}",
                account.address,
                account.balance,
                account.sequence_number,
                account.modules.len()
            );
        }
    }
    println!();

    // Get the new block
    println!("🔗 Latest Block:");
    if let Some(block) = engine.get_block(stats.height) {
        println!("  Height: {}", block.height);
        println!("  Timestamp: {}", block.timestamp);
        println!("  Hash: {}", block.hash);
        println!("  Prev Hash: {}", block.prev_hash);
        println!("  Transactions: {}", block.tx_count);
    }

    Ok(())
}
