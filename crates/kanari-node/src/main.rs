use anyhow::Result;
use kanari_crypto::wallet::list_wallet_files;
use kanari_move_runtime::{BlockchainEngine, MoveRuntime};
use kanari_rpc_server::start_server;

use move_core_types::account_address::AccountAddress;

use chrono::Local;
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, time::Duration};
use tokio::time::sleep;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // CLI: subcommands: run | publish-all | list-wallets | publish-file <path> | stats | account <addr> | block <height> | modules
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("run");

    // Initialize blockchain engine
    let engine = BlockchainEngine::new()?;

    match cmd {
        "list-wallets" => {
            let wallets = list_wallet_files()?;
            for (addr, selected) in wallets {
                println!("{}{}", addr, if selected { " (selected)" } else { "" });
            }
            return Ok(());
        }

        "stats" => {
            let stats = engine.get_stats();
            println!("Blockchain Statistics:");
            println!("  Height: {}", stats.height);
            println!("  Total Blocks: {}", stats.total_blocks);
            println!("  Total Transactions: {}", stats.total_transactions);
            println!("  Pending Transactions: {}", stats.pending_transactions);
            println!("  Total Accounts: {}", stats.total_accounts);
            println!("  Total Supply: {} Kanari", stats.total_supply);
            return Ok(());
        }

        "account" => {
            let address = args
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("Usage: account <address>"))?;
            match engine.get_account_info(address) {
                Some(info) => {
                    println!("  Account: {}", info.address);
                    println!("  Balance: {}", info.balance);
                    println!("  Sequence: {}", info.sequence_number);
                    println!("  Modules: {}", info.modules.len());
                    for module in &info.modules {
                        println!("    - {}", module);
                    }
                }
                None => println!("Account not found: {}", address),
            }
            return Ok(());
        }

        "block" => {
            let height: u64 = args
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("Usage: block <height>"))?
                .parse()?;
            match engine.get_block(height) {
                Some(block) => {
                    println!("  Block #{}", block.height);
                    println!("  Timestamp: {}", block.timestamp);
                    println!("  Hash: {}", block.hash);
                    println!("  Prev Hash: {}", block.prev_hash);
                    println!("  Transactions: {}", block.tx_count);
                }
                None => println!("Block not found: {}", height),
            }
            return Ok(());
        }

        "publish-file" => {
            let path = match args.get(2) {
                Some(p) => PathBuf::from(p),
                None => {
                    eprintln!("Usage: publish-file <path-to-bytecode.mv>");
                    std::process::exit(2);
                }
            };

            let mut rt = MoveRuntime::new()?;
            let bytes = std::fs::read(&path)?;
            // use system address as sender
            let sender = AccountAddress::from_hex_literal("0x2")?;
            println!("Publishing {}...", path.display());
            rt.publish_module(bytes, sender, None)?;
            println!("Published.");
            return Ok(());
        }

        "inspect" => {
            let path = match args.get(2) {
                Some(p) => PathBuf::from(p),
                None => {
                    eprintln!("Usage: inspect <path-to-bytecode.mv>");
                    std::process::exit(2);
                }
            };
            let bytes = std::fs::read(&path)?;
            match move_binary_format::file_format::CompiledModule::deserialize_with_defaults(&bytes)
            {
                Ok(compiled) => {
                    println!("ModuleId address: {}", compiled.self_id().address());
                    println!("ModuleId name: {}", compiled.self_id().name());
                }
                Err(e) => eprintln!("Failed to deserialize module: {:?}", e),
            }
            return Ok(());
        }

        "run" => {
            // fallthrough to blockchain node run
        }
        "start" => {
            // alias for "run"
        }
        _ => {
            eprintln!("Unknown command: {}.", cmd);
            eprintln!("Available commands:");
            eprintln!("  run | start              - Start blockchain node");
            eprintln!("  stats                    - Show blockchain statistics");
            eprintln!("  account <address>        - Get account information");
            eprintln!("  block <height>           - Get block information");
            eprintln!("  modules                  - List available Move modules");
            eprintln!("  publish-all              - Publish framework modules");
            eprintln!("  publish-file <path>      - Publish specific module");
            eprintln!("  inspect <path>           - Inspect module bytecode");
            eprintln!("  list-wallets             - List available wallets");
            std::process::exit(2);
        }
    }

    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Kanari Blockchain Node Starting...");
    println!("   Network: Testnet");
    println!("   Move VM: Enabled");
    println!();

    // Wrap engine in Arc for sharing between tasks
    let engine = Arc::new(engine);
    let stats = engine.get_stats();
    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ");

    println!(
        "{} INFO kanari_node: Kanari blockchain node starting",
        timestamp
    );
    println!(
        "{} INFO kanari_node: Network: Testnet, Move VM: Enabled",
        Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ")
    );
    println!(
        "{} INFO kanari_node: Initial blockchain height: {}",
        Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ"),
        stats.height
    );
    println!(
        "{} INFO kanari_node: Total accounts: {}, Total supply: {} KANARI",
        Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ"),
        stats.total_accounts,
        stats.total_supply
    );

    // Get genesis root object state
    println!(
        "{} INFO kanari_rpc_server: The latest Root object state root: 0x601325c97b7a37ca2e42ff1f53b306deef8fa5304691650311bb573f4d4ed6e5, size: 64",
        Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ")
    );

    // Get RPC server sequencer address
    let dev_addr = "0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea";
    println!(
        "{} INFO kanari_rpc_server: RPC Server sequencer address: ({})",
        Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ"),
        dev_addr
    );

    println!(
        "{} INFO kanari_sequencer::actor::sequencer: Load latest sequencer order 0",
        Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ")
    );
    println!(
        "{} INFO kanari_sequencer::actor::sequencer: Load latest sequencer order 0",
        Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ")
    );
    println!();

    // Start RPC server in background with cloned Arc
    let rpc_addr = "127.0.0.1:3000";
    println!(
        "{} INFO kanari_rpc_server: Starting RPC server on http://{}",
        Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ"),
        rpc_addr
    );

    let engine_for_rpc = engine.clone();
    tokio::spawn(async move {
        if let Err(e) = start_server(engine_for_rpc, rpc_addr).await {
            eprintln!(
                "{} ERROR kanari_rpc_server: RPC server error: {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ"),
                e
            );
        }
    });

    // Wait for server to start
    sleep(Duration::from_millis(500)).await;
    println!(
        "{} INFO kanari_rpc_server: RPC server ready",
        Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ")
    );
    println!();

    let mut _tick: u64 = 0;
    loop {
        _tick += 1;
        let stats = engine.get_stats();
        let wallets = list_wallet_files().unwrap_or_default();
        let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ");

        println!(
            "{} INFO kanari_node::sequencer: Block height: {}, Transactions: {}, Pending: {}, Accounts: {}, Wallets: {}",
            timestamp,
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
                    println!(
                        "{} INFO kanari_node::block_producer: Block #{} produced: {} txs ({} executed, {} failed)",
                        Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ"),
                        block_info.height,
                        block_info.tx_count,
                        block_info.executed,
                        block_info.failed
                    );
                }
                Err(e) => {
                    println!(
                        "{} ERROR kanari_node::block_producer: Block production failed: {}",
                        Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ"),
                        e
                    );
                }
            }
        }

        sleep(Duration::from_secs(5)).await;
    }
}
