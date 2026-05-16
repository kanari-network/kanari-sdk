// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Main entry point for Kanari blockchain node
use anyhow::Result;
use clap::{Parser, Subcommand};
use kanari_core::BlockchainEngine;
use kanari_core::engine::AccountInfo;
use kanari_crypto::wallet::list_wallet_files;
use kanari_rpc_server::start_server;
use kanari_types::address::Address as KanariAddress;
use kanari_types::kanari::{KANARI_TOKEN_TYPE, KanariModule};
use libp2p::identity::Keypair;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

mod indexer;
mod p2p;
mod peer_store;
mod sync;

use indexer::NodeIndexer;
use p2p::{P2PEventHandler, P2PMessage, P2PNetwork};
use peer_store::PeerStore;
use sync::SyncManager;

/// Kanari node command-line interface
#[derive(Parser)]
#[command(name = "kanari-node", about = "Kanari run server")]
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
        /// Run as relay server to help other nodes behind NAT
        #[arg(long, default_value = "false")]
        relay_server: bool,
        /// Authority ID for DAG consensus (e.g. 0x1)
        #[arg(long)]
        authority_id: Option<String>,
        /// List of authority IDs for DAG consensus (comma-separated)
        #[arg(long, value_delimiter = ',')]
        authorities: Option<Vec<String>>,
        /// Bootstrap peer multiaddr to connect to (can be specified multiple times)
        #[arg(long, value_name = "MULTIADDR")]
        bootstrap: Option<Vec<String>>,
    },
    /// Run a local-only node
    Local,
    /// List wallet files
    ListWallets,
    /// Show blockchain statistics
    Stats,
    /// Get account info
    Account { address: String },
    /// Get block information by height
    Block { height: u64 },
}

// Main entry point
// Initializes and runs the Kanari blockchain node
// Sets up P2P networking, RPC server, and blockchain engine
fn default_data_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home)
        .join(".kanari")
        .join("kanari-db")
}

fn create_engine(data_dir: &Option<std::path::PathBuf>) -> Result<BlockchainEngine> {
    if let Some(d) = data_dir {
        unsafe {
            std::env::set_var("KANARI_STATE_DB", d);
            std::env::set_var("KANARI_MOVE_VM_DB", d);
        }
        tracing::info!("Using data directory: {}", d.display());
        Ok(BlockchainEngine::new_dir(
            d.to_str().expect("Invalid data directory path"),
        )?)
    } else {
        Ok(BlockchainEngine::new()?)
    }
}

fn native_balance(info: &AccountInfo) -> u64 {
    info.token_balances
        .get(KANARI_TOKEN_TYPE)
        .copied()
        .unwrap_or(0)
}

fn log_shutdown() {
    tracing::info!("Shutdown signal received. Cleaning up and exiting...");
}

fn genesis_root_info(engine: &BlockchainEngine) -> (String, usize) {
    match engine.get_block(0) {
        Some(block) => {
            let size = block.hash.len() / 2;
            (block.hash, size)
        }
        None => ("unknown".to_string(), 0),
    }
}

fn queue_network_message(
    network_tx: &tokio::sync::mpsc::UnboundedSender<P2PMessage>,
    msg: P2PMessage,
    failure_context: &str,
) -> bool {
    match network_tx.send(msg) {
        Ok(_) => true,
        Err(e) => {
            tracing::warn!("{}: {}", failure_context, e);
            false
        }
    }
}

fn serialize_and_queue_message<T: Serialize>(
    network_tx: &tokio::sync::mpsc::UnboundedSender<P2PMessage>,
    value: &T,
    wrap: impl FnOnce(String) -> P2PMessage,
    serialize_context: &str,
    send_context: &str,
) -> Option<usize> {
    match serde_json::to_string(value) {
        Ok(payload) => {
            let payload_len = payload.len();
            queue_network_message(network_tx, wrap(payload), send_context);
            Some(payload_len)
        }
        Err(e) => {
            tracing::error!("{}: {}", serialize_context, e);
            None
        }
    }
}

fn print_stats() -> Result<()> {
    let stats = BlockchainEngine::new()?.get_stats();
    tracing::info!("Blockchain Statistics:");
    tracing::info!("  Height: {}", stats.height);
    tracing::info!("  Total Blocks: {}", stats.total_blocks);
    tracing::info!("  Total Transactions: {}", stats.total_transactions);
    tracing::info!("  Pending: {}", stats.pending_transactions);
    tracing::info!("  Accounts: {}", stats.total_accounts);
    tracing::info!("  Supply: {} Kanari", stats.total_supply);
    Ok(())
}

fn print_account(address: &str) -> Result<()> {
    match BlockchainEngine::new()?.get_account_info(address) {
        Some(info) => {
            tracing::info!("  Account: {}", info.address);
            tracing::info!("  Balance: {}", native_balance(&info));
            tracing::info!("  Sequence: {}", info.sequence_number);
            tracing::info!("  Modules: {}", info.modules.len());
            for module in &info.modules {
                tracing::info!("    - {}", module);
            }
        }
        None => tracing::info!("Account not found: {}", address),
    }
    Ok(())
}

fn print_block(height: u64) -> Result<()> {
    match BlockchainEngine::new()?.get_block(height) {
        Some(block) => {
            tracing::info!("  Block #{}", block.height);
            tracing::info!("  Timestamp: {}", block.timestamp);
            tracing::info!("  Hash: {}", block.hash);
            tracing::info!("  Prev Hash: {}", block.prev_hash);
            tracing::info!("  Transactions: {}", block.tx_count);
        }
        None => tracing::info!("Block not found: {}", height),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber first so all commands have log output
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::ListWallets => {
            for (addr, selected) in list_wallet_files()? {
                tracing::info!("{}{}", addr, if selected { " (selected)" } else { "" });
            }
            Ok(())
        }
        Commands::Stats => print_stats(),
        Commands::Account { address } => print_account(&address),
        Commands::Block { height } => print_block(height),
        Commands::Start {
            p2p_port,
            rpc_port,
            rpc_host,
            data_dir,
            relay_server,
            authority_id,
            authorities,
            bootstrap,
        } => {
            let data_dir_path = data_dir.clone().unwrap_or_else(default_data_dir);
            let mut engine = create_engine(&data_dir)?;

            if let (Some(id), Some(auths)) = (authority_id, authorities) {
                tracing::info!(
                    "Configuring Authority ID: {} with {} authorities",
                    id,
                    auths.len()
                );
                engine.set_authorities(id, auths);
            }

            run_node(
                Arc::new(engine),
                p2p_port,
                rpc_port,
                rpc_host,
                data_dir_path,
                relay_server,
                bootstrap,
            )
            .await
        }
        Commands::Local => {
            tracing::info!("Starting local node: RPC on 127.0.0.1:6767 (P2P disabled)");
            let data_dir_path = std::path::PathBuf::from("./.kanari-local");
            // Ensure data directory exists
            std::fs::create_dir_all(&data_dir_path)?;
            // Create engine with the same data directory for consistency
            let engine = BlockchainEngine::new_dir(data_dir_path.to_str().unwrap())?;
            run_node(
                Arc::new(engine),
                0,
                6767,
                "127.0.0.1".to_string(),
                data_dir_path,
                false,
                None,
            )
            .await
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
    relay_server: bool,
    bootstrap_peers: Option<Vec<String>>,
) -> Result<()> {
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

    let (genesis_root, size_bytes) = genesis_root_info(&engine);
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

    // Create channels for P2P message handling (used even in local mode, messages will be dropped)
    let (p2p_msg_tx, mut p2p_msg_rx) = tokio::sync::mpsc::unbounded_channel::<P2PMessage>();
    let (network_tx, network_rx) = tokio::sync::mpsc::unbounded_channel::<P2PMessage>();

    // Initialize P2P network info
    let keypair = Keypair::generate_ed25519();
    let peer_id = keypair.public().to_peer_id().to_string();
    tracing::info!("Node Peer ID: {}", peer_id);

    // Initialize blockchain indexer
    let node_indexer = match NodeIndexer::new(data_dir.clone()) {
        Ok(idx) => {
            tracing::info!("Blockchain indexer initialized successfully");
            Some(idx)
        }
        Err(e) => {
            tracing::warn!(
                "Failed to initialize indexer: {}. Indexing will be disabled.",
                e
            );
            None
        }
    };

    // Create sync manager with optional indexer
    let sync_manager = Arc::new(SyncManager::new(
        engine.clone(),
        network_tx.clone(),
        peer_id.clone(),
        node_indexer.as_ref().map(|idx| idx.indexer().clone()),
    ));

    // Start sync manager tasks
    sync_manager.clone().start().await;

    if p2p_port > 0 {
        // Load or create peer store
        let peer_store_path = PeerStore::default_path(&data_dir.display().to_string());
        let mut peer_store = PeerStore::load(peer_store_path.clone()).unwrap_or_else(|e| {
            tracing::warn!("Failed to load peer store: {}, creating new one", e);
            PeerStore::new(peer_store_path)
        });

        // Clean up old peers (older than 7 days)
        peer_store.cleanup_old_peers(7 * 24 * 60 * 60);

        let mut p2p_network = P2PNetwork::new(keypair, p2p_port, relay_server)?;
        tracing::info!("P2P network initialized on port {}", p2p_port);
        if relay_server {
            tracing::info!(
                "Relay server mode: ENABLED - This node will help relay traffic for NAT'd peers"
            );
        }

        // Connect to bootstrap peers if provided
        if let Some(bootstrap_list) = bootstrap_peers {
            for bootstrap_addr in bootstrap_list {
                match bootstrap_addr.parse::<libp2p::Multiaddr>() {
                    Ok(addr) => {
                        tracing::info!("Connecting to bootstrap peer: {}", addr);
                        if let Err(e) = p2p_network.swarm.dial(addr.clone()) {
                            tracing::warn!("Failed to dial bootstrap peer {}: {}", addr, e);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Invalid bootstrap multiaddr {}: {}", bootstrap_addr, e);
                    }
                }
            }
        }

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
        tokio::spawn(async move {
            // Wait a bit for peer discovery to complete before first broadcast
            sleep(Duration::from_secs(3)).await;
            loop {
                sync_for_broadcast.broadcast_peer_info().await;
                // Broadcast more frequently (every 5s) to ensure new peers sync quickly
                sleep(Duration::from_secs(5)).await;
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

    loop {
        let stats = engine.get_stats();
        let wallets = list_wallet_files().unwrap_or_default();
        tracing::info!(
            "Event height: {}, Transactions: {}, Pending: {}, Wallets: {}",
            stats.height,
            stats.total_transactions,
            stats.pending_transactions,
            wallets.len()
        );

        // Log indexer statistics periodically (every 10 iterations)
        if let Some(ref node_idx) = node_indexer
            && stats.height > 0
            && stats.height.is_multiple_of(10)
        {
            match node_idx.get_stats() {
                Ok(idx_stats) => tracing::info!("[INDEXER] {}", idx_stats),
                Err(e) => tracing::warn!("[INDEXER] Failed to get stats: {}", e),
            }
        }

        let mut did_work = false;

        if stats.pending_transactions > 0 {
            match engine.produce_block() {
                Ok(block_info) => {
                    did_work = true;

                    tracing::info!(
                        "DAG Vertex (Round #{}) produced: {} txs ({} executed, {} failed)",
                        block_info.round,
                        block_info.tx_count,
                        block_info.executed,
                        block_info.failed
                    );

                    if let Some(vertex) = block_info.vertex {
                        if let Some(vertex_len) = serialize_and_queue_message(
                            &network_tx,
                            &vertex,
                            P2PMessage::NewDagVertex,
                            "Failed to serialize DAG vertex for broadcast",
                            "Failed to queue DAG vertex broadcast",
                        ) {
                            tracing::info!(
                                "Broadcasting DAG vertex {} (round {}) to network ({} bytes)",
                                block_info.vertex_id,
                                block_info.round,
                                vertex_len
                            );
                            tracing::info!("DAG vertex queued for broadcast successfully");
                        }
                    } else {
                        tracing::warn!("No vertex in block_info to broadcast");
                    }

                    if let Some(ref node_idx) = node_indexer {
                        let current_height = engine.blockchain.read().unwrap().height();
                        if let Some(full_block_data) = engine.get_full_block(current_height) {
                            use kanari_types::block::{Block, BlockHeader};
                            use smt::compute_merkle_root;

                            let tx_hashes: Vec<Vec<u8>> = full_block_data
                                .transactions
                                .iter()
                                .map(|tx| tx.hash())
                                .collect();
                            let merkle_root = compute_merkle_root(&tx_hashes);

                            let header = BlockHeader::new(
                                full_block_data.height,
                                hex::decode(&full_block_data.prev_hash).unwrap_or_default(),
                                hex::decode(&full_block_data.state_root).unwrap_or_default(),
                                merkle_root,
                                full_block_data.tx_count,
                                full_block_data.timestamp,
                            );

                            let block = Block {
                                header,
                                transactions: full_block_data.transactions.clone(),
                                events: vec![],
                            };

                            match node_idx.index_block(&block) {
                                Ok(_) => {
                                    if current_height.is_multiple_of(100) {
                                        tracing::info!(
                                            "[INDEXER] Indexed locally produced block #{}",
                                            current_height
                                        );
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "[INDEXER] Failed to index locally produced block #{}: {}",
                                        current_height,
                                        e
                                    );
                                }
                            }
                        }
                    }

                    if block_info.checkpoint.is_some() {
                        let current_height = engine.blockchain.read().unwrap().height();
                        if let Some(full_block_data) = engine.get_full_block(current_height) {
                            serialize_and_queue_message(
                                &network_tx,
                                &full_block_data,
                                P2PMessage::NewBlock,
                                "Failed to serialize block for broadcast",
                                "Failed to queue block broadcast",
                            );
                        }
                    }
                }
                Err(e) => {
                    if !e.to_string().contains("DAG not ready") {
                        tracing::error!("Block production failed: {}", e);
                    }
                }
            }
        }

        if !did_work {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    log_shutdown();
                    break;
                }
                _ = sleep(Duration::from_secs(1)) => {}
            }
        } else if tokio::time::timeout(Duration::from_millis(1), tokio::signal::ctrl_c())
            .await
            .is_ok()
        {
            log_shutdown();
            break;
        }
    }

    tracing::info!("Node shutdown complete.");
    Ok(())
}
