# Kari Node Configuration Guide

This document provides comprehensive instructions for configuring a Kari blockchain node, with special focus on multi-node setups and peer-to-peer networking.

## Table of Contents

1. [Basic Configuration](#basic-configuration)
2. [Network Configuration](#network-configuration)
3. [Peer-to-Peer (P2P) Settings](#peer-to-peer-p2p-settings)
4. [RPC Server Configuration](#rpc-server-configuration)
5. [Wallet Management](#wallet-management)
6. [Certificate Management](#certificate-management)
7. [Staking Configuration](#staking-configuration)
8. [Security Settings](#security-settings)
9. [Firewall Configuration](#firewall-configuration)
10. [Advanced Options](#advanced-options)
11. [Troubleshooting](#troubleshooting)

## Basic Configuration

### Configuration File

The Kari node stores its configuration in a YAML file located at:
- Windows: `%USERPROFILE%\.kari\config.yaml`
- Linux/macOS: `~/.kari/config.yaml`

Basic configuration example:

```yaml
chain_id: "kari-testnet-001"
rpc_port: 30030
address: "0x7a1c8f19cAE0A90d4A4E445793eB0BED2FaA9ecF"
use_tls: false
```

### First-time Setup

When starting the node for the first time:

1. Create a wallet using `kari keytool generate`
2. Run `kari start` to initialize the configuration
3. The node will ask for configuration values or use defaults

## Network Configuration

### Basic Network Parameters

The following network settings can be configured:

| Parameter | Description | Default |
|-----------|-------------|---------|
| `rpc_port` | Port for RPC API server | `30030` |
| `peers` | List of peer nodes to connect with | `[]` |
| `chain_id` | Blockchain network identifier | `"kari-testnet-001"` |
| `use_tls` | Whether to use TLS for secure connections | `false` |
| `domain` | Domain name for the node (optional) | `null` |

### Domain Configuration

Using a domain name for your node is recommended for production environments:

```yaml
# In ~/.kari/config.yaml
domain: "devnet.kanari.site"
```

#### Setting Up Official Kanari Domains

If you're setting up an official Kanari network domain like "devnet.kanari.site":

1. Request access from the Kanari DevOps team
2. Provide your node's public IP address
3. The DevOps team will configure the DNS A record to point to your node
4. Update your node configuration to use the domain

#### Setting Up Your Own Domain

If you're setting up your own domain:

1. Register a domain with any registrar (Namecheap, GoDaddy, etc.)
2. Create an A record for your subdomain pointing to your server's IP address
3. Configure your node with the domain name
4. Set up proper TLS certificates for the domain using `kari certificate generate`

#### Official Kanari Network Domains

The Kanari Network uses these standard domain names:

| Domain | Purpose | Environment |
|--------|---------|-------------|
| `devnet.kanari.site` | Development network | Unstable, frequent resets |
| `testnet.kanari.site` | Testing network | More stable, occasional resets |
| `mainnet.kanari.site` | Production network | Stable, no resets |

When connecting to these networks, you can use the domain name directly:

```bash
kari start --peer devnet.kanari.site:51303
```

For complete domain setup instructions, see the [Domain Configuration Guide](domain_setup_guide.md).

### Running With Custom Port

To use a non-default port:

```bash
kari start --port 8545
```

This will update your configuration file with the new port.

## Peer-to-Peer (P2P) Settings

### Connecting to Peer Nodes

To connect your node to existing peer nodes:

```bash
kari start --peer 192.168.1.100:51303 --peer 192.168.1.101:51303
```

The peers will be saved in your configuration file.

### Manual Peer Configuration

You can manually edit the configuration file to add peers:

```yaml
chain_id: "kari-testnet-001"
rpc_port: 30030
address: "0x7a1c8f19cAE0A90d4A4E445793eB0BED2FaA9ecF"
peers:
  - "192.168.1.100:51303"
  - "public-node.kanari.site:51303"
  - "validator1.kanari.site:51303"
```

### Peer Discovery

Kari nodes perform automatic peer discovery:

1. When a node connects to a peer, it receives that peer's known peers
2. Your node will periodically try to connect to these discovered peers
3. Stale peers (not seen for 5 minutes) are removed from the peer list

### Port Forwarding

For nodes behind NAT (home networks/routers):

1. Configure your router to forward the P2P port (default: 51303)
2. Forward the RPC port (default: 30030) if external access is needed
3. Set up static IP for your node or use DHCP reservation

## RPC Server Configuration

The RPC server provides API access to your node.

### RPC Server Settings

| Parameter | Description | Default |
|-----------|-------------|---------|
| `rpc_port` | Port for JSON-RPC server | `30030` |
| `api_enabled` | Whether the API is enabled | `true` |

### CORS Settings

By default, the RPC server allows connections from any origin (`Access-Control-Allow-Origin: *`). This can be modified in the source code if needed for security.

### Health Endpoints

The RPC server provides health check endpoints:
- `http://NODE_IP:PORT/health` - Basic health check
- `http://NODE_IP:PORT/ready` - Readiness check

## Wallet Management

A wallet is required to run a Kari node.

### Creating a Wallet

```bash
kari keytool generate
```

This will:
1. Create a new wallet file in your data directory
2. Ask for a password to encrypt the wallet
3. Display the wallet address

### Managing Multiple Wallets

You can create multiple wallet files:

```bash
kari keytool generate
```

List your wallets:

```bash
kari keytool list
```

The node will use the wallet specified in your config file (`address`).

## Certificate Management

Kari supports TLS certificate management for secure node-to-node and client-node communication.

### Certificate Commands

| Command | Description |
|---------|-------------|
| `kari certificate generate` | Generate self-signed TLS certificates |
| `kari certificate status` | Check the status of current certificates |
| `kari certificate help` | Display certificate command help |

### Generating Certificates

To generate self-signed certificates:

```bash
kari certificate generate
```

This will:
1. Create a certificates directory in your Kari data directory
2. Generate a self-signed certificate and private key
3. Store them as `node.crt` and `node.key`

### Certificate Location

Certificates are stored in your Kari data directory:
- Windows: `%USERPROFILE%\.kari\certs\`
- Linux/macOS: `~/.kari/certs/`

### Enabling TLS

To enable TLS, modify your config file:

```yaml
use_tls: true
```

Or regenerate your configuration with TLS enabled.

## Staking Configuration

Staking allows nodes to participate in consensus and earn rewards.

### Validator Requirements

To run a validator node:

1. Have a minimum stake of at least `VALIDATOR_STAKING_MINIMUM_KARI` (1,000 KARI)
2. Stake your tokens using:
   ```
   # Through RPC API
   curl -X POST http://localhost:30030 -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"stake_tokens","params":{"address":"YOUR_ADDRESS","amount":1000,"password":"YOUR_PASSWORD"},"id":1}'
   ```

### Normal Node Requirements

To run a regular node:

1. Have a minimum stake of at least `NODE_STAKING_MINIMUM_KARI` (10 KARI)
2. Stake using the same methods as for validators

## Security Settings

### TLS Configuration

For fully secure connections:

1. Generate certificates using `kari certificate generate`
2. Enable TLS in your configuration (`use_tls: true`)
3. For public-facing nodes, use a reverse proxy like Nginx or Caddy with valid certificates

### HTTPS Setup

The built-in server supports HTTP only, but you can set up HTTPS using a reverse proxy:

1. Configure a reverse proxy (Nginx, Caddy, etc.)
2. Point it to your Kari node's RPC port
3. Set up certificates in the proxy

### Password Security

Your wallet password is used to:
1. Sign transactions
2. Protect your private keys
3. Authenticate certain node operations

Use a strong password and keep it secure.

## Firewall Configuration

For a node accessible from the internet:

### Required Ports

| Port | Protocol | Description | External Access? |
|------|----------|-------------|-----------------|
| 30030 (default) | TCP | RPC API Server | Optional, for remote API calls |
| 51303 (default) | TCP | Peer-to-peer communication | Required for multi-node |

### Firewall Rules (iptables)

```bash
# Allow P2P communication
iptables -A INPUT -p tcp --dport 51303 -j ACCEPT

# Allow RPC server (if you want it accessible externally)
iptables -A INPUT -p tcp --dport 30030 -j ACCEPT
```

### Firewall Rules (Windows)

1. Open Windows Firewall with Advanced Security
2. Create new Inbound Rule
3. Select "Port" and enter your P2P port (51303) and RPC port (30030)
4. Allow the connection and apply the rule

## Advanced Options

### Node ID Configuration

Each node has a unique ID generated automatically. This ID is used for P2P communication and is not the same as your blockchain address.

To view your node ID:
1. Start the node
2. Look for `node_id` in the startup logs

### Custom Data Directory

You can change the data directory location with the environment variable:

```bash
# Windows
set KARI_DATA_DIR=D:\kari-data

# Linux/macOS
export KARI_DATA_DIR=/path/to/kari-data
```

### Blockchain Explorer

To view the blockchain explorer:
1. Ensure your node is running
2. Access: `http://NODE_IP:PORT/explorer`

Note: The explorer is a web-based interface that connects to your node's RPC API.

## Troubleshooting

### Common Issues

#### Cannot connect to peers

1. Verify the peer address is correct
2. Check if the peer node is running
3. Ensure firewalls allow connections to the P2P port (51303)
4. Verify network connectivity between nodes

#### RPC server fails to start

1. Check if another process is using the specified port
2. Try a different port with `kari start --port <PORT>`
3. Ensure you have sufficient permissions

#### Certificate issues

1. Run `kari certificate status` to check certificate status
2. Verify that OpenSSL is installed if generating certificates
3. Check that certificate paths in configuration are correct
4. If TLS is enabled but certificates are missing, TLS will be automatically disabled

#### Node fails to sync

1. Verify that connected peers are on the same network (`chain_id`)
2. Check for blockchain data corruption
3. Restart the node with `kari start`

#### Out of sync with network

1. Delete blockchain data (backup your wallets first!)
2. Reconnect to trusted peers
3. Let the node resync from scratch

### Log Files

Log files are stored in:
- Windows: `%USERPROFILE%\.kari\logs\`
- Linux/macOS: `~/.kari/logs/`

### Getting Help

For additional support:
- Visit the documentation site: `kari info`
- Join the community Discord: `https://discord.gg/XyxZQNWhbF`
- Submit issues on GitHub: `https://github.com/kanari-network`
