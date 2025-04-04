use serde::{Serialize, Deserialize};
use crate::block::Block;
use crate::blockchain::{BlockchainError, BLOCKCHAIN_DATA};
use consensus_pos::Blake3Algorithm;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Block announcement message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockAnnouncement {
    pub node_id: String,
    pub block_index: u32,
    pub block_hash: String,
    pub timestamp: u64,
}

// Block request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRequest {
    pub node_id: String,
    pub block_index: u32,
}

// Block response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockResponse {
    pub node_id: String,
    pub block: Block<Blake3Algorithm>,
    pub timestamp: u64,
}

// Transaction announcement message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionAnnouncement {
    pub node_id: String,
    pub transaction_ids: Vec<String>,
    pub timestamp: u64,
}

// Node status message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatusMessage {
    pub node_id: String,
    pub blockchain_height: u32,
    pub peers: u32,
    pub address: String,
    pub is_validator: bool,
    pub timestamp: u64,
}

// Message types for node communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeMessage {
    Hello(String),                      // Node ID
    Ping(u64),                          // Timestamp
    Pong(u64),                          // Timestamp
    BlockAnnouncement(BlockAnnouncement),
    BlockRequest(BlockRequest),
    BlockResponse(BlockResponse),
    TransactionAnnouncement(TransactionAnnouncement),
    NodeStatus(NodeStatusMessage),
    Goodbye,
}

// Track received blocks to prevent duplication
lazy_static::lazy_static! {
    static ref RECEIVED_BLOCKS: RwLock<HashMap<String, u64>> = RwLock::new(HashMap::new());
}

// Process incoming node message
pub fn process_message(message: NodeMessage) -> Result<Option<NodeMessage>, BlockchainError> {
    match message {
        NodeMessage::Hello(node_id) => {
            debug!("Received Hello from node: {}", node_id);
            // Respond with our status
            let status = NodeStatusMessage {
                node_id: super::NODE_CONFIG.read().unwrap().node_id.clone(),
                blockchain_height: BLOCKCHAIN_DATA.len() as u32,
                peers: super::get_peer_count() as u32,
                address: super::NODE_CONFIG.read().unwrap().blockchain_address.clone(),
                is_validator: super::NODE_CONFIG.read().unwrap().is_validator,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };
            
            Ok(Some(NodeMessage::NodeStatus(status)))
        },
        
        NodeMessage::Ping(timestamp) => {
            // Respond with pong
            Ok(Some(NodeMessage::Pong(timestamp)))
        },
        
        NodeMessage::BlockAnnouncement(announcement) => {
            // Check if we've already seen this block
            let block_key = format!("{}:{}", announcement.block_index, announcement.block_hash);
            
            {
                let received = RECEIVED_BLOCKS.read().unwrap();
                if received.contains_key(&block_key) {
                    debug!("Ignoring duplicate block announcement: {}", block_key);
                    return Ok(None);
                }
            }
            
            // Check if we need this block
            if announcement.block_index <= BLOCKCHAIN_DATA.len() as u32 {
                debug!("Ignoring announcement for already known block: {}", announcement.block_index);
                return Ok(None);
            }
            
            // Request the block if it's the next one we need
            if announcement.block_index == BLOCKCHAIN_DATA.len() as u32 + 1 {
                info!("Requesting new block: {}", announcement.block_index);
                
                // Mark block as requested
                {
                    let mut received = RECEIVED_BLOCKS.write().unwrap();
                    received.insert(block_key, announcement.timestamp);
                }
                
                let request = BlockRequest {
                    node_id: super::NODE_CONFIG.read().unwrap().node_id.clone(),
                    block_index: announcement.block_index,
                };
                
                return Ok(Some(NodeMessage::BlockRequest(request)));
            }
            
            Ok(None)
        },
        
        NodeMessage::BlockRequest(request) => {
            // Check if we have the requested block
            if request.block_index < BLOCKCHAIN_DATA.len() as u32 {
                if let Some(block) = BLOCKCHAIN_DATA.get_block(request.block_index as usize) {
                    info!("Sending block {} to node {}", request.block_index, request.node_id);
                    
                    let response = BlockResponse {
                        node_id: super::NODE_CONFIG.read().unwrap().node_id.clone(),
                        block,
                        timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    };
                    
                    return Ok(Some(NodeMessage::BlockResponse(response)));
                }
            }
            
            debug!("Don't have requested block {}", request.block_index);
            Ok(None)
        },
        
        NodeMessage::BlockResponse(response) => {
            // Process received block
            info!("Received block {} from node {}", 
                  response.block.index, response.node_id);
                  
            // Verify the block before adding it to our chain
            if response.block.index != BLOCKCHAIN_DATA.len() as u32 {
                warn!("Received out of order block: expected {}, got {}", 
                     BLOCKCHAIN_DATA.len(), response.block.index);
                return Ok(None);
            }
            
            if let Some(prev_block) = BLOCKCHAIN_DATA.get_block(BLOCKCHAIN_DATA.len() - 1) {
                if !response.block.verify(&prev_block) {
                    warn!("Received invalid block: {}", response.block.index);
                    return Ok(None);
                }
                
                // Block is valid, add it to our chain
                BLOCKCHAIN_DATA.add_block(response.block.clone());
                info!("Added new block {} to chain", response.block.index);
                
                // Propagate the block to our peers
                if let Err(e) = super::propagate_block(&response.block) {
                    warn!("Failed to propagate block: {}", e);
                }
                
                // Clean up received blocks cache periodically
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                    
                {
                    let mut received = RECEIVED_BLOCKS.write().unwrap();
                    received.retain(|_, &mut timestamp| now - timestamp < 3600); // Keep for 1 hour
                }
            }
            
            Ok(None)
        },
        
        NodeMessage::TransactionAnnouncement(_) => {
            // TODO: Implement transaction processing
            debug!("Transaction announcement received - not yet implemented");
            Ok(None)
        },
        
        NodeMessage::NodeStatus(status) => {
            info!("Node {} status: height={}, peers={}, validator={}",
                 status.node_id, status.blockchain_height, status.peers, status.is_validator);
                 
            // TODO: Use this information for peer management
            
            Ok(None)
        },
        
        NodeMessage::Goodbye => {
            info!("Peer disconnecting");
            Ok(None)
        },
        
        _ => {
            warn!("Unknown message type received");
            Ok(None)
        }
    }
}
