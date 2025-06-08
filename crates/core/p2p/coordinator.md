# Blockchain Network Coordinator

The `coordinator.rs` module is a key component of the Kanari Network blockchain's peer-to-peer networking system. It handles high-level orchestration of node communication across the network.

## Core Responsibilities

### 1. Message Broadcasting
- **Block Broadcasting**: Distributes newly mined or validated blocks to all connected peers
- **Transaction Broadcasting**: Propagates new transactions to peers for inclusion in upcoming blocks

### 2. Network Statistics
- Tracks metrics such as blocks sent/received, transaction counts, and peer connections
- Provides performance insights and network health monitoring

### 3. Network Coordination 
- Manages peer-to-peer message distribution
- Ensures consistent blockchain state across the network
- Facilitates blockchain data synchronization between nodes

## Key Functions

| Function | Purpose |
|----------|---------|
| `broadcast_transaction()` | Sends new transactions to all connected peers |
| `broadcast_block()` | Announces new blocks to the network |
| `get_network_statistics()` | Reports current network metrics |
| `is_connected_to_peer()` | Verifies connection status to a specific peer |
| `get_connected_peers()` | Lists all connected node IDs |
| `request_sync_from_peer()` | Initiates blockchain synchronization |

## Architecture

The coordinator operates as a layer between the core blockchain logic and the low-level node networking implementation:

```
+------------------+
| Blockchain Logic |
+--------+---------+
         |
+--------v---------+
|   Coordinator    | <- You are here
+--------+---------+
         |
+--------v---------+
| P2P Network Layer|
+------------------+
```

## Usage Example

```rust
// Broadcasting a new block to all peers
let status_channel = mpsc::channel(10).0;
coordinator::broadcast_block(&new_block, &status_channel)?;

// Getting network statistics
let stats = coordinator::get_network_statistics();
println!("Connected peers: {}", stats.peers_connected);
```

## Relationship with Node Module

While the main `mod.rs` in the node directory provides low-level P2P connection management (establishing connections, maintaining peer lists, handling raw messages), the coordinator module focuses on higher-level orchestration - ensuring blockchain data propagates correctly through the network and tracking network health metrics.