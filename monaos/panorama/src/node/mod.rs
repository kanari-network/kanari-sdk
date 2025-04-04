use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use log::{debug, error, info, warn};
use tokio::sync::mpsc;

use crate::block::Block;
use crate::blockchain::{BlockchainError, BLOCKCHAIN_DATA, normalize_address};
use consensus_pos::Blake3Algorithm;
use mona_types::address::Address;


// Peer data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub address: String,     // Blockchain address
    pub node_id: String,     // Unique node identifier
    pub ip_address: String,  // IP address or hostname
    pub port: u16,           // Port number
    pub last_seen: u64,      // Last time this node was seen
    pub is_validator: bool,  // Whether this node is a validator
}

// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_id: String,      // Unique identifier for this node
    pub blockchain_address: String, // Address for blockchain transactions
    pub listen_ip: String,    // IP to listen on
    pub listen_port: u16,     // Port to listen on
    pub discovery_nodes: Vec<String>, // Known nodes to connect to on startup
    pub max_peers: usize,     // Maximum number of peer connections
    pub is_validator: bool,   // Whether this node is a validator
}

// Default settings
impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: generate_node_id(),
            blockchain_address: String::new(),
            listen_ip: "127.0.0.1".to_string(),
            listen_port: 30303,
            discovery_nodes: vec!["127.0.0.1:30304".to_string()],
            max_peers: 25,
            is_validator: false,
        }
    }
}

// Node status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeStatus {
    Starting,
    Running,
    Syncing,
    Stopped,
}

// Global node registry
lazy_static::lazy_static! {
    pub static ref PEER_LIST: RwLock<HashMap<String, Peer>> = RwLock::new(HashMap::new());
    pub static ref NODE_STATUS: Arc<Mutex<NodeStatus>> = Arc::new(Mutex::new(NodeStatus::Stopped));
    pub static ref NODE_CONFIG: RwLock<NodeConfig> = RwLock::new(NodeConfig::default());
}

// Generate a unique node ID
fn generate_node_id() -> String {
    use rand::{thread_rng, Rng};
    let mut rng = thread_rng();
    let node_id: String = (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..16);
            format!("{:x}", idx)
        })
        .collect();
    
    format!("node-{}", node_id)
}

// Start a node with the given configuration
pub fn start_node(
    config: NodeConfig, 
    status_tx: mpsc::Sender<String>,
) -> Result<(), BlockchainError> {
    // Update node configuration
    {
        let mut node_config = NODE_CONFIG.write().unwrap();
        *node_config = config.clone();
    }
    
    // Update node status
    {
        let mut status = NODE_STATUS.lock().unwrap();
        *status = NodeStatus::Starting;
    }
    
    // Start peer discovery and network listener in background threads
    let status_tx_clone = status_tx.clone();
    let config_clone = config.clone();
    
    thread::spawn(move || {
        if let Err(e) = run_network_listener(config_clone, status_tx_clone) {
            error!("Network listener error: {}", e);
        }
    });
    
    // Start peer discovery after a small delay
    let status_tx_clone = status_tx.clone();
    let config_clone = config.clone();
    
    thread::spawn(move || {
        // Wait a bit for listener to start
        thread::sleep(Duration::from_millis(500));
        
        if let Err(e) = discover_peers(config_clone, status_tx_clone) {
            error!("Peer discovery error: {}", e);
        }
    });
    
    // Update node status
    {
        let mut status = NODE_STATUS.lock().unwrap();
        *status = NodeStatus::Running;
    }
    
    Ok(())
}

// Run the network listener
fn run_network_listener(
    config: NodeConfig,
    status_tx: mpsc::Sender<String>,
) -> Result<(), BlockchainError> {
    // Bind to the configured address and port
    let addr = format!("{}:{}", config.listen_ip, config.listen_port);
    let listener = match TcpListener::bind(&addr) {
        Ok(listener) => listener,
        Err(e) => return Err(BlockchainError::Initialization(
            format!("Failed to bind to {}: {}", addr, e)
        )),
    };
    
    info!("Node listening on {}", addr);
    
    let node_status_json = serde_json::json!({
        "event": "node_listening",
        "node_id": config.node_id,
        "address": addr,
        "blockchain_address": config.blockchain_address,
    }).to_string();
    
    let _ = status_tx.blocking_send(node_status_json);
    
    // Accept connections
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // Handle the connection in a new thread
                let peer_addr = match stream.peer_addr() {
                    Ok(addr) => addr.to_string(),
                    Err(_) => "unknown".to_string(),
                };
                
                info!("New peer connection from {}", peer_addr);
                
                // Process the peer connection
                thread::spawn(move || {
                    if let Err(e) = handle_peer_connection(stream) {
                        error!("Error handling peer connection: {}", e);
                    }
                });
            },
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
    }
    
    Ok(())
}

// Handle peer connection
fn handle_peer_connection(stream: TcpStream) -> Result<(), BlockchainError> {
    // Set read/write timeouts
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    
    // TODO: Implement peer handshake protocol
    // This would include:
    // 1. Exchange node information (version, blockchain height, etc.)
    // 2. Verify compatibility
    // 3. Add to peer list if valid
    
    Ok(())
}

// Discover and connect to peers
fn discover_peers(
    config: NodeConfig,
    status_tx: mpsc::Sender<String>,
) -> Result<(), BlockchainError> {
    info!("Starting peer discovery with {} known nodes", config.discovery_nodes.len());
    
    // Connect to known discovery nodes
    for node_addr in &config.discovery_nodes {
        match TcpStream::connect(node_addr) {
            Ok(stream) => {
                info!("Connected to discovery node: {}", node_addr);
                
                // Process the peer connection
                thread::spawn(move || {
                    if let Err(e) = handle_peer_connection(stream) {
                        error!("Error handling peer connection: {}", e);
                    }
                });
            },
            Err(e) => {
                warn!("Failed to connect to discovery node {}: {}", node_addr, e);
            }
        }
    }
    
    // Periodic peer discovery and maintenance
    thread::spawn(move || {
        loop {
            // Sleep for a bit between peer maintenance cycles
            thread::sleep(Duration::from_secs(60));
            
            // Clean up stale peers
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
                
            let mut peers_to_remove = Vec::new();
            
            {
                let peer_list = PEER_LIST.read().unwrap();
                
                // Find peers that haven't been seen in 5 minutes
                for (id, peer) in peer_list.iter() {
                    if now - peer.last_seen > 300 {
                        peers_to_remove.push(id.clone());
                    }
                }
            }
            
            // Remove stale peers
            if !peers_to_remove.is_empty() {
                let mut peer_list = PEER_LIST.write().unwrap();
                
                for id in &peers_to_remove {
                    if let Some(peer) = peer_list.remove(id) {
                        info!("Removed stale peer: {}", peer.ip_address);
                    }
                }
            }
            
            // Send peer status update
            let peers_count = PEER_LIST.read().unwrap().len();
            let peer_status = serde_json::json!({
                "event": "peers_updated",
                "active_peers": peers_count,
            }).to_string();
            
            let _ = status_tx.blocking_send(peer_status);
        }
    });
    
    Ok(())
}

// Register a new peer
pub fn register_peer(peer: Peer) -> Result<(), BlockchainError> {
    let mut peers = PEER_LIST.write().unwrap();
    
    // Check if we've reached the maximum number of peers
    let config = NODE_CONFIG.read().unwrap();
    if peers.len() >= config.max_peers {
        return Err(BlockchainError::Transaction(
            "Maximum number of peers reached".to_string()
        ));
    }
    
    // Add or update the peer
    peers.insert(peer.node_id.clone(), peer.clone());
    
    info!("Registered peer: {} at {}", peer.node_id, peer.ip_address);
    Ok(())
}

// Propagate a new block to all peers
pub fn propagate_block(block: &Block<Blake3Algorithm>) -> Result<(), BlockchainError> {
    let peers = PEER_LIST.read().unwrap();
    if peers.is_empty() {
        // No peers to propagate to
        return Ok(());
    }
    
    info!("Propagating block {} to {} peers", block.index, peers.len());
    
    // TODO: Implement actual block propagation protocol
    // This would serialize the block and send it to all connected peers
    
    Ok(())
}

// Get peer count
pub fn get_peer_count() -> usize {
    PEER_LIST.read().unwrap().len()
}

// Get list of peers
pub fn get_peers() -> Vec<Peer> {
    let peers = PEER_LIST.read().unwrap();
    peers.values().cloned().collect()
}

// Get current node status
pub fn get_node_status() -> NodeStatus {
    NODE_STATUS.lock().unwrap().clone()
}

// Stop the node
pub fn stop_node() -> Result<(), BlockchainError> {
    // Update node status
    {
        let mut status = NODE_STATUS.lock().unwrap();
        *status = NodeStatus::Stopped;
    }
    
    // TODO: Implement graceful peer disconnection
    
    Ok(())
}
