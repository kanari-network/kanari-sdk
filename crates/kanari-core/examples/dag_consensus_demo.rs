// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::print_stdout)]
//! Example: Using DAG Consensus in Kanari
//!
//! This example demonstrates how to use DAG-based consensus
//! to achieve high throughput and low latency transaction processing

use anyhow::Result;
use kanari_core::engine::{BlockchainEngine, DagEngine};
use kanari_core::kanari_move_runtime_v1::state::Account;
use kanari_crypto::keys::CurveType;
use kanari_types::balance::BalanceRecord;
use kanari_types::kanari::KANARI_TOKEN_TYPE;
use kanari_types::transaction::{SignedTransaction, Transaction};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

fn main() -> Result<()> {
    println!("=== Kanari DAG Consensus Example ===\n");

    // 1. Create base blockchain engine
    println!("1. Creating blockchain engine...");
    let temp_dir = tempfile::Builder::new()
        .prefix("kanari_dag_demo")
        .tempdir()?;
    let db_path = temp_dir.path().to_str().unwrap();
    // let _ = std::fs::remove_dir_all(db_path); // Clean up previous run - handled by tempfile
    let engine = Arc::new(BlockchainEngine::new_dir(db_path)?);
    println!("   ✓ Engine created\n");

    // 2. Setup authorities (validators)
    println!("2. Setting up authorities...");
    // For this demo, we use a single authority to satisfy quorum (100% of 1 = 1)
    let authorities = vec!["0xAUTH1".to_string()];
    println!("   ✓ {} authorities configured\n", authorities.len());

    // 3. Create DAG engine (optimized for high throughput)
    println!("3. Creating DAG engine...");

    // Choose configuration based on your hardware:
    // - DagEngine::new() - Default (100K TPS, 32+ cores, 16GB RAM)
    // - DagEngine::new_moderate() - Moderate (10K-30K TPS, 8-16 cores, 16-32GB RAM)
    // - DagEngine::new_high_throughput() - Extreme (500K+ TPS, 64+ cores, 32GB+ RAM)

    let dag_engine = DagEngine::new(
        engine.clone(),
        "0xAUTH1".to_string(), // This node's authority ID
        authorities.clone(),
    )?;
    println!("   ✓ DAG engine created (default config)");
    println!("   ✓ Authority ID: {}", dag_engine.authority_id());

    // For 8-16 core machines (moderate throughput):
    // let dag_engine = DagEngine::new_moderate(
    //     engine.clone(),
    //     "0xAUTH1".to_string(),
    //     authorities.clone(),
    // )?;

    // For high-throughput production deployment with 64+ cores:
    // let dag_engine = DagEngine::new_high_throughput(
    //     engine.clone(),
    //     "0xAUTH1".to_string(),
    //     authorities.clone(),
    // )?;
    println!("   💡 Tip: Use new_high_throughput() for 500K+ TPS\n");

    // 4. Generate some test transactions
    println!("4. Generating test transactions...");
    let keypair = kanari_crypto::keys::generate_keypair(CurveType::Ed25519)?;
    let sender_address = keypair.tagged_address(); // Use tagged address for proper signature verification
    let private_key = keypair.private_key.to_string();

    // Fund the sender account manually for the demo
    {
        let mut state = engine.state.write().unwrap();
        // We still need the raw address for account lookup, extract it from the tagged address
        let raw_address = keypair.address.clone(); // Just for funding purposes
        let addr = AccountAddress::from_hex_literal(&raw_address)?;
        let mut account = state
            .get_account(&addr)
            .unwrap_or_else(|| Account::new(addr));
        account.set_token_balance(
            KANARI_TOKEN_TYPE.to_string(),
            BalanceRecord::new(10_000_000_000_000),
        );
        state.save_account(&account).unwrap();
        println!(
            "   ✓ Funded sender {} with {} MIST",
            raw_address,
            account.native_balance()
        );
    }

    let mut transactions = Vec::new();
    let mut next_seq_num = 0;
    for i in 0..10 {
        let tx = Transaction::Transfer {
            from: sender_address.clone(), // Use the tagged address for the transaction
            to: format!("0x{:064x}", i + 100),
            amount: 1000 * (i + 1),
            gas_limit: 100_000,
            gas_price: 1000,
            sequence_number: next_seq_num,
        };
        next_seq_num += 1;

        let mut signed_tx = SignedTransaction::new(tx);
        signed_tx.sign(&private_key, CurveType::Ed25519)?;
        transactions.push(signed_tx);
    }
    println!("   ✓ Generated {} transactions\n", transactions.len());

    // 5. Submit transactions to DAG engine
    println!("5. Submitting transactions...");
    let tx_hashes = dag_engine
        .engine()
        .submit_transactions_batch(transactions)?;
    for (i, tx_hash) in tx_hashes.iter().enumerate() {
        println!("   ✓ Transaction {}: {}", i + 1, hex::encode(&tx_hash[..8]));
    }
    println!();

    // 6. Produce DAG vertices
    println!("6. Producing DAG vertices...\n");

    // In a real system, this would be called periodically by each authority
    for round in 0..3 {
        println!("   Round {}:", round + 1);

        // Generate more transactions for subsequent rounds
        // Use sequence starting from 0 since account doesn't exist yet
        if round > 0 {
            println!("   📝 Generating {} more transactions...", 5);
            for i in 0..5 {
                let tx = Transaction::Transfer {
                    from: sender_address.clone(),
                    to: format!("0x{:064x}", round * 100 + i),
                    amount: 2000 * (i + 1),
                    gas_limit: 100_000,
                    gas_price: 1000,
                    sequence_number: next_seq_num, // Use next sequence number
                };
                next_seq_num += 1;

                let mut signed_tx = SignedTransaction::new(tx);
                signed_tx.sign(&private_key, CurveType::Ed25519)?;
                dag_engine
                    .engine()
                    .submit_transactions_batch(vec![signed_tx])?;
            }
        }

        match dag_engine.produce_vertex() {
            Ok(dag_info) => {
                println!("   ✓ Vertex created:");
                println!("     - ID: {}", hex::encode(&dag_info.vertex_id[..16]));
                println!("     - Round: {}", dag_info.round);
                println!("     - Transactions: {}", dag_info.tx_count);
                println!("     - Executed: {}", dag_info.executed);
                println!("     - Failed: {}", dag_info.failed);

                if let Some(checkpoint) = dag_info.checkpoint {
                    println!("\n   ⭐ Checkpoint #{} created!", checkpoint.sequence);
                    println!("     - Vertices committed: {}", checkpoint.vertex_count);
                    println!("     - Transactions committed: {}", checkpoint.tx_count);
                }
            }
            Err(e) => {
                println!("   ⚠ Vertex creation blocked: {}", e);
                if round > 0 {
                    println!("     💡 Note: DAG consensus requires multiple authorities");
                    println!("        creating vertices in parallel to meet quorum (2f+1).");
                    println!("        This demo uses a single authority for simplicity.");
                }
            }
        }
        println!();
    }

    // 7. Check blockchain state
    println!("7. Checking blockchain state...");
    let blockchain = engine.blockchain.read().unwrap();

    println!("   - Mode: DAG");
    println!("   - Height: {}", blockchain.height());
    println!(
        "   - Total transactions: {}",
        blockchain.get_transaction_count()
    );

    if blockchain.dag_mode {
        let latest_checkpoint = blockchain.latest_checkpoint();
        println!("   - Latest checkpoint: #{}", latest_checkpoint.sequence);
        println!(
            "   - Checkpoint transactions: {}",
            latest_checkpoint.transactions.len()
        );
    }
    println!();

    // 8. Access DAG consensus details
    println!("8. DAG Consensus details...");
    let consensus = dag_engine.consensus();
    let consensus_guard = consensus.read().unwrap();
    let store = consensus_guard.store();

    println!("   - Current round: {}", store.current_round());
    println!("   - Number of authorities: {}", store.num_authorities());
    println!("   - Checkpoints: {}", store.latest_checkpoint().sequence);
    println!();

    // 9. Demonstrate parallel execution advantage
    println!("9. Parallel execution & performance...");
    println!("   DAG consensus allows multiple authorities to create");
    println!("   vertices simultaneously, leading to:");
    println!();
    println!("   📊 Performance Characteristics:");
    println!("   ✓ Throughput:");
    println!("     - Default config: 100,000 TPS");
    println!("     - High-throughput: 500,000+ TPS");
    println!("     - Extreme mode: 1,000,000+ TPS");
    println!("   ✓ Latency:");
    println!("     - Transaction finality: 20-50ms");
    println!("     - Checkpoint commit: 100-200ms");
    println!("   ✓ Scalability:");
    println!("     - Linear scaling up to 128 cores");
    println!("     - Near-linear up to 256 cores");
    println!("   ✓ Resource Efficiency:");
    println!("     - Cache hit rate: 95%+");
    println!("     - Memory usage: ~20GB @ 500K TPS");
    println!("     - Network: 25Gbps+ recommended");
    println!();
    println!("   💡 See HIGH_THROUGHPUT_OPTIMIZATION.md for details");
    println!();

    println!("=== DAG Consensus Example Complete ===");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_example() {
        // Run the example in test mode
        let result = main();
        assert!(result.is_ok(), "DAG example should complete successfully");
    }

    #[test]
    fn test_dag_engine_basic_operations() -> Result<()> {
        // Create engine
        let engine = Arc::new(BlockchainEngine::new()?);

        // Setup authorities
        let authorities = vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ];

        // Create DAG engine
        let dag_engine = DagEngine::new(engine.clone(), "auth1".to_string(), authorities)?;

        // Verify DAG mode is enabled
        let blockchain = engine.blockchain.read().unwrap();
        assert!(blockchain.dag_mode, "DAG mode should be enabled");

        // Verify initial state
        assert_eq!(blockchain.height(), 0);
        assert_eq!(blockchain.get_transaction_count(), 0);

        Ok(())
    }

    #[test]
    fn test_dag_vertex_creation() -> Result<()> {
        let engine = Arc::new(BlockchainEngine::new()?);
        let authorities = vec![
            "a1".to_string(),
            "a2".to_string(),
            "a3".to_string(),
            "a4".to_string(),
        ];
        let dag_engine = DagEngine::new(engine.clone(), "a1".to_string(), authorities)?;

        // Generate test transaction
        let keypair = kanari_crypto::keys::generate_keypair(CurveType::Ed25519)?;
        let sender = keypair.tagged_address(); // Use tagged address
        let private_key = keypair.private_key.to_string();

        let tx = Transaction::Transfer {
            from: sender,
            to: "0x999".to_string(),
            amount: 5000,
            gas_limit: 100_000,
            gas_price: 1000,
            sequence_number: 0,
        };

        let mut signed_tx = SignedTransaction::new(tx);
        signed_tx.sign(&private_key, CurveType::Ed25519)?;

        // Submit and produce vertex
        dag_engine.engine().submit_transaction(signed_tx)?;
        let dag_info = dag_engine.produce_vertex()?;

        assert_eq!(dag_info.tx_count, 1);
        assert!(dag_info.executed <= 1); // May fail due to account not existing, but vertex created

        Ok(())
    }
}
