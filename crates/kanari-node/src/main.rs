// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Main entry point for Kanari blockchain node
use anyhow::Result;
use clap::{Parser, Subcommand};
use kanari_core::BlockchainEngine;
use kanari_crypto::wallet::list_wallet_files;
use kanari_rpc_server::start_server;
use kanari_types::address::Address as KanariAddress;
use kanari_types::kanari::KanariModule;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Kanari node command-line interface
#[derive(Parser)]
#[command(name = "kanari-node")]
#[command(about = "Kanari blockchain node", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// start the node
    Start,
    /// List wallet files
    ListWallets,
    /// Show blockchain statistics
    Stats,
    /// Get account info
    Account {
        /// Account address
        address: String,
    },
    /// Get block information by height
    Block {
        /// Block height
        height: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ListWallets => {
            let wallets = list_wallet_files()?;
            for (addr, selected) in wallets {
                tracing::info!("{}{}", addr, if selected { " (selected)" } else { "" });
            }
            return Ok(());
        }

        Commands::Stats => {
            let engine = BlockchainEngine::new()?;
            let stats = engine.get_stats();
            tracing::info!("Blockchain Statistics:");
            tracing::info!("  Height: {}", stats.height);
            tracing::info!("  Total Blocks: {}", stats.total_blocks);
            tracing::info!("  Total Transactions: {}", stats.total_transactions);
            tracing::info!("  Pending Transactions: {}", stats.pending_transactions);
            tracing::info!("  Total Accounts: {}", stats.total_accounts);
            tracing::info!("  Total Supply: {} Kanari", stats.total_supply);
            return Ok(());
        }

        Commands::Account { address } => {
            let engine = BlockchainEngine::new()?;
            match engine.get_account_info(&address) {
                Some(info) => {
                    tracing::info!("  Account: {}", info.address);
                    tracing::info!("  Balance: {}", info.balance);
                    tracing::info!("  Sequence: {}", info.sequence_number);
                    tracing::info!("  Modules: {}", info.modules.len());
                    for module in &info.modules {
                        tracing::info!("    - {}", module);
                    }
                }
                None => tracing::info!("Account not found: {}", address),
            }
            return Ok(());
        }

        Commands::Block { height } => {
            let engine = BlockchainEngine::new()?;
            match engine.get_block(height) {
                Some(block) => {
                    tracing::info!("  Block #{}", block.height);
                    tracing::info!("  Timestamp: {}", block.timestamp);
                    tracing::info!("  Hash: {}", block.hash);
                    tracing::info!("  Prev Hash: {}", block.prev_hash);
                    tracing::info!("  Transactions: {}", block.tx_count);
                }
                None => tracing::info!("Block not found: {}", height),
            }
            return Ok(());
        }

        Commands::Start => {
            let engine = BlockchainEngine::new()?;
            let engine_arc = Arc::new(engine);
            run_node(engine_arc).await?;
            return Ok(());
        }
    }
}

async fn run_node(engine: Arc<BlockchainEngine>) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Wrap engine in Arc for sharing between tasks

    let stats = engine.get_stats();

    tracing::info!("Kanari blockchain node starting");
    tracing::info!("Network: Testnet, Move VM: Enabled");
    tracing::info!("Initial blockchain height: {}", stats.height);
    let total_supply_str = KanariModule::format_kanari(stats.total_supply);
    tracing::info!(
        "Total accounts: {}, Total supply: {}",
        stats.total_accounts,
        total_supply_str
    );

    // Get genesis/root object state from block 0 if available
    let genesis_root = match engine.get_block(0) {
        Some(b) => b.hash,
        None => "unknown".to_string(),
    };
    let size_bytes = if genesis_root == "unknown" {
        0
    } else {
        genesis_root.len() / 2
    };
    let dao_addr = KanariAddress::DAO_ADDRESS;
    tracing::info!(
        "The latest Root object state root: 0x{}, size: {} bytes",
        genesis_root,
        size_bytes
    );
    tracing::info!("DAO address: ({})", dao_addr);

    // Get RPC server sequencer / dev address from kanari-types constants
    let dev_addr = KanariAddress::DEV_ADDRESS;
    tracing::info!("RPC Server sequencer address: ({})", dev_addr);

    tracing::info!("kanari_sequencer::actor::sequencer: Load latest sequencer order 0");
    tracing::info!("kanari_sequencer::actor::sequencer: Load latest sequencer order 0");

    // Start RPC server in background with cloned Arc
    let rpc_addr = "127.0.0.1:19001";
    tracing::info!("Starting RPC server on http://{}", rpc_addr);

    let engine_for_rpc = engine.clone();
    tokio::spawn(async move {
        if let Err(e) = start_server(engine_for_rpc, rpc_addr).await {
            tracing::error!("RPC server error: {}", e);
        }
    });

    // Wait for server to start
    sleep(Duration::from_millis(500)).await;
    tracing::info!("RPC server ready");

    let mut _tick: u64 = 0;
    loop {
        _tick += 1;
        let stats = engine.get_stats();
        let wallets = list_wallet_files().unwrap_or_default();
        tracing::info!(
            "Block height: {}, Transactions: {}, Pending: {}, Accounts: {}, Wallets: {}",
            stats.height,
            stats.total_transactions,
            stats.pending_transactions,
            stats.total_accounts,
            wallets.len()
        );

        // Try to produce block if there are pending transactions
        if stats.pending_transactions > 0 {
            match engine.produce_block() {
                Ok(block_info) => {
                    tracing::info!(
                        "Block #{} produced: {} txs ({} executed, {} failed)",
                        block_info.height,
                        block_info.tx_count,
                        block_info.executed,
                        block_info.failed
                    );
                }
                Err(e) => {
                    tracing::error!("Block production failed: {}", e);
                }
            }
        }

        sleep(Duration::from_secs(5)).await;
    }
}
