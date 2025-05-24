# Kanari Network Blockchain Node Module

This module implements peer-to-peer networking for Kanari Network blockchain nodes.

## Architecture

- `mod.rs`: Main node implementation with peer discovery and networking
- `coordinator.rs`: Handles block and transaction propagation between nodes

## Key Components

### NodeConfig

Configuration for a node instance, including:
- `node_id`: Unique identifier for this node
- `blockchain_address`: Associated wallet address
- `listen_ip` and `listen_port`: Network binding settings (default port: 51303)
- `discovery_nodes`: Initial peers for bootstrap
- `max_peers`: Maximum number of connections
- `is_validator`: Whether this node participates in validation

### Peer Management

- `PEER_LIST`: Thread-safe registry of connected peers
- `register_peer()`: Add a new peer to the network
- `discover_peers()`: Connect to known nodes
- `propagate_block()`: Share new blocks with peers

## Usage in Code

```rust
// Create node configuration
let node_config = NodeConfig {
    node_id: "node-abc123".to_string(),
    blockchain_address: "0x123...".to_string(),
    listen_ip: "127.0.0.1".to_string(),
    listen_port: 51303,  // Default P2P port
    discovery_nodes: vec!["127.0.0.1:51303".to_string()],
    max_peers: 25,
    is_validator: true,
    use_tls: false,
    cert_path: None,
    key_path: None,
};

// Start node operations
start_node(node_config, status_channel)?;

// Later, propagate new blocks
propagate_block(&new_block)?;

// When shutting down
stop_node()?;
```

## Protocol Details

### Peer Discovery
1. On startup, connects to known discovery nodes
2. Exchanges peer information and blockchain state
3. Periodically refreshes peer list and removes stale connections

### Block Propagation
1. New blocks are announced to all peers
2. Peers request blocks they don't have
3. Nodes respond with requested blocks
4. Received blocks are validated before adding to the chain

### Transaction Propagation
1. Transactions are batched and announced to peers
2. Peers request transactions they don't have
3. Nodes respond with requested transactions
4. Transactions are validated and added to the mempool

## Port Configuration

- **RPC Port**: 30030 (default, configurable) - Used for API access
- **P2P Port**: 51303 (default) - Used for node-to-node communication

When running multiple nodes on one machine, each node must use different ports.

## Multi-Node Setup

To run multiple nodes that can communicate with each other:

1. Start the first node with the default configuration:
   ```
   kari start
   ```

2. Start additional nodes specifying the first node as a peer and a different RPC port:
   ```
   kari start --peer 192.168.1.100:51303 --port 30031
   ```

3. For nodes on different machines, ensure the P2P port (51303) is open in your firewall.