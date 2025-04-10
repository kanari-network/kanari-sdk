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

2. (Optional) Generate TLS certificates for secure communication:
   ```bash
   kari certificate generate
   ```

3. Start the node:
   ```bash
   kari start
   ```

4. Note the IP address displayed in the console (this will be needed for other nodes)

5. This node will listen for P2P connections on port 51303 by default

## Additional Nodes Setup

For each additional node:

1. Create a unique wallet on the new machine:
   ```bash
   kari keytool generate
   ```

2. (Optional) Generate certificates for this node:
   ```bash
   kari certificate generate
   ```

3. Start the node specifying the first node as a peer:
   ```bash
   kari start --peer <FIRST_NODE_IP>:51303
   ```

4. If running multiple nodes on the same machine, specify different RPC ports:
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

### Domain Name Configuration

In multi-node setups, using domain names instead of IP addresses provides stability and flexibility:

#### Benefits of Using Domains for Nodes
- Easier maintenance (IP changes don't require reconfiguring peers)
- More professional and memorable addresses
- Better security when combined with proper TLS certificates

#### Configuring Domain Names for Multiple Nodes

1. Follow a consistent naming scheme for your nodes:
   ```
   node1.your-domain.com
   node2.your-domain.com
   validator1.your-domain.com
   ```
   
   Or for Kanari's domains:
   ```
   devnet.kanari.site (development network)
   testnet.kanari.site (testing network)
   mainnet.kanari.site (production network)
   ```

2. Create A records for each node in your DNS configuration
3. Configure each node with its domain name:
   ```yaml
   # In ~/.kari/config.yaml for each node
   domain: "node1.your-domain.com"
   ```

4. Reference other nodes by domain name:
   ```bash
   kari start --peer node1.your-domain.com:51303 --peer node2.your-domain.com:51303
   ```

For detailed domain and DNS setup instructions, see the [Domain Configuration Guide](domain_setup_guide.md).

### Load Balancing Multiple Nodes

For high availability in a production environment, consider:

1. Running multiple nodes behind a load balancer:
   ```
   api.kanari.site → Load Balancer → Multiple Kari Nodes
   ```

2. Configure your DNS with multiple A records for automatic round-robin:
   ```
   api.kanari.site. IN A 203.0.113.1
   api.kanari.site. IN A 203.0.113.2
   api.kanari.site. IN A 203.0.113.3
   ```

3. For advanced setups, use georouting to direct users to the closest node:
   ```
   api-us.kanari.site → US-based nodes
   api-eu.kanari.site → Europe-based nodes
   api-asia.kanari.site → Asia-based nodes
   ```

This configuration provides redundancy and improved response times.

## Security Configuration

### TLS Certificate Setup

For secure node-to-node communication:

1. Generate certificates on each node:
   ```bash
   kari certificate generate
   ```

2. Check certificate status:
   ```bash
   kari certificate status
   ```

3. Enable TLS in the configuration:
   ```yaml
   use_tls: true
   ```

4. If OpenSSL is not installed:
   - On Linux: `apt-get install openssl` or `yum install openssl`
   - On macOS: `brew install openssl`
   - On Windows: Download from [slproweb.com](https://slproweb.com/products/Win32OpenSSL.html)

### Certificate Locations

Certificates are stored in the Kari data directory:
- Linux/macOS: `~/.kari/certs/`
- Windows: `%USERPROFILE%\.kari\certs\`

The following files are used:
- `node.crt` - The node's certificate
- `node.key` - The node's private key

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
   ```
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

### Certificate Issues

1. Make sure certificates exist in the expected location
2. Check that OpenSSL is installed and accessible
3. Verify that the ownership and permissions of certificate files are correct
4. If TLS is enabled but certificates are missing, TLS will be automatically disabled

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
