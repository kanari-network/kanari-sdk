// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::p2p::{P2PMessage, PeerInfoMsg};
use centauri::consensus::DagVertex;
use kanari_core::{BlockchainEngine, FullBlockData, engine::DagEngine};
use kanari_types::transaction::SignedTransaction;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Handles block and transaction synchronization between peers
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct SyncManager {
    engine: Arc<BlockchainEngine>,
    network_tx: mpsc::UnboundedSender<P2PMessage>,
    local_peer_id: String,
    /// Buffer for blocks that arrived out of order (height -> block)
    block_buffer: Mutex<std::collections::BTreeMap<u64, FullBlockData>>,
    /// Highest height seen in the network
    max_peer_height: AtomicU64,
}

impl SyncManager {
    pub fn new(
        engine: Arc<BlockchainEngine>,
        network_tx: mpsc::UnboundedSender<P2PMessage>,
        local_peer_id: String,
    ) -> Self {
        Self {
            engine,
            network_tx,
            local_peer_id,
            block_buffer: Mutex::new(std::collections::BTreeMap::new()),
            max_peer_height: AtomicU64::new(0),
        }
    }

    /// Process incoming P2P messages
    pub async fn handle_message(&self, msg: P2PMessage) {
        match msg {
            P2PMessage::NewTransaction(tx_data) => {
                self.handle_new_transaction(tx_data).await;
            }
            P2PMessage::NewBlock(block_data) => {
                self.handle_new_block(block_data).await;
            }
            P2PMessage::NewDagVertex(vertex_data) => {
                self.handle_new_dag_vertex(vertex_data).await;
            }
            P2PMessage::BlockRequest(height, _timestamp) => {
                self.handle_block_request(height).await;
            }
            P2PMessage::BlockResponse(block_data) => {
                self.handle_block_response(block_data).await;
            }
            P2PMessage::PeerInfo(peer_info) => {
                self.handle_peer_info(peer_info).await;
            }
        }
    }

    async fn handle_new_transaction(&self, tx_data: String) {
        match serde_json::from_str::<SignedTransaction>(&tx_data) {
            Ok(signed_tx) => match self.engine.submit_transaction(signed_tx.clone()) {
                Ok(tx_hash) => {
                    info!(
                        "Received transaction from network: 0x{}",
                        hex::encode(tx_hash)
                    );
                }
                Err(e) => {
                    warn!("Failed to submit transaction from network: {}", e);
                }
            },
            Err(e) => {
                error!("Failed to deserialize transaction: {}", e);
            }
        }
    }

    async fn handle_new_block(&self, block_data: String) {
        match serde_json::from_str::<FullBlockData>(&block_data) {
            Ok(block) => {
                let stats = self.engine.get_stats();
                if block.height > stats.height {
                    info!(
                        "Received new block #{} from network (current: {}) with {} transactions",
                        block.height, stats.height, block.tx_count
                    );

                    // Buffer the block
                    {
                        let mut buffer = self.block_buffer.lock().unwrap();
                        buffer.insert(block.height, block);
                    }

                    // Try to apply consecutive blocks from the buffer
                    self.try_apply_buffered_blocks().await;

                    // If we're still behind, request missing blocks
                    let new_stats = self.engine.get_stats();
                    let latest_buffered = {
                        let buffer = self.block_buffer.lock().unwrap();
                        buffer.keys().last().cloned().unwrap_or(0)
                    };

                    if latest_buffered > new_stats.height + 1 {
                        self.request_blocks(new_stats.height + 1, latest_buffered - 1)
                            .await;
                    }
                }
            }
            Err(e) => {
                error!("Failed to deserialize new block: {}", e);
            }
        }
    }

    /// Try to apply buffered blocks in sequence
    async fn try_apply_buffered_blocks(&self) {
        loop {
            let stats = self.engine.get_stats();
            let next_height = stats.height + 1;

            let next_block = {
                let mut buffer = self.block_buffer.lock().unwrap();
                buffer.remove(&next_height)
            };

            if let Some(block) = next_block {
                info!("Applying block #{} from buffer", block.height);
                match self.engine.sync_full_block_from_data(&block) {
                    Ok(_) => {
                        info!(
                            "Successfully synced block #{} with {} transactions",
                            block.height, block.tx_count
                        );
                        // Broadcast our new height
                        self.broadcast_peer_info().await;
                    }
                    Err(e) => {
                        error!("Failed to sync block #{}: {}", block.height, e);
                        // Put it back in buffer if it failed (maybe temporary?)
                        // Actually, if it failed due to state root mismatch, putting it back won't help.
                        // But for now, let's keep it simple.
                        break;
                    }
                }
            } else {
                // No more consecutive blocks in buffer.
                // Check if we are still behind the network and should request more.
                let stats = self.engine.get_stats();
                let max_seen = self.max_peer_height.load(Ordering::Relaxed);
                if stats.height < max_seen {
                    info!(
                        "[SYNC] Applied all buffered blocks but still behind network (current: {}, max seen: {}). Requesting next batch.",
                        stats.height, max_seen
                    );
                    self.request_blocks(stats.height + 1, max_seen).await;
                }
                break;
            }
        }

        // Clean up old blocks from buffer
        {
            let stats = self.engine.get_stats();
            let mut buffer = self.block_buffer.lock().unwrap();
            buffer.retain(|&h, _| h > stats.height);
        }
    }

    async fn handle_new_dag_vertex(&self, vertex_data: String) {
        // DAG vertices are serialized as centauri::consensus::DagVertex
        // NOT as DagBlockInfo which is just metadata

        match serde_json::from_str::<DagVertex>(&vertex_data) {
            Ok(vertex) => {
                let stats = self.engine.get_stats();
                info!(
                    "Received DAG vertex {} (round {}) from network with {} transactions",
                    hex::encode(vertex.id),
                    vertex.round,
                    vertex.transactions.len()
                );

                // Add vertex to local DAG consensus
                // Auto-initialize DAG engine if not already initialized
                if let Some(dag_engine_arc) = self.engine.get_dag_engine() {
                    // Ensure DAG engine is initialized before processing vertex
                    {
                        let mut dag_engine_opt = dag_engine_arc.write().unwrap();
                        if dag_engine_opt.is_none() {
                            info!(
                                "[DAG SYNC] Auto-initializing DAG engine for vertex round {}",
                                vertex.round
                            );

                            // Initialize DAG engine with same authorities as the network
                            let authority_id = self.engine.get_authority_id();
                            let authorities = self.engine.get_authorities();

                            match DagEngine::new(self.engine.clone(), authority_id, authorities) {
                                Ok(engine) => {
                                    *dag_engine_opt = Some(engine);
                                    info!("[DAG SYNC] DAG engine initialized successfully");
                                }
                                Err(e) => {
                                    error!("[DAG SYNC] Failed to initialize DAG engine: {}", e);
                                    return;
                                }
                            }
                        }
                    }

                    let height_changed = {
                        let dag_engine_opt = dag_engine_arc.read().unwrap();
                        if let Some(ref dag_engine) = *dag_engine_opt {
                            match dag_engine.add_network_vertex(vertex.clone()) {
                                Ok(_) => {
                                    info!(
                                        "Successfully added DAG vertex {} to local consensus",
                                        hex::encode(vertex.id)
                                    );
                                    // Check if height changed to broadcast new info
                                    let new_stats = self.engine.get_stats();
                                    new_stats.height > stats.height
                                }
                                Err(e) => {
                                    warn!("Failed to add DAG vertex to consensus: {}", e);
                                    false
                                }
                            }
                        } else {
                            warn!("DAG engine not initialized, cannot process vertex");
                            false
                        }
                    };

                    if height_changed {
                        let new_stats = self.engine.get_stats();
                        info!("New height reached via DAG: {}", new_stats.height);
                        self.broadcast_peer_info().await;
                    }
                } else {
                    warn!("DAG mode not enabled, ignoring vertex");
                }
            }
            Err(e) => {
                error!("Failed to deserialize DAG vertex: {}", e);
            }
        }
    }

    async fn handle_block_request(&self, height: u64) {
        info!("[SYNC] Received block request for height {}", height);
        if let Some(full_block_data) = self.engine.get_full_block(height) {
            info!(
                "[SYNC] Found block #{} with {} txs, sending response",
                height, full_block_data.tx_count
            );
            if let Ok(data_str) = serde_json::to_string(&full_block_data) {
                let msg = P2PMessage::BlockResponse(data_str);
                if let Err(e) = self.network_tx.send(msg) {
                    error!("[SYNC] Failed to send block response: {}", e);
                }
            }
        } else {
            warn!(
                "[SYNC] Block #{} not found in our engine for request",
                height
            );
        }
    }

    async fn handle_block_response(&self, block_data: String) {
        match serde_json::from_str::<FullBlockData>(&block_data) {
            Ok(block) => {
                let stats = self.engine.get_stats();

                if block.height > stats.height {
                    info!(
                        "[SYNC] Received block response #{} (current: {}, txs: {})",
                        block.height, stats.height, block.tx_count
                    );

                    // Buffer the block
                    {
                        let mut buffer = self.block_buffer.lock().unwrap();
                        buffer.insert(block.height, block);
                    }

                    // Try to apply consecutive blocks
                    self.try_apply_buffered_blocks().await;
                } else {
                    info!(
                        "[SYNC] Received old block response #{} (current: {}) - ignoring",
                        block.height, stats.height
                    );
                }
            }
            Err(e) => {
                error!("[SYNC] Failed to deserialize block response: {}", e);
            }
        }
    }

    async fn handle_peer_info(&self, peer_info: PeerInfoMsg) {
        let stats = self.engine.get_stats();

        // Update max seen height
        let current_max = self.max_peer_height.load(Ordering::Relaxed);
        if peer_info.height > current_max {
            self.max_peer_height
                .store(peer_info.height, Ordering::Relaxed);
        }

        if peer_info.height > stats.height {
            info!(
                "Peer {} is ahead at height {} (current: {})",
                peer_info.peer_id, peer_info.height, stats.height
            );
            // Request missing blocks starting from the next block we need
            // If we are at 0 (genesis only), start from 1.
            // If we are at N, start from N+1.
            let start_height = stats.height + 1;

            // Only request if start_height <= peer_height
            if start_height <= peer_info.height {
                self.request_blocks(start_height, peer_info.height).await;
            } else {
                warn!(
                    "Peer {} has height {} but we need {} (stats.height: {}). Not requesting.",
                    peer_info.peer_id, peer_info.height, start_height, stats.height
                );
            }
        } else {
            info!(
                "Peer {} is at height {} (current: {}). We are synced.",
                peer_info.peer_id, peer_info.height, stats.height
            );
        }
    }

    async fn request_blocks(&self, from: u64, to: u64) {
        if from > to {
            return;
        }

        info!(
            "[SYNC] Requesting blocks from {} to {} (current height: {})",
            from,
            to,
            self.engine.get_stats().height
        );
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Limit the number of blocks requested at once to avoid network congestion
        let max_request = 50;
        let actual_to = to.min(from + max_request);

        for height in from..=actual_to {
            let msg = P2PMessage::BlockRequest(height, timestamp);
            if let Err(e) = self.network_tx.send(msg) {
                error!(
                    "[SYNC] Failed to send block request for height {}: {}",
                    height, e
                );
                break;
            }
        }
    }

    /// Broadcast local chain height to peers
    pub async fn broadcast_peer_info(&self) {
        let stats = self.engine.get_stats();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let msg = P2PMessage::PeerInfo(PeerInfoMsg {
            height: stats.height,
            peer_id: self.local_peer_id.clone(),
            timestamp,
        });

        if let Err(e) = self.network_tx.send(msg) {
            error!("Failed to broadcast peer info: {}", e);
        }
    }
}
