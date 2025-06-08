# Kanari P2P Protocol Crate

This crate implements the peer-to-peer networking protocol for Kanari Network blockchain nodes. It provides the core functionality for node discovery, connection management, and data propagation across the network.

## Architecture

```
src/
├── lib.rs           # Public API and re-exports
├── node.rs          # Core P2P node implementation  
└── coordinator.rs   # Network coordination and broadcasting
```

## Key Features

- **Secure P2P Communication**: Encrypted connections with TLS support
- **Automatic Peer Discovery**: Bootstrap from known nodes and discover network peers
- **Block Propagation**: Efficient distribution of new blocks across the network
- **Transaction Broadcasting**: Propagate transactions to all connected peers
- **Network Statistics**: Real-time monitoring of network health and performance
- **Domain Resolution**: Support for connecting to peers via domain names
- **Fallback Mechanisms**: Robust connection handling with retry logic

## Core Components

### NodeConfig
Comprehensive configuration for P2P nodes:
- Network binding settings (IP, port)
- Peer discovery bootstrap nodes
- Security settings (TLS, certificates)
- Connection limits and timeouts

### Peer Management
- Thread-safe peer registry
- Automatic connection management
- Heartbeat/ping system for connection health
- Graceful handling of peer disconnections

### Protocol Messages
- Secure handshake with cryptographic nonces
- Block announcements and requests
- Transaction propagation
- Peer list exchange
- Ping/pong for connection health

## Usage

### Basic Node Setup

```rust
use p2p_protocol::{NodeConfig, start_node};
use tokio::sync::mpsc;

// Create configuration
let config = NodeConfig {
    node_id: "node-123".to_string(),
    blockchain_address: "0xabc...".to_string(),
    listen_ip: "0.0.0.0".to_string(),
    listen_port: 51303,
    discovery_nodes: vec![
        "bootstrap.kanari.site:51303".to_string(),
    ],
    max_peers: 50,
    is_validator: false,
    use_tls: false,
    cert_path: None,
    key_path: None,
};

// Start the node
let (status_tx, status_rx) = mpsc::channel(100);
start_node(config, status_tx).await?;
```

### Broadcasting Data

```rust
use p2p_protocol::{broadcast_block, broadcast_transaction};

// Broadcast a new block
broadcast_block(&new_block, &status_channel)?;

// Broadcast a transaction
broadcast_transaction(&transaction)?;
```

### Network Statistics

```rust
use p2p_protocol::get_network_statistics;

let stats = get_network_statistics();
println!("Connected peers: {}", stats.peers_connected);
println!("Blocks sent: {}", stats.blocks_sent);
```

## Network Protocol

### Connection Flow
1. **Bootstrap**: Connect to known discovery nodes
2. **Handshake**: Exchange node information and protocol version
3. **Peer Exchange**: Share known peer lists
4. **Data Sync**: Request missing blocks/transactions
5. **Maintenance**: Periodic ping/pong for connection health

### Security Features
- **Nonce-based Handshakes**: Prevent replay attacks
- **Protocol Version Checking**: Ensure compatibility
- **TLS Encryption**: Optional encrypted connections
- **Trusted Peer Lists**: Whitelist known good peers

## Configuration

### Default Settings
- **P2P Port**: 51303
- **Max Peers**: 50
- **Connection Timeout**: 5 seconds
- **Handshake Timeout**: 10 seconds
- **Ping Interval**: 30 seconds

### Bootstrap Nodes
Default discovery nodes for different networks:
- **DevNet**: `devnet.kanari.site:51303`
- **TestNet**: `testnet.kanari.site:51303`
- **MainNet**: `mainnet.kanari.site:51303`

## Error Handling

The crate provides comprehensive error handling through `BlockchainError`:
- Network connection failures
- Protocol incompatibilities
- Peer limits exceeded
- Invalid message formats

## Integration

This crate is designed to integrate with:
- **mona_blockchain**: For block and transaction data
- **mona_crypto**: For cryptographic operations
- **consensus_pos**: For consensus algorithm integration
- **common**: For shared utilities and configuration

## Examples

See the `examples/` directory for complete usage examples:
- Basic node setup
- Multi-node local network
- Production deployment configuration

## Development

### Testing
```bash
cargo test --package p2p-protocol
```

### Documentation
```bash
cargo doc --package p2p-protocol --open
```

### Benchmarks
```bash
cargo bench --package p2p-protocol
```

## Contributing

When contributing to this crate:
1. Maintain backward compatibility in public APIs
2. Add comprehensive tests for new features
3. Update documentation for any API changes
4. Follow the established error handling patterns

## License

This crate is part of the Kanari SDK and follows the same licensing terms.
