# KARI Blockchain Multi-Node Setup Guide

This guide explains how to set up and run multiple KARI blockchain nodes that can communicate with each other.

## Prerequisites

- Kanari SDK installed
- Multiple terminal windows or command prompts

## Starting Multiple Nodes

### Step 1: Start the First Node (Bootstrap Node)

Start your first node which will act as a bootstrap node:

```bash
cargo run --bin karid -- --node-address 0xYOUR_FIRST_WALLET_ADDRESS --port 30031
```

This node will listen for peer connections on port 30303 by default.

### Step 2: Start Additional Nodes

In a new terminal, start a second node and connect it to the first one:

```bash
cargo run --bin karid -- --node-address 0xYOUR_SECOND_WALLET_ADDRESS --port 30032 --peer 127.0.0.1:30303
```

For a third node:

```bash
cargo run --bin karid -- --node-address 0xYOUR_THIRD_WALLET_ADDRESS --port 30033 --peer 127.0.0.1:30303
```

## Node Configuration Parameters

- `--node-address`: The blockchain address of the node operator
- `--port`: HTTP RPC API port (default: 30031)
- `--peer`: Address of a peer node to connect to (format: IP:PORT)
- `--listen-port`: Port to listen for P2P connections (default: calculated from address)
- `--no-validator`: Run as a regular node without validation capabilities
- `--max-peers`: Maximum number of peer connections (default: 25)

## Staking Tokens

To participate in the network as a validator:

1. Ensure you have at least 32 KARI in your wallet
2. Use the RPC API to stake tokens:

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "stake_tokens",
  "params": {
    "address": "YOUR_WALLET_ADDRESS",
    "amount": 32.0,
    "password": "YOUR_WALLET_PASSWORD",
    "validator": true
  },
  "id": 1
}' http://127.0.0.1:YOUR_NODE_PORT
```

## Verifying Node Connections

To check if your nodes are properly connected:

1. Check the node logs for "Connected to peer" messages
2. Use the Node Status API:

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "get_node_status",
  "params": [],
  "id": 1
}' http://127.0.0.1:YOUR_NODE_PORT
```

## Checking Block Propagation

When a node creates a new block, it will automatically propagate to connected peers. 
You can verify block propagation by checking the block heights across different nodes:

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "blockchain_status",
  "params": [],
  "id": 1
}' http://127.0.0.1:YOUR_NODE_PORT
```

Compare the `block_height` values from different nodes - they should converge to the same value.

## Troubleshooting

If nodes aren't connecting:

1. Ensure your firewall isn't blocking the connections
2. Verify the IP and port are correct in the `--peer` argument
3. Check the node logs for connection errors
4. Restart the nodes if necessary

## Network Diagram

A typical KARI network setup:

```
                   ┌──────────────┐
                   │              │
                ┌──┤ Bootstrap    │
                │  │ Node (30303) │
                │  │              │
                │  └──────────────┘
                │
    ┌───────────┼───────────┐
    │           │           │
┌───▼───┐   ┌───▼───┐   ┌───▼───┐
│       │   │       │   │       │
│ Node2 │   │ Node3 │   │ Node4 │
│       │   │       │   │       │
└───────┘   └───────┘   └───────┘
```

Each node will maintain its own copy of the blockchain and participate in the consensus process.
