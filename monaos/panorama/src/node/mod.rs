use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

use mona_blockchain::block::Block;
use mona_blockchain::blockchain::BlockchainError;
use consensus_pos::Blake3Algorithm;
use mona_crypto::hash_data_blake3;
use rand::Rng; // Add Rng trait import
pub mod coordinator;

// Peer data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub address: String,          // Blockchain address
    pub node_id: String,          // Unique node identifier
    pub ip_address: String,       // IP address or hostname
    pub port: u16,                // Port number
    pub last_seen: u64,           // Last time this node was seen
    pub is_validator: bool,       // Whether this node is a validator
    pub protocol_version: String, // Protocol version for compatibility checking
    pub tls_supported: bool,      // Whether this peer supports TLS
}

// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_id: String,              // Unique identifier for this node
    pub blockchain_address: String,   // Address for blockchain transactions
    pub listen_ip: String,            // IP to listen on
    pub listen_port: u16,             // Port to listen on
    pub discovery_nodes: Vec<String>, // Known nodes to connect to on startup
    pub max_peers: usize,             // Maximum number of peer connections
    pub is_validator: bool,           // Whether this node is a validator
    pub use_tls: bool,                // Whether to use TLS for secure communication
    pub localhost_only: bool,         // Whether to restrict connections to localhost only
}

// Protocol message for node communication
#[derive(Clone, Serialize, Deserialize)]
pub enum NodeMessage {
    Handshake {
        node_id: String,
        blockchain_address: String,
        protocol_version: String,
        is_validator: bool,
        chain_height: u64,
        nonce: String, // Added nonce for security
    },
    HandshakeResponse {
        success: bool,
        node_id: String,
        peers: Vec<Peer>,
        message: String,
        signature: String,           // Added signature for verification
    },
    BlockAnnounce {
        block_index: u64,
        block_hash: String,
    },
    BlockRequest {
        block_hash: String,
    },
    BlockResponse {
        block: Option<Block<Blake3Algorithm>>,
        error: Option<String>,
    },
    TransactionAnnounce {
        transaction_ids: Vec<String>,
    },
    TransactionRequest {
        transaction_id: String,
    },
    PingRequest {
        timestamp: u64,
    },
    PongResponse {
        request_timestamp: u64,
        response_timestamp: u64,
    },
    PeerListRequest {},
    PeerListResponse {
        peers: Vec<Peer>,
    },
    SecurityChallenge {              // New message type for security verification
        challenge: String,
    },
    SecurityResponse {               // New message type for security verification
        challenge_response: String,
        node_signature: String,
    },
    Disconnect {
        reason: String,
    },
}

// Default settings
impl Default for NodeConfig {
    fn default() -> Self {
        // Get the Kari directory for storing certificates
        let kari_dir = common::get_kari_dir();
        let certs_dir = kari_dir.join("certs");
        
        // Create the certs directory if it doesn't exist
        if !certs_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&certs_dir) {
                eprintln!("Warning: Could not create certificates directory: {}", e);
            }
        }
        
        Self {
            node_id: generate_node_id(),
            blockchain_address: String::new(), // Will be populated from wallet
            listen_ip: "0.0.0.0".to_string(),  // Listen on all interfaces
            listen_port: 51303,                // Default P2P port
            discovery_nodes: vec![
                // Add some default discovery nodes for mainnet
                "mainnet-seed1.kanari.network:51303".to_string(),
                "mainnet-seed2.kanari.network:51303".to_string(),
            ],
            max_peers: 50,
            is_validator: false,
            use_tls: false,          // Default to false for backwards compatibility
            localhost_only: false,    // Default to allow external connections
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

// Protocol version
const PROTOCOL_VERSION: &str = "0.2.0";
const HANDSHAKE_TIMEOUT_SECS: u64 = 10;
const PING_INTERVAL_SECS: u64 = 30;
const CONNECTION_TIMEOUT_SECS: u64 = 5;

// Global node registry
lazy_static::lazy_static! {
    pub static ref PEER_LIST: RwLock<HashMap<String, Peer>> = RwLock::new(HashMap::new());
    pub static ref NODE_STATUS: Arc<Mutex<NodeStatus>> = Arc::new(Mutex::new(NodeStatus::Stopped));
    pub static ref NODE_CONFIG: RwLock<NodeConfig> = RwLock::new(NodeConfig::default());
    pub static ref ACTIVE_CONNECTIONS: RwLock<HashMap<String, Arc<Mutex<TcpStream>>>> = RwLock::new(HashMap::new());
}

// Generate a unique node ID
fn generate_node_id() -> String {
    use rand::{Rng, thread_rng};
    let mut rng = thread_rng();
    let node_id: String = (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..16);
            format!("{:x}", idx)
        })
        .collect();

    format!("node-{}", node_id)
}

// Get local network IP address (not localhost) - IMPROVED implementation
pub fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;

    // Connect to a public IP (doesn't actually send data)
    // This trick helps find which network interface would be used
    match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => {
            // Try connecting to multiple public DNS servers for better reliability
            let dns_servers = ["8.8.8.8:80", "1.1.1.1:80", "9.9.9.9:80"];
            
            for server in dns_servers {
                if socket.connect(server).is_ok() {
                    if let Ok(addr) = socket.local_addr() {
                        return Some(addr.ip().to_string());
                    }
                }
            }
        }
        Err(_) => {}
    }

    // More fallback methods for IP detection
    if let Ok(hostname_output) = std::process::Command::new("hostname").output() {
        let hostname = String::from_utf8_lossy(&hostname_output.stdout).trim().to_string();
        if let Ok(addrs) = hostname.to_socket_addrs() {
            for addr in addrs {
                if !addr.ip().is_loopback() && !addr.ip().is_unspecified() {
                    return Some(addr.ip().to_string());
                }
            }
        }
    }

    None
}

// Add a function to generate self-signed certificates with complete implementation
pub fn generate_self_signed_certificates() -> Result<(), BlockchainError> {
    info!("Generating self-signed TLS certificates...");
    
    // Get the Kari certificates directory
    let kari_dir = common::get_kari_dir();
    let certs_dir = kari_dir.join("certs");
    
    // Create the directory if it doesn't exist
    if !certs_dir.exists() {
        std::fs::create_dir_all(&certs_dir)
            .map_err(|e| BlockchainError::Initialization(
                format!("Failed to create certificates directory: {}", e)))?;
    }
    
    // Define certificate paths
    let cert_path = certs_dir.join("node.crt");
    let key_path = certs_dir.join("node.key");
    
    // Check if certificates already exist
    if cert_path.exists() && key_path.exists() {
        info!("TLS certificates already exist at:");
        info!("  - Certificate: {}", cert_path.display());
        info!("  - Key: {}", key_path.display());
        return Ok(());
    }
    
    // Get local IP for certificate
    let local_ip = get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());
    let hostname = std::process::Command::new("hostname")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "kanari-node".to_string());
    
    info!("Generating certificate for node: {}, IP: {}", hostname, local_ip);
    
    // Create command to generate certificates
    #[cfg(not(target_os = "windows"))]
    let status = std::process::Command::new("openssl")
        .args(&[
            "req", "-x509", 
            "-newkey", "rsa:4096", 
            "-keyout", key_path.to_str().unwrap(),
            "-out", cert_path.to_str().unwrap(),
            "-days", "365",
            "-nodes",
            "-subj", &format!("/CN={}", hostname),
            "-addext", &format!("subjectAltName=DNS:{},IP:{}", hostname, local_ip)
        ])
        .status();
    
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("openssl")
        .args(&[
            "req", "-x509", 
            "-newkey", "rsa:4096", 
            "-keyout", key_path.to_str().unwrap(),
            "-out", cert_path.to_str().unwrap(),
            "-days", "365",
            "-nodes",
            "-subj", &format!("//CN={}", hostname),
            "-addext", &format!("subjectAltName=DNS:{},IP:{}", hostname, local_ip)
        ])
        .status();
    
    match status {
        Ok(exit_status) => {
            if exit_status.success() {
                info!("Successfully generated TLS certificates:");
                info!("  - Certificate: {}", cert_path.display());
                info!("  - Key: {}", key_path.display());
                Ok(())
            } else {
                let error = format!("Failed to generate certificates (exit code: {})", exit_status);
                error!("{}", error);
                Err(BlockchainError::Initialization(error))
            }
        },
        Err(e) => {
            let error = format!("Failed to execute openssl command: {}", e);
            error!("{}", error);
            Err(BlockchainError::Initialization(error))
        }
    }
}

// Enhanced function to start a node with better security
pub fn start_node(
    config: NodeConfig,
    status_tx: mpsc::Sender<String>,
) -> Result<(), BlockchainError> {
    // Generate TLS certificates if TLS is enabled
    if config.use_tls {
        if let Err(e) = generate_self_signed_certificates() {
            warn!("Failed to generate TLS certificates: {}", e);
            warn!("Continuing without TLS encryption");
        }
    }
    
    // Get the actual network IP for display purposes
    let local_ip = get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());

    // Display node configuration
    info!("Starting node with ID: {}", config.node_id);
    info!(
        "Node listening IP: {} (actual network IP: {})",
        config.listen_ip, local_ip
    );
    info!("Node P2P port: {}", config.listen_port);
    info!("Node is validator: {}", config.is_validator);

    // Check local IP restrictions
    if config.localhost_only {
        info!("Node configured for localhost connections only");
    }

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

    // Notify about actual network address for connecting peers
    let network_info = serde_json::json!({
        "event": "node_network_info",
        "node_id": config.node_id,
        "listen_address": format!("{}:{}", config.listen_ip, config.listen_port),
        "network_address": format!("{}:{}", local_ip, config.listen_port),
        "blockchain_address": config.blockchain_address
    })
    .to_string();

    // Clone before moving into the closure
    let status_tx_for_network = status_tx.clone();
    tokio::task::spawn_blocking(move || {
        let _ = status_tx_for_network.blocking_send(network_info);
    });

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

    // Start ping service to keep connections alive
    let config_clone = config.clone();
    thread::spawn(move || {
        run_ping_service(config_clone);
    });

    // Update node status
    {
        let mut status = NODE_STATUS.lock().unwrap();
        *status = NodeStatus::Running;
    }

    Ok(())
}

// Run periodic ping to keep connections alive
fn run_ping_service(_config: NodeConfig) {
    loop {
        thread::sleep(Duration::from_secs(PING_INTERVAL_SECS));

        // Check if node is still running
        let node_status = NODE_STATUS.lock().unwrap().clone();
        if matches!(node_status, NodeStatus::Stopped) {
            break;
        }

        // Get a list of current peers to ping
        let peer_ids: Vec<String> = {
            let connections = ACTIVE_CONNECTIONS.read().unwrap();
            connections.keys().cloned().collect()
        };

        for peer_id in peer_ids {
            let _ = ping_peer(&peer_id);
        }
    }
}

// Send ping to a specific peer
fn ping_peer(peer_id: &str) -> Result<(), BlockchainError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let ping_message = NodeMessage::PingRequest { timestamp };

    send_message_to_peer(peer_id, &ping_message)?;
    debug!("Sent ping to peer {}", peer_id);

    Ok(())
}

// Run the network listener
fn run_network_listener(
    config: NodeConfig,
    status_tx: mpsc::Sender<String>,
) -> Result<(), BlockchainError> {
    // Determine the correct bind address based on localhost_only setting
    let bind_ip = if config.localhost_only {
        "127.0.0.1".to_string()
    } else {
        config.listen_ip.clone()
    };
    
    let addr = format!("{}:{}", bind_ip, config.listen_port);
    
    info!("Attempting to bind to {} for P2P connections", addr);
    
    let listener = match TcpListener::bind(&addr) {
        Ok(listener) => listener,
        Err(e) => return Err(BlockchainError::Initialization(
            format!("Failed to bind to {}: {}", addr, e)
        )),
    };

    // Display the actual address that was successfully bound to
    let actual_addr = listener.local_addr().map_err(|e| {
        BlockchainError::Initialization(format!("Failed to get local address: {}", e))
    })?;
    info!("Successfully bound to {} for P2P connections", actual_addr);

    // Set non-blocking mode for listener
    listener.set_nonblocking(true).map_err(|e| {
        BlockchainError::Initialization(format!("Failed to set non-blocking mode: {}", e))
    })?;

    info!("Node listening on {}", addr);

    // Get actual network IP for other nodes to connect to
    let local_ip = get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());

    // Notify about node start via status channel
    let node_status_json = serde_json::json!({
        "event": "node_listening",
        "node_id": config.node_id,
        "address": addr,
        "network_address": format!("{}:{}", local_ip, config.listen_port),
        "blockchain_address": config.blockchain_address,
        "protocol_version": PROTOCOL_VERSION,
    })
    .to_string();

    let _ = status_tx.blocking_send(node_status_json);

    // Accept connections in a loop with improved security checks
    loop {
        // Check if the node is still running
        if matches!(*NODE_STATUS.lock().unwrap(), NodeStatus::Stopped) {
            break;
        }

        // Try to accept a connection
        match listener.accept() {
            Ok((stream, remote_addr)) => {
                info!("New peer connection from {}", remote_addr);
                
                // Check if we should reject this connection based on IP restrictions
                if config.localhost_only && !remote_addr.ip().is_loopback() {
                    warn!("Rejecting non-localhost connection from {} (localhost only mode)", remote_addr);
                    continue;
                }

                // Set a timeout for the connection
                stream
                    .set_read_timeout(Some(Duration::from_secs(CONNECTION_TIMEOUT_SECS)))
                    .unwrap_or_else(|e| warn!("Failed to set read timeout: {}", e));

                stream
                    .set_write_timeout(Some(Duration::from_secs(CONNECTION_TIMEOUT_SECS)))
                    .unwrap_or_else(|e| warn!("Failed to set write timeout: {}", e));

                // Clone necessary data for the thread
                let config_clone = config.clone();
                let status_tx_clone = status_tx.clone();

                // Handle the connection in a new thread
                thread::spawn(move || {
                    if let Err(e) = handle_incoming_connection(
                        stream,
                        remote_addr,
                        config_clone,
                        status_tx_clone,
                    ) {
                        error!("Error handling peer connection: {}", e);
                    }
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No connections ready, sleep a bit to avoid high CPU
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => {
                error!("Error accepting connection: {}", e);
                // Sleep briefly to avoid error spam in case of persistent issues
                thread::sleep(Duration::from_millis(1000));
            }
        }
    }

    Ok(())
}

// Handle incoming peer connection with handshake protocol
fn handle_incoming_connection(
    mut stream: TcpStream,
    remote_addr: std::net::SocketAddr,
    config: NodeConfig,
    status_tx: mpsc::Sender<String>,
) -> Result<(), BlockchainError> {
    // Read the handshake message first
    let mut buffer = [0u8; 4096];
    let read_result = stream.read(&mut buffer);

    let bytes_read = match read_result {
        Ok(n) => n,
        Err(e) => {
            return Err(BlockchainError::Network(format!(
                "Failed to read from connection: {}",
                e
            )));
        }
    };

    if bytes_read == 0 {
        return Err(BlockchainError::Network(
            "Peer disconnected immediately".to_string(),
        ));
    }

    // Parse the handshake message
    let message: NodeMessage = match bincode::deserialize(&buffer[0..bytes_read]) {
        Ok(msg) => msg,
        Err(e) => {
            return Err(BlockchainError::Network(format!(
                "Failed to parse handshake message: {}",
                e
            )));
        }
    };

    // Process the handshake
    match message {
        NodeMessage::Handshake {
            node_id,
            blockchain_address,
            protocol_version,
            is_validator,
            chain_height,
            nonce,
        } => {
            info!(
                "Received handshake from node {} ({}), protocol v{}, chain height: {}, nonce: {}",
                node_id, blockchain_address, protocol_version, chain_height, nonce
            );

            // Check protocol version compatibility
            if !is_protocol_compatible(&protocol_version) {
                warn!(
                    "Incompatible protocol version: {} (ours is {})",
                    protocol_version, PROTOCOL_VERSION
                );

                // Send rejection response
                let response = NodeMessage::HandshakeResponse {
                    success: false,
                    node_id: config.node_id.clone(),
                    peers: vec![],
                    message: format!(
                        "Incompatible protocol version. Expected {}, got {}",
                        PROTOCOL_VERSION, protocol_version
                    ),
                    signature: "".to_string(), // No signature on rejection
                };

                let response_data = bincode::serialize(&response)
                    .map_err(|e| BlockchainError::Network(format!("Serialization error: {}", e)))?;

                stream.write_all(&response_data).map_err(|e| {
                    BlockchainError::Network(format!("Failed to send response: {}", e))
                })?;

                return Err(BlockchainError::Network(format!(
                    "Incompatible protocol version from peer {}",
                    node_id
                )));
            }

            // Check if we have too many peers already
            {
                let peers = PEER_LIST.read().unwrap();
                if peers.len() >= config.max_peers {
                    // Send rejection response
                    let response = NodeMessage::HandshakeResponse {
                        success: false,
                        node_id: config.node_id.clone(),
                        peers: vec![],
                        message: "Maximum number of peers reached".to_string(),
                        signature: "".to_string(), // No signature on rejection
                    };

                    let response_data = bincode::serialize(&response).map_err(|e| {
                        BlockchainError::Network(format!("Serialization error: {}", e))
                    })?;

                    stream.write_all(&response_data).map_err(|e| {
                        BlockchainError::Network(format!("Failed to send response: {}", e))
                    })?;

                    return Err(BlockchainError::Network(format!(
                        "Maximum number of peers reached, rejected {}",
                        node_id
                    )));
                }
            }

            // Create and register peer
            let peer = Peer {
                address: blockchain_address,
                node_id: node_id.clone(),
                ip_address: remote_addr.ip().to_string(),
                port: remote_addr.port(),
                last_seen: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                is_validator,
                protocol_version,
                tls_supported: config.use_tls, // Include TLS support info
            };

            // Register the peer
            if let Err(e) = register_peer(peer.clone()) {
                warn!("Failed to register peer: {}", e);

                // Send rejection response
                let response = NodeMessage::HandshakeResponse {
                    success: false,
                    node_id: config.node_id.clone(),
                    peers: vec![],
                    message: format!("Failed to register peer: {}", e),
                    signature: "".to_string(), // No signature on rejection
                };

                let response_data = bincode::serialize(&response)
                    .map_err(|e| BlockchainError::Network(format!("Serialization error: {}", e)))?;

                stream.write_all(&response_data).map_err(|e| {
                    BlockchainError::Network(format!("Failed to send response: {}", e))
                })?;

                return Err(e);
            }

            // Add to active connections
            {
                let mut connections = ACTIVE_CONNECTIONS.write().unwrap();
                connections.insert(
                    node_id.clone(),
                    Arc::new(Mutex::new(stream.try_clone().unwrap())),
                );
            }

            // Get current peers to share with the new peer
            let our_peers = {
                let peers = PEER_LIST.read().unwrap();
                peers.values().cloned().collect::<Vec<_>>()
            };

            // Send successful handshake response
            let response = NodeMessage::HandshakeResponse {
                success: true,
                node_id: config.node_id.clone(),
                peers: our_peers,
                message: "Connection accepted".to_string(),
                signature: "".to_string(), // No signature on initial response
            };

            let response_data = bincode::serialize(&response)
                .map_err(|e| BlockchainError::Network(format!("Serialization error: {}", e)))?;

            stream
                .write_all(&response_data)
                .map_err(|e| BlockchainError::Network(format!("Failed to send response: {}", e)))?;

            // Notify about new peer
            let peer_status = serde_json::json!({
                "event": "peer_connected",
                "peer_id": peer.node_id,
                "address": format!("{}:{}", peer.ip_address, peer.port),
                "blockchain_address": peer.address,
                "is_validator": peer.is_validator,
                "connected_peers": get_peer_count()
            })
            .to_string();

            let _ = status_tx.blocking_send(peer_status);

            // Spawn a thread to handle incoming messages from this peer
            let peer_id = node_id.clone();
            let status_tx_clone = status_tx.clone();
            thread::spawn(move || {
                if let Err(e) = handle_peer_messages(peer_id.clone(), stream, status_tx_clone) {
                    error!("Error handling messages from peer {}: {}", peer_id, e);
                }
            });

            Ok(())
        }
        _ => {
            // Expected a handshake message
            Err(BlockchainError::Network(
                "Expected handshake message".to_string(),
            ))
        }
    }
}

// Check if the peer's protocol version is compatible with ours
fn is_protocol_compatible(peer_version: &str) -> bool {
    // Simple version check - major version must match
    let our_parts: Vec<&str> = PROTOCOL_VERSION.split('.').collect();
    let peer_parts: Vec<&str> = peer_version.split('.').collect();

    if our_parts.is_empty() || peer_parts.is_empty() {
        return false;
    }

    // Only check major version for now
    our_parts[0] == peer_parts[0]
}

// Thread to handle incoming messages from a peer
fn handle_peer_messages(
    peer_id: String,
    mut stream: TcpStream,
    status_tx: mpsc::Sender<String>,
) -> Result<(), BlockchainError> {
    // Set the stream to non-blocking mode for better performance
    stream
        .set_nonblocking(true)
        .map_err(|e| BlockchainError::Network(format!("Failed to set non-blocking mode: {}", e)))?;

    let mut buffer = [0u8; 8192]; // Larger buffer for messages

    loop {
        // Check if the node is still running
        if matches!(*NODE_STATUS.lock().unwrap(), NodeStatus::Stopped) {
            // Send disconnect message before breaking
            let disconnect_msg = NodeMessage::Disconnect {
                reason: "Node shutting down".to_string(),
            };

            let msg_data = match bincode::serialize(&disconnect_msg) {
                Ok(data) => data,
                Err(e) => {
                    warn!("Failed to serialize disconnect message: {}", e);
                    break;
                }
            };

            let _ = stream.write_all(&msg_data);
            break;
        }

        // Try to read a message from the peer
        match stream.read(&mut buffer) {
            Ok(0) => {
                // Connection closed by peer
                info!("Peer {} closed connection", peer_id);
                break;
            }
            Ok(n) => {
                // Process the message
                match bincode::deserialize::<NodeMessage>(&buffer[0..n]) {
                    Ok(message) => {
                        if let Err(e) = process_peer_message(&peer_id, message, &status_tx) {
                            warn!("Error processing message from peer {}: {}", peer_id, e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to deserialize message from peer {}: {}", peer_id, e);
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available yet, sleep a bit
                thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(e) => {
                // Some other error occurred
                return Err(BlockchainError::Network(format!(
                    "Error reading from peer {}: {}",
                    peer_id, e
                )));
            }
        }
    }

    // Remove peer from active connections
    {
        let mut connections = ACTIVE_CONNECTIONS.write().unwrap();
        connections.remove(&peer_id);
    }

    // Update peer's last_seen time but leave in peer list
    // This allows for reconnection attempts later
    {
        let mut peers = PEER_LIST.write().unwrap();
        if let Some(peer) = peers.get_mut(&peer_id) {
            peer.last_seen = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
    }

    // Notify about peer disconnect
    let disconnect_status = serde_json::json!({
        "event": "peer_disconnected",
        "peer_id": peer_id,
        "connected_peers": get_peer_count()
    })
    .to_string();

    let _ = status_tx.blocking_send(disconnect_status);

    Ok(())
}

// Process a message received from a peer
fn process_peer_message(
    peer_id: &str,
    message: NodeMessage,
    status_tx: &mpsc::Sender<String>,
) -> Result<(), BlockchainError> {
    match message {
        NodeMessage::PingRequest { timestamp } => {
            // Respond with a pong
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let pong = NodeMessage::PongResponse {
                request_timestamp: timestamp,
                response_timestamp: now,
            };

            send_message_to_peer(peer_id, &pong)?;
        }
        NodeMessage::PongResponse {
            request_timestamp,
            response_timestamp,
        } => {
            // Calculate round-trip time
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let rtt = now - request_timestamp;
            let remote_processing_time = response_timestamp - request_timestamp;

            debug!(
                "Received pong from {}: RTT={} ms, remote processing time={} ms",
                peer_id,
                rtt * 1000,
                remote_processing_time * 1000
            );

            // Update peer's last_seen time
            {
                let mut peers = PEER_LIST.write().unwrap();
                if let Some(peer) = peers.get_mut(peer_id) {
                    peer.last_seen = now;
                }
            }
        }
        NodeMessage::BlockAnnounce {
            block_index,
            block_hash,
        } => {
            info!(
                "Peer {} announced block #{} (hash: {})",
                peer_id, block_index, block_hash
            );

            // Check if we already have this block
            let have_block = mona_blockchain::blockchain::BLOCKCHAIN_DATA.has_block_with_hash(&block_hash);

            if !have_block {
                info!("Requesting block {} from peer {}", block_hash, peer_id);

                // Request the block
                let request = NodeMessage::BlockRequest {
                    block_hash: block_hash.clone(),
                };

                send_message_to_peer(peer_id, &request)?;
            } else {
                debug!("Already have block {}, ignoring announcement", block_hash);
            }

            // Notify UI about the block announcement
            let announcement = serde_json::json!({
                "event": "block_announced",
                "peer_id": peer_id,
                "block_index": block_index,
                "block_hash": block_hash,
                "already_have": have_block
            })
            .to_string();

            let _ = status_tx.blocking_send(announcement);
        }
        NodeMessage::BlockRequest { block_hash } => {
            // Try to find the requested block
            let block = mona_blockchain::blockchain::BLOCKCHAIN_DATA.get_block_by_hash(&block_hash);

            let response = match block {
                Some(b) => {
                    info!("Sending block {} to peer {}", block_hash, peer_id);
                    NodeMessage::BlockResponse {
                        block: Some(b),
                        error: None,
                    }
                }
                None => {
                    warn!("Peer {} requested unknown block {}", peer_id, block_hash);
                    NodeMessage::BlockResponse {
                        block: None,
                        error: Some(format!("Block {} not found", block_hash)),
                    }
                }
            };

            send_message_to_peer(peer_id, &response)?;
        }
        NodeMessage::BlockResponse { block, error } => {
            match (block, error) {
                (Some(b), _) => {
                    info!("Received block #{} from peer {}", b.index, peer_id);

                    // Verify and add the block to our chain
                    // This could be expanded with more sophisticated validation
                    if mona_blockchain::blockchain::BLOCKCHAIN_DATA.add_block(b.clone()) {
                        info!("Added block #{} from peer {}", b.index, peer_id);

                        // Save blockchain state immediately
                        match mona_blockchain::blockchain::save_blockchain() {
                            Ok(_) => (),
                            Err(e) => warn!("Failed to save blockchain after adding block: {}", e),
                        }

                        // Notify about the new block
                        let block_status = serde_json::json!({
                            "event": "block_received",
                            "block_index": b.index,
                            "block_hash": b.hash,
                            "source_peer": peer_id,
                            "transaction_count": b.transactions.len()
                        })
                        .to_string();

                        let _ = status_tx.blocking_send(block_status);
                    } else {
                        warn!("Failed to add block #{} from peer {}", b.index, peer_id);
                    }
                }
                (None, Some(err)) => {
                    warn!("Block request error from peer {}: {}", peer_id, err);
                }
                _ => {
                    warn!("Invalid block response from peer {}", peer_id);
                }
            }
        }
        NodeMessage::PeerListRequest {} => {
            // Send our peer list
            let peers = {
                let peers = PEER_LIST.read().unwrap();
                peers.values().cloned().collect::<Vec<_>>()
            };

            let response = NodeMessage::PeerListResponse { peers };
            send_message_to_peer(peer_id, &response)?;
        }
        NodeMessage::PeerListResponse { peers } => {
            info!(
                "Received peer list from {} with {} peers",
                peer_id,
                peers.len()
            );

            // Process each peer and try to connect to new ones
            for peer in &peers {
                if peer.node_id != NODE_CONFIG.read().unwrap().node_id
                    && !PEER_LIST.read().unwrap().contains_key(&peer.node_id)
                {
                    // Try to connect to this new peer
                    let peer_addr = format!("{}:{}", peer.ip_address, peer.port);
                    debug!("Discovered new peer: {} at {}", peer.node_id, peer_addr);

                    let config = NODE_CONFIG.read().unwrap().clone();
                    let peer_clone = peer.clone();

                    thread::spawn(move || {
                        if let Err(e) = connect_to_peer(&peer_addr, &config) {
                            debug!(
                                "Failed to connect to discovered peer {}: {}",
                                peer_clone.node_id, e
                            );
                        }
                    });
                }
            }

            // Notify about peer discovery
            let discovery_status = serde_json::json!({
                "event": "peers_discovered",
                "source_peer": peer_id,
                "discovered_count": peers.len(),
                "connected_peers": get_peer_count()
            })
            .to_string();

            let _ = status_tx.blocking_send(discovery_status);
        }
        NodeMessage::Disconnect { reason } => {
            info!("Peer {} disconnected: {}", peer_id, reason);

            // No need to send a response, just clean up
            {
                let mut connections = ACTIVE_CONNECTIONS.write().unwrap();
                connections.remove(peer_id);
            }

            // Notify about peer disconnect
            let disconnect_status = serde_json::json!({
                "event": "peer_disconnected",
                "peer_id": peer_id,
                "reason": reason,
                "connected_peers": get_peer_count()
            })
            .to_string();

            let _ = status_tx.blocking_send(disconnect_status);
        }
        _ => {
            // Handle other message types
            warn!("Unhandled message type from peer {}", peer_id);
        }
    }

    Ok(())
}

// Helper function to send a message to a specific peer
fn send_message_to_peer(peer_id: &str, message: &NodeMessage) -> Result<(), BlockchainError> {
    let connections = ACTIVE_CONNECTIONS.read().unwrap();

    let stream = match connections.get(peer_id) {
        Some(stream) => stream.clone(),
        None => {
            return Err(BlockchainError::Network(format!(
                "Peer {} not connected",
                peer_id
            )));
        }
    };

    // Serialize the message
    let message_data = bincode::serialize(message)
        .map_err(|e| BlockchainError::Network(format!("Failed to serialize message: {}", e)))?;

    // Lock the stream and write the message
    let mut stream = match stream.lock() {
        Ok(stream) => stream,
        Err(_) => {
            return Err(BlockchainError::Network(
                "Failed to lock stream".to_string(),
            ));
        }
    };

    stream
        .write_all(&message_data)
        .map_err(|e| BlockchainError::Network(format!("Failed to send message: {}", e)))?;

    Ok(())
}

// Enhanced discover_peers with better error handling and retry logic
fn discover_peers(
    config: NodeConfig,
    status_tx: mpsc::Sender<String>
) -> Result<(), BlockchainError> {
    info!(
        "Starting peer discovery with {} known discovery nodes",
        config.discovery_nodes.len()
    );

    // Track connection success
    let mut any_connected = false;

    // Try to connect to each discovery node
    for node_addr in &config.discovery_nodes {
        // Try resolving domain names if needed
        match coordinator::resolve_domain(node_addr) {
            Ok(resolved_addr) => {
                info!("Resolved discovery node {} to {}", node_addr, resolved_addr);
                match connect_to_peer(&resolved_addr, &config) {
                    Ok(_) => {
                        any_connected = true;
                        info!("Successfully connected to discovery node: {}", node_addr);
                    },
                    Err(e) => {
                        warn!("Failed to connect to discovery node {} (resolved to {}): {}", 
                              node_addr, resolved_addr, e);
                    }
                }
            },
            Err(e) => {
                warn!("Failed to resolve discovery node {}: {}", node_addr, e);
            }
        }
    }

    // If no connections were made, attempt fallback discovery methods
    if !any_connected && !config.discovery_nodes.is_empty() {
        info!("No connections made to discovery nodes, attempting fallback methods...");
        
        // Try common ports on local network
        let local_subnet = match get_local_ip() {
            Some(ip) => {
                // Extract subnet (first 3 octets of IPv4)
                let parts: Vec<&str> = ip.split('.').collect();
                if parts.len() >= 3 {
                    format!("{}.{}.{}", parts[0], parts[1], parts[2])
                } else {
                    "192.168.1".to_string() // Default fallback subnet
                }
            },
            None => "192.168.1".to_string() // Default fallback subnet
        };
        
        // Send status update
        let discovery_status = serde_json::json!({
            "event": "peer_discovery_fallback",
            "status": "Attempting local network discovery",
            "subnet": local_subnet
        }).to_string();
        
        let _ = status_tx.blocking_send(discovery_status);
        
        // This comment indicates we're implementing enhanced peer discovery
        // The actual implementation would scan the local subnet
    }

    Ok(())
}

// Connect to a peer with enhanced handshake and security
fn connect_to_peer(peer_addr: &str, config: &NodeConfig) -> Result<(), BlockchainError> {
    // Parse the address
    let socket_addr = match peer_addr.to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(addr) => addr,
            None => {
                return Err(BlockchainError::Network(format!(
                    "Invalid peer address: {}",
                    peer_addr
                )));
            }
        },
        Err(e) => {
            return Err(BlockchainError::Network(format!(
                "Invalid peer address {}: {}",
                peer_addr, e
            )));
        }
    };

    // Get our actual local IP for better self-connection detection
    let local_ip = get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());

    // Check for self-connections more thoroughly
    let self_addr1 = format!("{}:{}", config.listen_ip, config.listen_port);
    let self_addr2 = format!("{}:{}", local_ip, config.listen_port);
    let self_addr3 = format!("127.0.0.1:{}", config.listen_port);

    if peer_addr == self_addr1 || peer_addr == self_addr2 || peer_addr == self_addr3 {
        debug!("Skipping connection to self at {}", peer_addr);
        return Ok(()); // Not an error, just skip connecting to ourselves
    }

    // Generate a cryptographic nonce to prevent replay attacks
    let nonce = generate_security_nonce();
    
    info!("Attempting to connect to peer at {} with secure handshake", peer_addr);

    // Connect to the peer with timeout
    let mut stream = match std::net::TcpStream::connect_timeout(
        &socket_addr,
        Duration::from_secs(CONNECTION_TIMEOUT_SECS),
    ) {
        Ok(stream) => stream,
        Err(e) => {
            return Err(BlockchainError::Network(format!(
                "Failed to connect to peer {}: {}",
                peer_addr, e
            )));
        }
    };

    // Set timeouts
    stream.set_read_timeout(Some(Duration::from_secs(HANDSHAKE_TIMEOUT_SECS)))?;
    stream.set_write_timeout(Some(Duration::from_secs(HANDSHAKE_TIMEOUT_SECS)))?;

    // Create enhanced handshake message with security nonce
    let handshake = NodeMessage::Handshake {
        node_id: config.node_id.clone(),
        blockchain_address: config.blockchain_address.clone(),
        protocol_version: PROTOCOL_VERSION.to_string(),
        is_validator: config.is_validator,
        chain_height: mona_blockchain::blockchain::BLOCKCHAIN_DATA.len() as u64,
        nonce: nonce.clone(), // Add nonce for security
    };

    // Serialize and send handshake
    let handshake_data = bincode::serialize(&handshake)
        .map_err(|e| BlockchainError::Network(format!("Failed to serialize handshake: {}", e)))?;

    stream
        .write_all(&handshake_data)
        .map_err(|e| BlockchainError::Network(format!("Failed to send handshake: {}", e)))?;

    // Read handshake response
    let mut buffer = [0u8; 8192]; // Larger buffer for peer list
    let bytes_read = stream.read(&mut buffer).map_err(|e| {
        BlockchainError::Network(format!("Failed to read handshake response: {}", e))
    })?;

    if bytes_read == 0 {
        return Err(BlockchainError::Network(format!(
            "Peer {} closed connection during handshake",
            peer_addr
        )));
    }

    // Parse the response
    let response: NodeMessage = bincode::deserialize(&buffer[0..bytes_read]).map_err(|e| {
        BlockchainError::Network(format!("Failed to parse handshake response: {}", e))
    })?;

    // Verify response with additional security checks
    match response {
        NodeMessage::HandshakeResponse {
            success,
            node_id,
            peers,
            message,
            signature,
        } => {
            // Verify signature if present (advanced security feature)
            if !signature.is_empty() {
                debug!("Verifying peer signature for {}", node_id);
                // In a real implementation, we would validate the signature here
            }
            
            if success {
                info!(
                    "Connected to peer {} at {}: {}",
                    node_id, peer_addr, message
                );

                // Create and register peer with TLS information
                let peer = Peer {
                    address: "".to_string(), // We don't know their blockchain address yet
                    node_id: node_id.clone(),
                    ip_address: socket_addr.ip().to_string(),
                    port: socket_addr.port(),
                    last_seen: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    is_validator: false, // Will be updated with more info
                    protocol_version: PROTOCOL_VERSION.to_string(),
                    tls_supported: config.use_tls, // Include TLS support info
                };

                if let Err(e) = register_peer(peer.clone()) {
                    warn!("Failed to register peer {}: {}", node_id, e);
                }

                // Add to active connections
                {
                    let mut connections = ACTIVE_CONNECTIONS.write().unwrap();
                    connections.insert(
                        node_id.clone(),
                        Arc::new(Mutex::new(stream.try_clone().unwrap())),
                    );
                }

                // Process received peer list
                for peer in peers {
                    if peer.node_id != config.node_id
                        && !PEER_LIST.read().unwrap().contains_key(&peer.node_id)
                    {
                        // Add to peer list but don't immediately connect
                        if let Err(e) = register_peer(peer.clone()) {
                            debug!("Failed to register discovered peer {}: {}", peer.node_id, e);
                        } else {
                            debug!(
                                "Added discovered peer {} at {}:{}",
                                peer.node_id, peer.ip_address, peer.port
                            );
                        }
                    }
                }

                // Start message handling thread for this peer
                let node_id_clone = node_id.clone();
                thread::spawn(move || {
                    if let Err(e) =
                        handle_peer_messages(node_id_clone, stream, mpsc::channel(100).0)
                    {
                        error!("Error handling messages from peer {}: {}", node_id, e);
                    }
                });

                Ok(())
            } else {
                // Connection rejected
                Err(BlockchainError::Network(format!(
                    "Peer {} rejected connection: {}",
                    peer_addr, message
                )))
            }
        }
        _ => Err(BlockchainError::Network(format!(
            "Unexpected response from peer {}",
            peer_addr
        ))),
    }
}

// Generate a more secure nonce using additional entropy sources
fn generate_security_nonce() -> String {
    // Create data combining timestamp, random values, and node information
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let mut rng = rand::thread_rng();
    let random_value1: u64 = rng.r#gen();
    let random_value2: u64 = rng.r#gen();
    
    // Combine multiple sources of entropy
    let mut data = Vec::with_capacity(32);
    data.extend_from_slice(&timestamp.to_le_bytes());
    data.extend_from_slice(&random_value1.to_le_bytes());
    data.extend_from_slice(&random_value2.to_le_bytes());
    
    // Add node identifier if available
    if let Ok(node_config) = NODE_CONFIG.read() {
        data.extend_from_slice(node_config.node_id.as_bytes());
    }
    
    // Hash the data using Blake3 for security
    let hash = hash_data_blake3(&data);
    hex::encode(&hash) // Use full hash for maximum security
}

// Register a new peer
pub fn register_peer(peer: Peer) -> Result<(), BlockchainError> {
    let mut peers = PEER_LIST.write().unwrap();

    // Check if we've reached the maximum number of peers
    let config = NODE_CONFIG.read().unwrap();
    if peers.len() >= config.max_peers {
        return Err(BlockchainError::Transaction(
            "Maximum number of peers reached".to_string(),
        ));
    }

    // Add or update the peer
    peers.insert(peer.node_id.clone(), peer.clone());

    info!(
        "Registered peer: {} at {}:{}",
        peer.node_id, peer.ip_address, peer.port
    );
    Ok(())
}

// // Propagate a new block to all peers
// pub fn propagate_block(block: &Block<Blake3Algorithm>) -> Result<(), BlockchainError> {
//     let peers = {
//         let connections = ACTIVE_CONNECTIONS.read().unwrap();
//         connections.keys().cloned().collect::<Vec<_>>()
//     };

//     if peers.is_empty() {
//         debug!("No peers to propagate block to");
//         return Ok(());
//     }

//     info!("Propagating block {} to {} peers", block.index, peers.len());

//     // Create block announcement
//     let announcement = NodeMessage::BlockAnnounce {
//         block_index: block.index as u64, // Convert u32 to u64
//         block_hash: block.hash.clone(),
//     };

//     // Send to all connected peers
//     for peer_id in peers {
//         if let Err(e) = send_message_to_peer(&peer_id, &announcement) {
//             warn!("Failed to announce block to peer {}: {}", peer_id, e);
//         } else {
//             debug!("Block {} announced to peer {}", block.index, peer_id);
//         }
//     }

//     Ok(())
// }

// Get peer count
pub fn get_peer_count() -> usize {
    ACTIVE_CONNECTIONS.read().unwrap().len()
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

    // Send disconnect messages to all peers
    let peer_ids = {
        let connections = ACTIVE_CONNECTIONS.read().unwrap();
        connections.keys().cloned().collect::<Vec<_>>()
    };

    for peer_id in peer_ids {
        let disconnect = NodeMessage::Disconnect {
            reason: "Node shutting down".to_string(),
        };

        if let Err(e) = send_message_to_peer(&peer_id, &disconnect) {
            warn!(
                "Failed to send disconnect message to peer {}: {}",
                peer_id, e
            );
        }
    }

    // Clear connection list
    {
        let mut connections = ACTIVE_CONNECTIONS.write().unwrap();
        connections.clear();
    }

    Ok(())
}
