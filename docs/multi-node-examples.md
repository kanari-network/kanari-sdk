# Running Multiple Kanari Blockchain Nodes

This guide provides examples and commands for running a multi-node Kanari blockchain network.

## Prerequisites

- Kanari SDK installed
- Multiple terminal windows or command prompts
- At least one wallet created with `kari keytool generate`

## Running Your First Multi-Node Setup

### Step 1: Start the Bootstrap Node

In your first terminal window, start the bootstrap node:

```bash
kari start --port 30031
```

This will start a node on port 30031 for the RPC API and automatically 
assign a P2P port for peer connections.

### Step 2: Start a Second Node Connected to the First

In a second terminal window, start another node and connect it to the first one:

```bash
kari start --port 30032 --peer 127.0.0.1:30303
```

The `--peer` parameter points to the first node's P2P port, which is 30303 by default.

### Step 3: Start a Third Node (Optional)

For a more robust network, you can add a third node:

```bash
kari start --port 30033 --peer 127.0.0.1:30303
```

## Testing Your Multi-Node Network

### Check Block Propagation

1. Create a transaction on one node:

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "transfer",
  "params": {
    "from": "YOUR_WALLET_ADDRESS",
    "to": "ANOTHER_WALLET_ADDRESS",
    "amount": 1.0,
    "password": "YOUR_WALLET_PASSWORD"
  },
  "id": 1
}' http://127.0.0.1:30031
```

2. Check if the transaction appears on the other node:

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "blockchain_status",
  "params": [],
  "id": 1
}' http://127.0.0.1:30032
```

## Common Issues and Solutions

### Nodes Not Connecting

If your nodes aren't connecting:

1. Ensure your firewall isn't blocking connections
2. Verify that the peer address includes the correct port (default: 30303)
3. Check the terminal output for connection errors
4. Restart the nodes if necessary

### Port Already in Use

If you get an error that the port is already in use, choose a different port:

```bash
kari start --port 30034
```

## Advanced Configuration

You can connect to multiple peers by specifying the `--peer` parameter multiple times:

```bash
kari start --port 30034 --peer 127.0.0.1:30303 --peer 127.0.0.1:30304
```

## Creating a Local Test Network

For a full test network, you'll need:

1. At least 3 nodes for proper consensus
2. Different wallet addresses for each node
3. Stake tokens to participate in validation

Follow these steps:

1. Create 3 different wallets
2. Start 3 different nodes with different wallets
3. Connect the nodes to each other
4. Use the staking API to stake tokens for validation
5. Create transactions and observe them propagating through the network
