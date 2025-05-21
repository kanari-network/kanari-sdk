# Kari Multi-Node Setup Guide

This guide provides step-by-step instructions for setting up a Kari blockchain network with multiple nodes.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Network Architecture](#network-architecture)
3. [First Node Setup](#first-node-setup)
4. [Additional Nodes Setup](#additional-nodes-setup)
5. [Network Configuration](#network-configuration)
6. [Security Configuration](#security-configuration)
7. [Firewall Configuration](#firewall-configuration)
8. [Validator Node Setup](#validator-node-setup)
9. [Troubleshooting](#troubleshooting)

## Prerequisites

Before setting up a multi-node network, ensure you have:

- Kari CLI installed on all machines
- Each node has a unique wallet/address
- Network connectivity between machines
- Open ports in firewall (if across different machines)
- Sufficient storage and RAM on each machine

## Network Architecture

A typical Kari network consists of:

- **Bootstrap Node(s)**: Initial nodes that others can discover and connect to
- **Validator Nodes**: Stake tokens and participate in consensus
- **Regular Nodes**: Process and relay transactions without validation rights

## First Node Setup

The first node will serve as a bootstrap node for others:

1. Create a wallet:
   ```bash
   kari keytool generate
   ```

2. Start the node:
   ```bash
   kari start
   ```

3. Note the IP address displayed in the console (this will be needed for other nodes)

4. This node will listen for P2P connections on port 51303 by default

## Additional Nodes Setup

For each additional node:

1. Create a unique wallet on the new machine:
   ```bash
   kari keytool generate
   ```

2. Start the node specifying the first node as a peer:
   ```bash
   kari start --peer <FIRST_NODE_IP>:51303
   ```

3. If running multiple nodes on the same machine, specify different RPC ports:
   ```bash
   kari start --peer 192.168.1.100:51303 --port 30031
   ```

## Network Configuration

Each node's configuration is stored in a YAML file at:
- Windows: `%USERPROFILE%\.kari\config.yaml`
- Linux/macOS: `~/.kari/config.yaml`

You can manually edit this file to add multiple peers:

```yaml
chain_id: "kari-testnet-001"
rpc_port: 30030
address: "0x7a1c8f19cAE0A90d4A4E445793eB0BED2FaA9ecF"
peers:
  - "192.168.1.100:51303"
  - "192.168.1.101:51303"
use_tls: true  # Enable TLS if certificates are available
```

### Load Balancing Multiple Nodes

For high availability in a production environment, consider:

1. Running multiple nodes behind a load balancer
2. For advanced setups, use georouting to direct users to the closest node

This configuration provides redundancy and improved response times.

## Security Configuration

### Secure Communication

For secure communication between nodes:

1. Use a virtual private network (VPN) for node-to-node communication
2. Configure firewalls to allow only specific IP addresses
3. Use a reverse proxy for HTTPS support

## Firewall Configuration

When running nodes across different machines:

1. Open the P2P port (51303) on all machines:

   **Linux**:
   ```bash
   sudo ufw allow 51303/tcp
   ```

   **Windows**:
   ```
   netsh advfirewall firewall add rule name="Kari P2P" dir=in action=allow protocol=TCP localport=51303
   ```

2. If you need remote access to the RPC API, also open the RPC port:
   ```bash
   sudo ufw allow 30030/tcp
   ```

## Validator Node Setup

To configure a node as a validator:

1. Start the node and ensure it's connected to the network
2. Stake the required minimum tokens (32-200 KARI):
   ```bash
   curl -X POST http://localhost:30030 -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"stake_tokens","params":{"address":"YOUR_ADDRESS","amount":1000,"password":"YOUR_PASSWORD"},"id":1}'
   ```
3. Wait for the staking transaction to be confirmed
4. Restart the node for the validator status to take effect

## Troubleshooting

### Node Cannot Connect to Peers

1. Verify the peer IP and port are correct
2. Check firewall settings on both machines
3. Ensure the peer node is actually running
4. Try using the machine's local network IP (192.168.x.x) instead of localhost

### Connection Lost After Initial Connection

1. Check for network stability issues
2. Ensure the peer node hasn't reached its max_peers limit (default: 50)
3. Verify both nodes have the same chain_id in their configuration

### Different Blockchain States

If nodes show different blockchain heights:

1. The nodes may still be synchronizing - wait for the process to complete
2. If the issue persists, check logs for validation errors
3. As a last resort, reset the blockchain data on the problematic node and resync

### Performance Issues

1. Ensure each node has sufficient resources (CPU, RAM)
2. Consider reducing max_peers for resource-constrained devices
3. For validator nodes, ensure they have stable, low-latency connections
