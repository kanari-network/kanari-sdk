// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_core::{BlockchainEngine, FullBlockData, SignedTransaction};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::p2p::{P2PMessage, PeerInfoMsg};

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
            // Request missing blocks starting from block 1 (skip genesis at height 0)
            // This ensures we sync all blocks from the beginning
            let start_height = if stats.height == 0 {
                1
            } else {
                stats.height + 1
            };
            self.request_blocks(start_height, peer_info.height).await;
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
