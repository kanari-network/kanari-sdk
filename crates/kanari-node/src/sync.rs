// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::p2p::{P2PMessage, PeerInfoMsg};
use centauri::consensus::DagVertex;
use kanari_core::{BlockchainEngine, FullBlockData, engine::DagEngine};
use kanari_types::transaction::SignedTransaction;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Handles block and transaction synchronization between peers
pub struct SyncManager {
    engine: Arc<BlockchainEngine>,
    network_tx: mpsc::UnboundedSender<P2PMessage>,
}

impl SyncManager {
    pub fn new(
        engine: Arc<BlockchainEngine>,
        network_tx: mpsc::UnboundedSender<P2PMessage>,
    ) -> Self {
        Self { engine, network_tx }
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
                    // TODO: Validate and apply block to local chain
                    // For now, if we're behind, request missing blocks
                    if block.height > stats.height + 1 {
                        self.request_blocks(stats.height + 1, block.height - 1)
                            .await;
                    }
                }
            }
            Err(e) => {
                error!("Failed to deserialize block: {}", e);
            }
        }
    }

    async fn handle_new_dag_vertex(&self, vertex_data: String) {
        // DAG vertices are serialized as centauri::consensus::DagVertex
        // NOT as DagBlockInfo which is just metadata

        match serde_json::from_str::<DagVertex>(&vertex_data) {
            Ok(vertex) => {
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

                    let dag_engine_opt = dag_engine_arc.read().unwrap();
                    if let Some(ref dag_engine) = *dag_engine_opt {
                        match dag_engine.add_network_vertex(vertex.clone()) {
                            Ok(_) => {
                                info!(
                                    "Successfully added DAG vertex {} to local consensus",
                                    hex::encode(vertex.id)
                                );

                                // Check if this triggered a checkpoint commit
                                let consensus = dag_engine.consensus();
                                if let Ok(Some(checkpoint)) =
                                    consensus.write().unwrap().try_commit()
                                {
                                    info!(
                                        "Checkpoint {} committed with {} vertices and {} transactions",
                                        checkpoint.sequence,
                                        checkpoint.vertices.len(),
                                        checkpoint.transactions.len()
                                    );

                                    // Apply checkpoint to blockchain
                                    if let Err(e) = self
                                        .engine
                                        .blockchain
                                        .write()
                                        .unwrap()
                                        .add_checkpoint(checkpoint)
                                    {
                                        error!("Failed to apply checkpoint to blockchain: {}", e);
                                    } else {
                                        // Persist blockchain and state after checkpoint commit
                                        if let Some(store) = &self.engine.persistent_store {
                                            let chain = self.engine.blockchain.read().unwrap();
                                            if let Err(e) = store.save("blockchain", &*chain) {
                                                error!(
                                                    "Failed to persist blockchain after checkpoint: {}",
                                                    e
                                                );
                                            }
                                            drop(chain);

                                            let state = self.engine.state.read().unwrap();
                                            if let Err(e) = store.save("state_manager", &*state) {
                                                error!(
                                                    "Failed to persist state after checkpoint: {}",
                                                    e
                                                );
                                            }
                                            drop(state);

                                            if let Err(e) = store.flush() {
                                                error!(
                                                    "Failed to flush store after checkpoint: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to add DAG vertex to consensus: {}", e);
                            }
                        }
                    } else {
                        warn!("DAG engine not initialized, cannot process vertex");
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
        if let Some(full_block_data) = self.engine.get_full_block(height) {
            if let Ok(data_str) = serde_json::to_string(&full_block_data) {
                let msg = P2PMessage::BlockResponse(data_str);
                if let Err(e) = self.network_tx.send(msg) {
                    error!("Failed to send block response: {}", e);
                }
            }
        } else {
            warn!("Block #{} not found for request", height);
        }
    }

    async fn handle_block_response(&self, block_data: String) {
        match serde_json::from_str::<FullBlockData>(&block_data) {
            Ok(block) => {
                let stats = self.engine.get_stats();

                // Only apply if it's the next block we need
                if block.height == stats.height + 1 {
                    info!(
                        "Applying block #{} with {} transactions from network",
                        block.height, block.tx_count
                    );
                    match self.engine.sync_full_block_from_data(&block) {
                        Ok(_) => {
                            info!(
                                "Successfully synced block #{} with {} transactions",
                                block.height, block.tx_count
                            );
                        }
                        Err(e) => {
                            error!("Failed to sync block #{}: {}", block.height, e);
                        }
                    }
                } else if block.height > stats.height + 1 {
                    info!(
                        "Received block #{} but still need block #{}",
                        block.height,
                        stats.height + 1
                    );
                } else {
                    info!(
                        "Received old block #{} (current: {})",
                        block.height, stats.height
                    );
                }
            }
            Err(e) => {
                error!("Failed to deserialize block response: {}", e);
            }
        }
    }

    async fn handle_peer_info(&self, peer_info: PeerInfoMsg) {
        let stats = self.engine.get_stats();
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
        info!("Requesting blocks from {} to {}", from, to);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for height in from..=to.min(from + 10) {
            // Limit batch size
            let msg = P2PMessage::BlockRequest(height, timestamp);
            if let Err(e) = self.network_tx.send(msg) {
                error!("Failed to send block request: {}", e);
                break;
            }
        }
    }

    /// Broadcast local chain height to peers
    pub async fn broadcast_peer_info(&self, peer_id: String) {
        let stats = self.engine.get_stats();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let msg = P2PMessage::PeerInfo(PeerInfoMsg {
            height: stats.height,
            peer_id,
            timestamp,
        });

        if let Err(e) = self.network_tx.send(msg) {
            error!("Failed to broadcast peer info: {}", e);
        }
    }
}
