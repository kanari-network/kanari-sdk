# Kanari Multi-Node Setup Guide

Guide for running Kanari blockchain with multiple nodes using libp2p

## Features

- ✅ P2P networking with libp2p
- ✅ Automatic peer discovery with mDNS (for local network)
- ✅ Kademlia DHT for peer discovery
- ✅ Gossipsub protocol for message propagation
- ✅ Block and transaction synchronization
- ✅ Configurable P2P and RPC ports

## Installation

1. Build project:

```bash
cargo build --release
```

1. Generated file:

```
target/release/kanari-node
```

## Running Multi-Node

### Node 1 (Bootstrap Node / Authority 0x1)

```bash
cargo run --bin kanari-node -- start --p2p-port 19000 --rpc-port 19001 --data-dir data/node1 --authority-id 0x1 --authorities 0x1,0x2,0x3,0x4,0x5
```

### Node 2 (Authority 0x2)

```bash
cargo run --bin kanari-node -- start --p2p-port 19010 --rpc-port 19011 --data-dir data/node2 --authority-id 0x2 --authorities 0x1,0x2,0x3,0x4,0x5
```

### Node 3 (Authority 0x3)

```bash
cargo run --bin kanari-node -- start --p2p-port 19020 --rpc-port 19021 --data-dir data/node3 --authority-id 0x3 --authorities 0x1,0x2,0x3,0x4,0x5
```

## Connecting Nodes

### Automatic (Local Network)

If nodes are running on the same network, they will discover each other automatically via mDNS

## Usage Examples

### 1. View Blockchain Stats

```bash
kanari-node stats
```

### 2. View Account Information

```bash
kanari-node account 0x1
```

### 3. View Block Information

```bash
kanari-node block 0
```

### 4. List Wallets

```bash
kanari-node list-wallets
```

### 5. View All Options

```bash
kanari-node start --help
```

**Important options:**

- `--p2p-port <PORT>` - Set port for P2P networking (default: 19000)
- `--rpc-port <PORT>` - Set port for RPC server (default: 19001)
- `--rpc-host <HOST>` - Set host/IP for RPC (default: 0.0.0.0)
- `--data-dir <PATH>` - Set location for storing blockchain and state data
- `--relay-server` - Enable relay server mode to help nodes behind NAT
- `--bootstrap <MULTIADDR>` - Connect to bootstrap peer (can specify multiple times)

## P2P Network Structure

```
┌─────────────────────────────────────────────┐
│           Kanari P2P Network                │
├─────────────────────────────────────────────┤
│                                             │
│  Node 1 (19000) ←→ Node 2 (19010)          │
│       ↑                 ↓                   │
│       └─────→ Node 3 (19020)               │
│                                             │
└─────────────────────────────────────────────┘
```

### Protocols Used

1. **Gossipsub** - For broadcasting blocks and transactions
   - Topic: `kanari/blocks`
   - Topic: `kanari/transactions`
   - Topic: `kanari/peers`

2. **mDNS** - Auto-discovery in local network

3. **Kademlia DHT** - Distributed peer discovery

4. **Noise Protocol** - Encrypted transport

5. **Yamux** - Stream multiplexing

6. **DCUtR** - Direct Connection Upgrade through Relay for NAT traversal

7. **Identify** - Peer information exchange (protocol version, addresses)

8. **Ping** - Connection keep-alive and latency measurement

9. **Relay** - Circuit relay protocol for nodes behind strict NAT

## P2P Message Types

```rust
pub enum P2PMessage {
    NewTransaction(String),  // New transaction
    NewBlock(String),        // New block
    BlockRequest(u64),       // Request block by height
    BlockResponse(String),   // Response with block data
    PeerInfo(PeerInfoMsg),   // Peer information (height, peer_id)
}
```

## Block Synchronization

When a new node joins the network:

1. Node receives `PeerInfo` from other peers
2. Compares height with local blockchain
3. If lower, sends `BlockRequest` to request missing blocks
4. Receives `BlockResponse` and applies blocks to local chain

## NAT Traversal and Hole Punching

Kanari supports connections between nodes behind NAT/firewall through:

### DCUtR (Direct Connection Upgrade through Relay)

- **Hole punching** - Creates direct connections between nodes behind NAT
- **Identify protocol** - Exchanges peer information and addresses
- **Ping protocol** - Keep-alive connections and latency checking

### How It Works

1. Nodes use **Identify protocol** to exchange information and addresses
2. **Kademlia DHT** helps discover peers across networks
3. **DCUtR** attempts to create direct connections (hole punching)
4. If successful, nodes communicate directly without relay

**Note:** For strict NAT/symmetric NAT where hole punching cannot work, additional relay server may be needed (see below)

### Relay Server Mode

For nodes behind strict NAT or symmetric NAT where hole punching cannot work, you can use relay server mode:

```bash
kanari-node start --p2p-port 19000 --rpc-port 19001 --relay-server
```

**Features:**

- Accepts reservation requests from nodes that want to use relay
- Creates circuit relay between nodes
- Helps nodes behind NAT communicate even when hole punching fails

**Recommendation:** Run relay server on a node with public IP or on a network accessible from outside

## Port Configuration

| Service | Default Port | Customization |
|---------|--------------|---------------|
| P2P     | 19000        | `--p2p-port`  |
| RPC     | 19001        | `--rpc-port`  |

## Data Directory Configuration

Each node should have a separate data directory to prevent data conflicts:

### Windows

```bash
--data-dir C:\Users\<Username>\.kanari\kanari-db\node1
--data-dir C:\Users\<Username>\.kanari\kanari-db\node2
--data-dir C:\Users\<Username>\.kanari\kanari-db\node3
```

### Linux/macOS

```bash
--data-dir ~/.kanari/kanari-db/node1
--data-dir ~/.kanari/kanari-db/node2
--data-dir ~/.kanari/kanari-db/node3
```

**Note:** If `--data-dir` is not specified, the system will use the default directory which may cause nodes to share data

## Advanced Setup Examples

### Using PowerShell Scripts (Windows)

This folder contains PowerShell scripts to help run multi-node:

#### 1. Setup and View Configuration Information

```powershell
.\setup-multi-node.ps1
```

This script will:

- Create data directories for each node
- Display configuration information for each node
- Show commands for running nodes

#### 2. Run Each Node

```powershell
# Terminal 1
.\start-node.ps1 -NodeId 1

# Terminal 2
.\start-node.ps1 -NodeId 2

# Terminal 3
.\start-node.ps1 -NodeId 3
```

### Running 6 Nodes Simultaneously (Manual)

```bash
# Terminal 1 (Authority 0x1)
cargo run --bin kanari-node -- start --p2p-port 19000 --rpc-port 19001 --data-dir data/node1 --authority-id 0x1 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 2 (Authority 0x2)
cargo run --bin kanari-node -- start --p2p-port 19010 --rpc-port 19011 --data-dir data/node2 --authority-id 0x2 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 3 (Authority 0x3)
cargo run --bin kanari-node -- start --p2p-port 19020 --rpc-port 19021 --data-dir data/node3 --authority-id 0x3 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 4 (Authority 0x4)
cargo run --bin kanari-node -- start --p2p-port 19030 --rpc-port 19031 --data-dir data/node4 --authority-id 0x4 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 5 (Authority 0x5)
cargo run --bin kanari-node -- start --p2p-port 19040 --rpc-port 19041 --data-dir data/node5 --authority-id 0x5 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 6 (Authority 0x6)
cargo run --bin kanari-node -- start --p2p-port 19050 --rpc-port 19051 --data-dir data/node6 --authority-id 0x6 --authorities 0x1,0x2,0x3,0x4,0x5,0x6
```

## RPC Endpoints

- Local (loopback):
  - Node 1: `http://127.0.0.1:19001`
  - Node 2: `http://127.0.0.1:19011`
  - Node 3: `http://127.0.0.1:19021`

- LAN (reachable from other machines on your network):
  - Node 1: `http://<machine_ip>:19001`
  - Node 2: `http://<machine_ip>:19011`
  - Node 3: `http://<machine_ip>:19021`

To expose RPC to the LAN, start each node with either `--rpc-host 0.0.0.0` (bind all interfaces) or `--rpc-host <machine_ip>` (bind a single interface). Example:

```powershell
kanari-node start --p2p-port 19000 --rpc-port 19001 --rpc-host 0.0.0.0 --data-dir C:\Users\Pukpuy\.kanari\kanari-db\node1
```

Security note: binding RPC to all interfaces exposes the API to your local network — ensure your firewall and network policies allow or block access as intended.

## Monitoring the Network

Check logs to monitor P2P events:

```
INFO kanari_node: Node Peer ID: 12D3KooW...
INFO kanari_node: P2P network initialized on port 19000
INFO kanari_node: Listening on /ip4/0.0.0.0/tcp/19000
INFO kanari_node: Discovered peer: 12D3KooW... at /ip4/...
INFO kanari_node: Connection established with 12D3KooW...
INFO kanari_node: Received transaction from network: 0x...
INFO kanari_node: Received new block #123 from network
```

## Troubleshooting

### Problem: Nodes Cannot Find Each Other

**Solution:**

1. Check that ports are not duplicated
2. Check firewall settings
3. Try manual bootstrap with `--bootstrap`

### Problem: Block Sync Not Working

**Solution:**

1. Check logs for errors
2. Verify that blocks are being broadcast
3. Restart nodes to re-sync

## Architecture

```
┌──────────────────────────────────────────┐
│         Kanari Node                      │
├──────────────────────────────────────────┤
│                                          │
│  ┌────────────┐      ┌──────────────┐    │
│  │    RPC     │      │  P2P Network │    │
│  │  Server    │      │  (libp2p)    │    │
│  └──────┬─────┘      └──────┬───────┘    │
│         │                   │            │
│         └────┬──────────────┘            │
│              │                           │
│      ┌───────▼────────┐                  │
│      │                │                  │
│      │     Engine     │                  │
│      └───────┬────────┘                  │
│              │                           │
│      ┌───────▼────────┐                  │
│      │ Move Runtime   │                  │
│      │ + State        │                  │
│      └────────────────┘                  │
│                                          │
└──────────────────────────────────────────┘
```

## Merkle Tree Architecture

Kanari uses **2 types** of Merkle trees:

### 1. Sparse Merkle Tree (SMT) - State Storage

- **Location**: `crates/smt/`
- **Purpose**: Account state verification and proofs
- **Used for**: Account balances, modules, objects, state root
- **Storage**: Persistent in RocksDB

### 2. Transaction Merkle Tree - Block Verification  

- **Location**: `crates/kanari-core/src/blockchain/merkle.rs`
- **Purpose**: Light client transaction verification
- **Used for**: Block header merkle root, transaction inclusion proofs
- **Storage**: In-memory, recalculated per block

See [DOCS/MERKLE_TREES.md](../../../DOCS/MERKLE_TREES.md) for more details

## License

Apache-2.0
