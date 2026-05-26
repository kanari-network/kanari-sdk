// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use kanari_core::BlockchainEngine;
use kanari_core::engine::AccountInfo;
use kanari_crypto::wallet::list_wallet_files;
use kanari_rpc_server::start_server;
use kanari_types::address::Address as KanariAddress;
use kanari_types::kanari::{KANARI_TOKEN_TYPE, KanariModule};
use libp2p::identity::Keypair;
use serde::Serialize;
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tokio::time::sleep;

use crate::NetworkMode;
use crate::indexer::NodeIndexer;
use crate::p2p::{DagVertexMsg, P2PEventHandler, P2PMessage, P2PNetwork};
use crate::peer_store::PeerStore;
use crate::sync::SyncManager;

pub fn default_data_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home)
        .join(".kanari")
        .join("kanari-db")
}

fn env_write_guard() -> &'static Mutex<()> {
    static ENV_WRITE_GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_WRITE_GUARD.get_or_init(|| Mutex::new(()))
}

fn path_to_env_value(path: &std::path::Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid data directory path: {}", path.display()))
}

fn configure_engine_environment(
    data_dir: Option<&std::path::Path>,
    network: &NetworkMode,
) -> Result<()> {
    let _env_guard = env_write_guard().lock().unwrap_or_else(|e| e.into_inner());

    // Safety: this runs during CLI startup before the Tokio runtime is created
    // and before any background worker threads are spawned.
    unsafe {
        std::env::set_var("KANARI_NETWORK", network.as_str());
    }

    if let Some(dir) = data_dir {
        let dir_str = path_to_env_value(dir)?;
        // Safety: guarded by the same startup-only contract described above.
        unsafe {
            std::env::set_var("KANARI_STATE_DB", dir_str);
            std::env::set_var("KANARI_MOVE_VM_DB", dir_str);
        }
    }

    Ok(())
}

pub fn create_engine(
    data_dir: &Option<std::path::PathBuf>,
    network: &NetworkMode,
) -> Result<BlockchainEngine> {
    configure_engine_environment(data_dir.as_deref(), network)?;
    if let Some(dir) = data_dir {
        tracing::info!("Using data directory: {}", dir.display());
        let dir_str = path_to_env_value(dir)?;
        Ok(BlockchainEngine::new_dir(dir_str)?)
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

fn rebroadcast_latest_dag_vertex(
    engine: &Arc<BlockchainEngine>,
    network_tx: &tokio::sync::mpsc::UnboundedSender<P2PMessage>,
    peer_id: &str,
) {
    let vertices = match engine.latest_own_dag_vertices(16) {
        Ok(vertices) => vertices,
        Err(e) => {
            tracing::warn!("Failed to load latest DAG vertices for rebroadcast: {}", e);
            return;
        }
    };

    if vertices.is_empty() {
        return;
    }

    let nonce_base = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    for vertex in vertices {
        if let Ok(vertex_data) = serde_json::to_string(&vertex) {
            let msg = P2PMessage::DagVertexRebroadcast(DagVertexMsg {
                vertex_data,
                nonce: nonce_base ^ vertex.round,
                sender_peer_id: peer_id.to_string(),
            });
            if queue_network_message(network_tx, msg, "Failed to queue DAG vertex rebroadcast") {
                tracing::info!(
                    "Rebroadcasting DAG vertex {} (round {}) while waiting for quorum",
                    hex::encode(vertex.id),
                    vertex.round
                );
            }
        }
    }
}

pub fn print_stats() -> Result<()> {
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

pub fn print_account(address: &str) -> Result<()> {
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

pub fn print_block(height: u64) -> Result<()> {
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

fn detect_local_ip() -> Option<String> {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .ok()
        .and_then(|socket| {
            if socket.connect("8.8.8.8:80").is_ok() {
                socket.local_addr().ok().map(|a| a.ip().to_string())
            } else {
                None
            }
        })
}

fn current_chain_height(engine: &Arc<BlockchainEngine>) -> u64 {
    engine
        .blockchain
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .height()
}

pub async fn run_node(
    engine: Arc<BlockchainEngine>,
    network: String,
    p2p_port: u16,
    rpc_port: u16,
    rpc_host: String,
    data_dir: std::path::PathBuf,
    relay_server: bool,
    bootstrap_peers: Option<Vec<String>>,
) -> Result<()> {
    engine.validate_runtime_health()?;
    let runtime_guards = engine.runtime_guard_config();
    let stats = engine.get_stats();

    tracing::info!("Kanari blockchain node starting");
    tracing::info!("Network: {}, Move VM: Enabled", network);
    tracing::info!(
        "Runtime guards: strict_persistence={}, strict_checkpoint_roots={}, fail_fast_supply={}",
        runtime_guards.strict_persistence_required,
        runtime_guards.strict_checkpoint_roots,
        runtime_guards.fail_fast_supply_enabled
    );
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

    let dev_addr = KanariAddress::DEV_ADDRESS;
    tracing::info!("RPC Server sequencer address: ({})", dev_addr);

    let (p2p_msg_tx, mut p2p_msg_rx) = tokio::sync::mpsc::unbounded_channel::<P2PMessage>();
    let (network_tx, network_rx) = tokio::sync::mpsc::unbounded_channel::<P2PMessage>();

    let keypair = Keypair::generate_ed25519();
    let peer_id = keypair.public().to_peer_id().to_string();
    tracing::info!("Node Peer ID: {}", peer_id);

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

    let sync_manager = Arc::new(SyncManager::new(
        engine.clone(),
        network_tx.clone(),
        peer_id.clone(),
        node_indexer.as_ref().map(|idx| idx.indexer().clone()),
    ));
    sync_manager.clone().start().await;

    if p2p_port > 0 {
        let peer_store_path = PeerStore::default_path(&data_dir.display().to_string());
        let mut peer_store = PeerStore::load(peer_store_path.clone()).unwrap_or_else(|e| {
            tracing::warn!("Failed to load peer store: {}, creating new one", e);
            PeerStore::new(peer_store_path)
        });
        peer_store.cleanup_old_peers(7 * 24 * 60 * 60);

        let mut p2p_network = P2PNetwork::new(keypair, p2p_port, relay_server)?;
        tracing::info!("P2P network initialized on port {}", p2p_port);
        if relay_server {
            tracing::info!(
                "Relay server mode: ENABLED - This node will help relay traffic for NAT'd peers"
            );
        }

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

        let peer_store_arc = Arc::new(tokio::sync::Mutex::new(peer_store));
        let mut event_handler = P2PEventHandler::new(p2p_network, p2p_msg_tx)
            .with_outgoing(network_rx)
            .with_peer_store(peer_store_arc.clone());
        tokio::spawn(async move {
            event_handler.run().await;
        });

        let sync_for_messages = sync_manager.clone();
        tokio::spawn(async move {
            while let Some(msg) = p2p_msg_rx.recv().await {
                sync_for_messages.handle_message(msg).await;
            }
        });

        let sync_for_broadcast = sync_manager.clone();
        tokio::spawn(async move {
            sleep(Duration::from_secs(3)).await;
            loop {
                sync_for_broadcast.broadcast_peer_info().await;
                sleep(Duration::from_secs(5)).await;
            }
        });
    } else {
        tracing::info!("Running in local-only mode: P2P disabled");
    }

    let bind_addr = format!("{}:{}", rpc_host, rpc_port);
    tracing::info!("Binding RPC server to {}", bind_addr);

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

        if stats.pending_transactions > 0 || engine.should_produce_dag_progress() {
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
                        let current_height = current_chain_height(&engine);
                        if let Some(full_block_data) = engine.get_full_block(current_height) {
                            let block = BlockchainEngine::block_from_full_data(&full_block_data);

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
                        let current_height = current_chain_height(&engine);
                        if let Some(checkpoint_sync) = engine.get_checkpoint_sync(current_height) {
                            serialize_and_queue_message(
                                &network_tx,
                                &checkpoint_sync,
                                P2PMessage::NewCheckpoint,
                                "Failed to serialize checkpoint for broadcast",
                                "Failed to queue checkpoint broadcast",
                            );
                        }
                    }
                }
                Err(e) => {
                    let error_text = e.to_string();
                    if error_text.contains("DAG_WAITING")
                        || error_text.contains("SYNC_WAITING")
                        || error_text.contains("Not enough parents for quorum")
                    {
                        tracing::info!("Block production waiting: {}", error_text);
                        rebroadcast_latest_dag_vertex(&engine, &network_tx, &peer_id);
                    } else if !error_text.contains("DAG not ready") {
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
