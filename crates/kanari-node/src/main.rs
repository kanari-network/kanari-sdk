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
use libp2p::identity::Keypair;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

mod p2p;
mod peer_store;
mod sync;

use p2p::{P2PEventHandler, P2PMessage, P2PNetwork};
use peer_store::PeerStore;
use sync::SyncManager;

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
    Start {
        /// P2P listen port
        #[arg(long, default_value = "19000")]
        p2p_port: u16,

        /// RPC listen port
        #[arg(long, default_value = "19001")]
        rpc_port: u16,

        /// RPC listen host/IP (use 0.0.0.0 to bind all interfaces)
        #[arg(long, default_value = "0.0.0.0")]
        rpc_host: String,

        /// Data directory for blockchain and state storage
        #[arg(long)]
        data_dir: Option<std::path::PathBuf>,
    },
    /// Run a local-only node
    Local {},
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

        Commands::Start {
            p2p_port,
            rpc_port,
            rpc_host,
            data_dir,
            // bootstrap,
        } => {
            let data_dir_path = data_dir.clone().unwrap_or_else(|| {
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_else(|_| ".".to_string());
                std::path::PathBuf::from(home)
                    .join(".kanari")
                    .join("kanari-db")
            });

            if let Some(ref d) = data_dir {
                unsafe {
                    std::env::set_var("KANARI_STATE_DB", d);
                }
                tracing::info!("Using data directory: {}", d.display());
            }

            let engine = BlockchainEngine::new()?;
            let engine_arc = Arc::new(engine);

            run_node(
                engine_arc,
                p2p_port,
                rpc_port,
                rpc_host,
                data_dir_path,
                false,
            )
            .await?;
            return Ok(());
        }
        Commands::Local {} => {
            // Run a local-only node: RPC bound to localhost, P2P disabled
            let p2p_port = 0;
            let rpc_port = 19001;
            let rpc_host = "127.0.0.1".to_string();

            let data_dir_path = std::path::PathBuf::from("./.kanari-local");
            tracing::info!(
                "Starting local node: RPC on {}:{} (P2P disabled)",
                rpc_host,
                rpc_port
            );

            let engine = BlockchainEngine::new()?;
            let engine_arc = Arc::new(engine);

            run_node(
                engine_arc,
                p2p_port,
                rpc_port,
                rpc_host,
                data_dir_path,
                true,
            )
            .await?;
            return Ok(());
        }
    }
}

fn detect_local_ip() -> Option<String> {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .ok()
        .and_then(|socket| {
            // connect to a public IP to determine the outbound interface
            if socket.connect("8.8.8.8:80").is_ok() {
                socket.local_addr().ok().map(|a| a.ip().to_string())
            } else {
                None
            }
        })
}

async fn run_node(
    engine: Arc<BlockchainEngine>,
    p2p_port: u16,
    rpc_port: u16,
    rpc_host: String,
    data_dir: std::path::PathBuf,
    disable_p2p: bool,
) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

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

    // Create channels for P2P message handling (used even in local mode, messages will be dropped)
    let (p2p_msg_tx, mut p2p_msg_rx) = tokio::sync::mpsc::unbounded_channel::<P2PMessage>();
    let (network_tx, network_rx) = tokio::sync::mpsc::unbounded_channel::<P2PMessage>();

    // Create sync manager
    let sync_manager = Arc::new(SyncManager::new(engine.clone(), network_tx.clone()));

    if !disable_p2p {
        // Load or create peer store
        let peer_store_path = PeerStore::default_path(&data_dir.display().to_string());
        let mut peer_store = PeerStore::load(peer_store_path.clone()).unwrap_or_else(|e| {
            tracing::warn!("Failed to load peer store: {}, creating new one", e);
            PeerStore::new(peer_store_path)
        });

        // Clean up old peers (older than 7 days)
        peer_store.cleanup_old_peers(7 * 24 * 60 * 60);

        // Initialize P2P network
        let keypair = Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id().to_string();
        tracing::info!("Node Peer ID: {}", peer_id);

        let p2p_network = P2PNetwork::new(keypair, p2p_port)?;
        tracing::info!("P2P network initialized on port {}", p2p_port);

        // Wrap peer store in Arc<Mutex> for sharing
        let peer_store_arc = Arc::new(tokio::sync::Mutex::new(peer_store));

        // Start P2P event handler with both incoming and outgoing message channels + peer store
        let mut event_handler = P2PEventHandler::new(p2p_network, p2p_msg_tx)
            .with_outgoing(network_rx)
            .with_peer_store(peer_store_arc.clone());
        tokio::spawn(async move {
            event_handler.run().await;
        });

        // Handle P2P messages from network
        let sync_for_messages = sync_manager.clone();
        tokio::spawn(async move {
            while let Some(msg) = p2p_msg_rx.recv().await {
                sync_for_messages.handle_message(msg).await;
            }
        });

        // Broadcast peer info periodically
        let sync_for_broadcast = sync_manager.clone();
        let peer_id_clone = peer_id.clone();
        tokio::spawn(async move {
            // Wait a bit for peer discovery to complete before first broadcast
            sleep(Duration::from_secs(3)).await;
            loop {
                sync_for_broadcast
                    .broadcast_peer_info(peer_id_clone.clone())
                    .await;
                sleep(Duration::from_secs(30)).await;
            }
        });
    } else {
        tracing::info!("Running in local-only mode: P2P disabled");
    }

    // Start RPC server in background with cloned Arc
    let bind_addr = format!("{}:{}", rpc_host, rpc_port);
    tracing::info!("Binding RPC server to {}", bind_addr);

    // If binding to all interfaces, try to detect a representative local IP for display
    let display_ip = if rpc_host == "0.0.0.0" {
        detect_local_ip().unwrap_or_else(|| "127.0.0.1".to_string())
    } else {
        rpc_host.clone()
    };
    tracing::info!("Starting RPC server on http://{}:{}", display_ip, rpc_port);

    let engine_for_rpc = engine.clone();
    let bind_addr_clone = bind_addr.clone();
    tokio::spawn(async move {
        if let Err(e) = start_server(engine_for_rpc, &bind_addr_clone).await {
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

                    // Broadcast the new block with full transaction data to the network
                    if let Some(full_block_data) = engine.get_full_block(block_info.height)
                        && let Ok(block_str) = serde_json::to_string(&full_block_data)
                    {
                        let msg = P2PMessage::NewBlock(block_str);
                        if let Err(e) = network_tx.send(msg) {
                            tracing::warn!("Failed to queue block broadcast: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Block production failed: {}", e);
                }
            }
        }

        sleep(Duration::from_secs(5)).await;
    }
}
