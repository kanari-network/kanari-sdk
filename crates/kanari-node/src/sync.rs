// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::p2p::{BlockRequestMsg, BlockResponseMsg, P2PMessage, PeerInfoMsg, decompress_block};
use centauri::consensus::DagVertex;
use kanari_core::{BlockchainEngine, FullBlockData, engine::DagEngine};
use kanari_types::transaction::SignedTransaction;
use serde::de::DeserializeOwned;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Handles block and transaction synchronization between peers
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub struct SyncManager {
    engine: Arc<BlockchainEngine>,
    network_tx: mpsc::UnboundedSender<P2PMessage>,
    local_peer_id: String,
    /// Optional indexer for blockchain data indexing
    indexer: Option<Arc<Mutex<kanari_indexer::Indexer>>>,
    /// Buffer for blocks that arrived out of order (height -> candidate blocks)
    block_buffer: Mutex<BTreeMap<u64, VecDeque<FullBlockData>>>,
    /// Highest height seen in the network
    max_peer_height: AtomicU64,
    /// Last advertised height by peer id.
    peer_heights: Mutex<BTreeMap<String, u64>>,
    /// Last request timestamp per block height to avoid request spam while still retrying fast.
    pending_block_requests: Mutex<BTreeMap<u64, u64>>,
    /// Maximum number of blocks to keep in buffer to prevent memory exhaustion
    max_buffer_size: usize,
}

impl SyncManager {
    pub fn new(
        engine: Arc<BlockchainEngine>,
        network_tx: mpsc::UnboundedSender<P2PMessage>,
        local_peer_id: String,
        indexer: Option<Arc<Mutex<kanari_indexer::Indexer>>>,
    ) -> Self {
        Self {
            engine,
            network_tx,
            local_peer_id,
            indexer,
            block_buffer: Mutex::new(BTreeMap::new()),
            max_peer_height: AtomicU64::new(0),
            peer_heights: Mutex::new(BTreeMap::new()),
            pending_block_requests: Mutex::new(BTreeMap::new()),
            max_buffer_size: 1000, // Limit buffer to 1000 blocks for 200-node networks
        }
    }

    /// Start periodic sync tasks
    pub async fn start(self: Arc<Self>) {
        let sync = self.clone();
        tokio::spawn(async move {
            loop {
                sync.check_sync_status().await;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
    }

    /// Check if we are behind and need to sync
    async fn check_sync_status(&self) {
        let stats = self.engine.get_stats();
        let max_seen = self.max_peer_height.load(Ordering::Relaxed);

        // Fallback to P2P sync if we are behind
        if stats.height < max_seen {
            let target_peer = self.best_peer_for_height(stats.height + 1);
            info!(
                "[SYNC] Behind network P2P (current: {}, max seen: {}, target: {:?}). Requesting via P2P...",
                stats.height, max_seen, target_peer
            );
            self.request_blocks(stats.height + 1, max_seen, target_peer.as_deref())
                .await;
        }
    }

    /// Process incoming P2P messages
    pub async fn handle_message(&self, msg: P2PMessage) {
        match msg {
            P2PMessage::NewTransaction(tx_data) => {
                info!("[P2P] Received NewTransaction");
                self.handle_new_transaction(tx_data).await;
            }
            P2PMessage::NewBlock(block_data) => {
                info!("[P2P] Received NewBlock");
                self.handle_new_block(block_data).await;
            }
            P2PMessage::NewDagVertex(vertex_data) => {
                info!("[P2P] Received NewDagVertex");
                self.handle_new_dag_vertex(vertex_data).await;
            }
            P2PMessage::DagVertexRebroadcast(msg) => {
                if msg.sender_peer_id == self.local_peer_id {
                    return;
                }
                info!(
                    "[P2P] Received DagVertexRebroadcast from {}",
                    msg.sender_peer_id
                );
                self.handle_new_dag_vertex(msg.vertex_data).await;
            }
            P2PMessage::BlockRequest(height, timestamp) => {
                info!("[P2P] Received BlockRequest for height {}", height);
                self.handle_block_request(height, timestamp, None, None)
                    .await;
            }
            P2PMessage::TargetedBlockRequest(req) => {
                if req.requester_peer_id == self.local_peer_id {
                    return;
                }
                if req.responder_peer_id != self.local_peer_id {
                    return;
                }
                info!(
                    "[P2P] Received targeted BlockRequest for height {} from {}",
                    req.height, req.requester_peer_id
                );
                self.handle_block_request(
                    req.height,
                    req.timestamp,
                    Some(req.requester_peer_id.as_str()),
                    Some(req.responder_peer_id.as_str()),
                )
                .await;
            }
            P2PMessage::BlockResponse(block_data) => {
                info!("[P2P] Received BlockResponse");
                self.handle_block_response(block_data).await;
            }
            P2PMessage::TargetedBlockResponse(resp) => {
                if resp.requester_peer_id != self.local_peer_id {
                    return;
                }
                info!(
                    "[P2P] Received targeted BlockResponse for height {} from {}",
                    resp.height, resp.responder_peer_id
                );
                self.handle_block_response(resp.block_data).await;
            }
            P2PMessage::PeerInfo(peer_info) => {
                info!("[P2P] Received PeerInfo from {}", peer_info.peer_id);
                self.handle_peer_info(peer_info).await;
            }
            // Handle compressed messages (should be decompressed before reaching here)
            P2PMessage::CompressedBlock(_) | P2PMessage::CompressedDagVertex(_) => {
                warn!(
                    "[P2P] Received compressed message in sync manager - should be decompressed already"
                );
            }
            P2PMessage::CompressedTargetedBlockResponse(_) => {
                warn!(
                    "[P2P] Received compressed targeted block response in sync manager - should be decompressed already"
                );
            }
            P2PMessage::CompressedBlockResponse(compressed_data) => {
                if let Ok(data) = decompress_block(compressed_data.to_vec()) {
                    self.handle_block_response(data).await;
                }
            }
        }
    }

    fn parse_message<T: DeserializeOwned>(data: &str, context: &str) -> Option<T> {
        match serde_json::from_str(data) {
            Ok(value) => Some(value),
            Err(e) => {
                error!("Failed to deserialize {}: {}", context, e);
                None
            }
        }
    }

    fn send_network_message(&self, msg: P2PMessage, context: &str) -> bool {
        match self.network_tx.send(msg) {
            Ok(_) => true,
            Err(e) => {
                error!("{}: {}", context, e);
                false
            }
        }
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn buffer_block(&self, block: FullBlockData, label: &str) -> Option<usize> {
        let block_height = block.height;
        let mut buffer = self.block_buffer.lock().unwrap();
        let candidate_count: usize = buffer.values().map(VecDeque::len).sum();
        if candidate_count >= self.max_buffer_size {
            warn!(
                "[SYNC] Block buffer full (max: {}). Dropping block #{}",
                self.max_buffer_size, block_height
            );
            return None;
        }

        let candidates = buffer.entry(block_height).or_default();
        if candidates.iter().any(|candidate| {
            candidate.hash == block.hash && candidate.state_root == block.state_root
        }) {
            info!(
                "[SYNC] Duplicate {} #{} already buffered, ignoring",
                label, block_height
            );
            return Some(candidate_count);
        }

        candidates.push_back(block);
        let candidate_count = candidate_count + 1;
        info!(
            "[SYNC] Buffered {} #{}. Candidates for height: {}, total buffered: {}/{}",
            label,
            block_height,
            candidates.len(),
            candidate_count,
            self.max_buffer_size
        );
        Some(candidate_count)
    }

    fn latest_buffered_height(&self) -> u64 {
        let buffer = self.block_buffer.lock().unwrap();
        buffer.keys().last().copied().unwrap_or(0)
    }

    fn best_peer_for_height(&self, height: u64) -> Option<String> {
        let peers = self.peer_heights.lock().unwrap();
        peers
            .iter()
            .filter(|(peer_id, peer_height)| {
                peer_id.as_str() != self.local_peer_id && **peer_height >= height
            })
            .max_by_key(|(_, peer_height)| *peer_height)
            .map(|(peer_id, _)| peer_id.clone())
    }

    fn should_request_height(&self, height: u64, now: u64) -> bool {
        let mut pending = self.pending_block_requests.lock().unwrap();
        match pending.get(&height).copied() {
            Some(last_requested) if now.saturating_sub(last_requested) < 2 => false,
            _ => {
                pending.insert(height, now);
                true
            }
        }
    }

    fn clear_pending_requests_up_to(&self, height: u64) {
        let mut pending = self.pending_block_requests.lock().unwrap();
        pending.retain(|pending_height, _| *pending_height > height);
    }

    async fn process_incoming_block(
        &self,
        block: FullBlockData,
        received_label: &str,
        buffered_label: &str,
        check_for_gap: bool,
    ) {
        let stats = self.engine.get_stats();
        info!(
            "[SYNC] Processing {} #{} (current height: {}, txs: {})",
            received_label, block.height, stats.height, block.tx_count
        );

        if block.height <= stats.height {
            info!(
                "[SYNC] Received old {} #{} (current: {}) - ignoring",
                received_label, block.height, stats.height
            );
            return;
        }

        let block_height = block.height;
        if self.buffer_block(block, buffered_label).is_none() {
            return;
        }

        self.try_apply_buffered_blocks().await;

        if check_for_gap {
            let new_stats = self.engine.get_stats();
            let latest_buffered = self.latest_buffered_height();
            if latest_buffered > new_stats.height + 1 {
                info!(
                    "[SYNC] Gap detected after {} #{}. Current: {}, buffered: {}. Requesting missing blocks...",
                    received_label, block_height, new_stats.height, latest_buffered
                );
                let target_peer = self.best_peer_for_height(new_stats.height + 1);
                self.request_blocks(
                    new_stats.height + 1,
                    latest_buffered - 1,
                    target_peer.as_deref(),
                )
                .await;
            }
        }
    }

    async fn handle_new_transaction(&self, tx_data: String) {
        if let Some(signed_tx) = Self::parse_message::<SignedTransaction>(&tx_data, "transaction") {
            match self.engine.submit_transaction(signed_tx.clone()) {
                Ok(tx_hash) => {
                    info!(
                        "Received transaction from network: 0x{}",
                        hex::encode(tx_hash)
                    );
                }
                Err(e) => {
                    warn!("Failed to submit transaction from network: {}", e);
                }
            }
        }
    }

    async fn handle_new_block(&self, block_data: String) {
        if let Some(block) = Self::parse_message::<FullBlockData>(&block_data, "new block") {
            self.process_incoming_block(block, "NewBlock", "NewBlock", true)
                .await;
        }
    }

    /// Try to apply buffered blocks in sequence
    async fn try_apply_buffered_blocks(&self) {
        loop {
            let stats = self.engine.get_stats();
            let next_height = stats.height + 1;

            let next_block = {
                let mut buffer = self.block_buffer.lock().unwrap();
                let next_block = buffer
                    .get_mut(&next_height)
                    .and_then(|candidates| candidates.pop_front());
                let should_remove = buffer
                    .get(&next_height)
                    .map(|candidates| candidates.is_empty())
                    .unwrap_or(false);
                if should_remove {
                    buffer.remove(&next_height);
                }
                next_block
            };

            if let Some(block) = next_block {
                info!(
                    "[SYNC] Found block #{} in buffer, attempting to apply. (current height: {})",
                    block.height, stats.height
                );
                match self.engine.sync_full_block_from_data(&block) {
                    Ok(_) => {
                        info!(
                            "[SYNC] Successfully synced block #{} with {} transactions",
                            block.height, block.tx_count
                        );

                        // Index the block if indexer is available
                        if let Some(ref indexer) = self.indexer {
                            match self.index_block_with_indexer(indexer, &block) {
                                Ok(_) => {}
                                Err(e) => {
                                    error!(
                                        "[INDEXER] Failed to index block #{}: {}",
                                        block.height, e
                                    );
                                }
                            }
                        }

                        // Broadcast our new height
                        self.broadcast_peer_info().await;
                        self.clear_pending_requests_up_to(block.height);
                    }
                    Err(e) => {
                        let has_more_candidates = {
                            let buffer = self.block_buffer.lock().unwrap();
                            buffer
                                .get(&block.height)
                                .map(|candidates| !candidates.is_empty())
                                .unwrap_or(false)
                        };

                        if has_more_candidates {
                            warn!(
                                "[SYNC] Failed to sync block #{} candidate: {}. Trying next candidate.",
                                block.height, e
                            );
                            continue;
                        }

                        warn!(
                            "[SYNC] Failed to sync block #{} candidate: {}. Requesting replacement.",
                            block.height, e
                        );
                        let target_peer = self.best_peer_for_height(block.height);
                        self.request_blocks(block.height, block.height, target_peer.as_deref())
                            .await;
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
                        "[SYNC] Applied all buffered blocks but still behind network (current: {}, max seen: {}).",
                        stats.height, max_seen
                    );

                    // Check buffer for gaps
                    {
                        let buffer = self.block_buffer.lock().unwrap();
                        if !buffer.is_empty() {
                            let buffered_heights: Vec<_> = buffer.keys().collect();
                            info!(
                                "[SYNC] Buffered blocks: {:?}. Next required: {}",
                                buffered_heights,
                                stats.height + 1
                            );
                        }
                    }

                    let target_peer = self.best_peer_for_height(stats.height + 1);
                    self.request_blocks(stats.height + 1, max_seen, target_peer.as_deref())
                        .await;
                }
                break;
            }
        }

        // Clean up old blocks from buffer
        {
            let stats = self.engine.get_stats();
            let mut buffer = self.block_buffer.lock().unwrap();
            let initial_len = buffer.len();
            buffer.retain(|&h, _| h > stats.height);
            if buffer.len() < initial_len {
                info!(
                    "[SYNC] Cleaned up {} old blocks from buffer. Current size: {}",
                    initial_len - buffer.len(),
                    buffer.len()
                );
            }
        }
    }

    async fn handle_new_dag_vertex(&self, vertex_data: String) {
        // DAG vertices are serialized as centauri::consensus::DagVertex
        // NOT as DagBlockInfo which is just metadata

        if let Some(vertex) = Self::parse_message::<DagVertex>(&vertex_data, "DAG vertex") {
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
                // Check and initialize with write lock to prevent TOCTOU race
                {
                    let mut guard = dag_engine_arc.write().unwrap();

                    // Only initialize if still None after acquiring write lock
                    if guard.is_none() {
                        info!(
                            "[DAG SYNC] Auto-initializing DAG engine for vertex round {}",
                            vertex.round
                        );

                        // Initialize DAG engine with same authorities as the network
                        let authority_id = self.engine.get_authority_id();
                        let authorities = self.engine.get_authorities();

                        match DagEngine::new(self.engine.clone(), authority_id, authorities) {
                            Ok(engine) => {
                                *guard = Some(engine);
                                info!("[DAG SYNC] DAG engine initialized successfully");
                            }
                            Err(e) => {
                                error!("[DAG SYNC] Failed to initialize DAG engine: {}", e);
                            }
                        }
                    }
                    // Write lock is released here when guard goes out of scope
                }

                // Get the engine with a read lock for processing
                let dag_engine_opt = {
                    let guard = dag_engine_arc.read().unwrap();
                    guard.as_ref().cloned()
                };

                if let Some(dag_engine) = dag_engine_opt {
                    match dag_engine.add_network_vertex(vertex.clone()) {
                        Ok(_) => {
                            info!(
                                "Successfully added DAG vertex {} to local consensus",
                                hex::encode(vertex.id)
                            );
                            // Check if height changed to broadcast new info
                            let new_stats = self.engine.get_stats();
                            if new_stats.height > stats.height {
                                info!("New height reached via DAG: {}", new_stats.height);
                                self.broadcast_peer_info().await;
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
    }

    async fn handle_block_request(
        &self,
        height: u64,
        request_timestamp: u64,
        requester_peer_id: Option<&str>,
        responder_peer_id: Option<&str>,
    ) {
        info!("[SYNC] Received block request for height {}", height);
        if let Some(full_block_data) = self.engine.get_full_block(height) {
            info!(
                "[SYNC] Found block #{} with {} txs, sending response",
                height, full_block_data.tx_count
            );
            if let Ok(data_str) = serde_json::to_string(&full_block_data) {
                let msg = if let (Some(requester_peer_id), Some(responder_peer_id)) =
                    (requester_peer_id, responder_peer_id)
                {
                    P2PMessage::TargetedBlockResponse(BlockResponseMsg {
                        height,
                        request_timestamp,
                        requester_peer_id: requester_peer_id.to_string(),
                        responder_peer_id: responder_peer_id.to_string(),
                        block_data: data_str,
                    })
                } else {
                    P2PMessage::BlockResponse(data_str)
                };
                self.send_network_message(msg, "[SYNC] Failed to send block response");
            }
        } else {
            warn!(
                "[SYNC] Block #{} not found in our engine for request",
                height
            );
        }
    }

    async fn handle_block_response(&self, block_data: String) {
        if let Some(block) = Self::parse_message::<FullBlockData>(&block_data, "block response") {
            self.clear_pending_requests_up_to(block.height.saturating_sub(1));
            self.process_incoming_block(block, "block response", "block", false)
                .await;
        }
    }

    async fn handle_peer_info(&self, peer_info: PeerInfoMsg) {
        let stats = self.engine.get_stats();
        let local_checkpoint_hash = self.engine.latest_checkpoint_hash_hex();
        let local_state_root = self.engine.latest_checkpoint_state_root_hex();
        info!(
            "[SYNC] Received PeerInfo from {}: height={}, txs={}, our_height={}, our_txs={}",
            peer_info.peer_id,
            peer_info.height,
            peer_info.total_transactions,
            stats.height,
            stats.total_transactions
        );

        if peer_info.peer_id != self.local_peer_id {
            let mut peer_heights = self.peer_heights.lock().unwrap();
            peer_heights.insert(peer_info.peer_id.clone(), peer_info.height);
        }

        // Update max seen height
        let current_max = self.max_peer_height.load(Ordering::Relaxed);
        if peer_info.height > current_max {
            info!(
                "[SYNC] Updating max_peer_height from {} to {}",
                current_max, peer_info.height
            );
            self.max_peer_height
                .store(peer_info.height, Ordering::Relaxed);
        }

        if peer_info.height == stats.height {
            if peer_info.latest_state_root != local_state_root {
                warn!(
                    "[SYNC] Diverged state detected with peer {} at height {}. local_state_root={}, peer_state_root={}",
                    peer_info.peer_id, stats.height, local_state_root, peer_info.latest_state_root
                );
            } else if peer_info.latest_checkpoint_hash != local_checkpoint_hash {
                warn!(
                    "[SYNC] Diverged checkpoint history detected with peer {} at height {}. local_checkpoint_hash={}, peer_checkpoint_hash={}",
                    peer_info.peer_id,
                    stats.height,
                    local_checkpoint_hash,
                    peer_info.latest_checkpoint_hash
                );
            }
        }

        if peer_info.height > stats.height {
            info!(
                "[SYNC] Peer {} is ahead at height {} (current: {})",
                peer_info.peer_id, peer_info.height, stats.height
            );
            // Request missing blocks starting from the next block we need
            // If we are at 0 (genesis only), start from 1.
            // If we are at N, start from N+1.
            let start_height = stats.height + 1;

            // Only request if start_height <= peer_height
            if start_height <= peer_info.height {
                info!(
                    "[SYNC] Requesting blocks from {} to {} from peer {}",
                    start_height, peer_info.height, peer_info.peer_id
                );
                self.request_blocks(start_height, peer_info.height, Some(&peer_info.peer_id))
                    .await;
            } else {
                warn!(
                    "[SYNC] Peer {} has height {} but we need {} (stats.height: {}). Not requesting.",
                    peer_info.peer_id, peer_info.height, start_height, stats.height
                );
            }
        } else {
            info!(
                "[SYNC] Peer {} is at height {} (current: {}). We are synced or ahead.",
                peer_info.peer_id, peer_info.height, stats.height
            );
        }
    }

    async fn request_blocks(&self, from: u64, to: u64, target_peer_id: Option<&str>) {
        if from > to {
            return;
        }

        let stats = self.engine.get_stats();
        info!(
            "[SYNC] Requesting blocks from {} to {} (our current height: {}, target: {:?})",
            from, to, stats.height, target_peer_id
        );
        let timestamp = Self::current_timestamp();

        // Limit the number of blocks requested at once to avoid network congestion
        // Increased from 50 to 200 for better performance in large networks (200+ nodes)
        let max_request = 200;
        let actual_to = to.min(from + max_request);
        let mut sent = 0u64;

        for height in from..=actual_to {
            // Check if we already have this block in buffer before requesting
            {
                let buffer = self.block_buffer.lock().unwrap();
                if buffer.contains_key(&height) {
                    continue;
                }
            }

            if !self.should_request_height(height, timestamp) {
                continue;
            }

            let msg = if let Some(target_peer_id) = target_peer_id {
                P2PMessage::TargetedBlockRequest(BlockRequestMsg {
                    height,
                    timestamp,
                    requester_peer_id: self.local_peer_id.clone(),
                    responder_peer_id: target_peer_id.to_string(),
                })
            } else {
                P2PMessage::BlockRequest(height, timestamp)
            };

            if !self.send_network_message(
                msg,
                &format!("[SYNC] Failed to send block request for height {}", height),
            ) {
                break;
            }
            sent += 1;
        }
        info!("[SYNC] Sent {} block requests starting from {}", sent, from);
    }

    /// Broadcast local chain height to peers
    pub async fn broadcast_peer_info(&self) {
        let stats = self.engine.get_stats();
        let timestamp = Self::current_timestamp();

        let msg = P2PMessage::PeerInfo(PeerInfoMsg {
            height: stats.height,
            peer_id: self.local_peer_id.clone(),
            timestamp,
            latest_checkpoint_hash: self.engine.latest_checkpoint_hash_hex(),
            latest_state_root: self.engine.latest_checkpoint_state_root_hex(),
            total_transactions: stats.total_transactions,
        });

        self.send_network_message(msg, "Failed to broadcast peer info");
    }

    /// Index a block using the indexer (helper method)
    fn index_block_with_indexer(
        &self,
        indexer: &Arc<Mutex<kanari_indexer::Indexer>>,
        full_block: &FullBlockData,
    ) -> anyhow::Result<()> {
        let block = BlockchainEngine::block_from_full_data(full_block);
        let idx = indexer
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire indexer lock: {}", e))?;
        idx.index_block(&block)?;

        Ok(())
    }
}
