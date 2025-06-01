# Kanari Blockchain Node User Guide

This guide explains how to install, use, and manage Kanari blockchain nodes, including configuration settings, network connections, and status monitoring.

## Table of Contents
1. [Prerequisites](#prerequisites)
2. [Node Installation](#node-installation)
3. [Basic Usage](#basic-usage)
4. [Configuration](#configuration)
5. [Network Modes](#network-modes)
6. [Security Settings](#security-settings)
7. [Node Status Monitoring](#node-status-monitoring)
8. [Troubleshooting](#troubleshooting)
9. [Advanced Configuration](#advanced-configuration)

## Prerequisites

Before running a Kanari blockchain node, you need:

- Kanari SDK and CLI tools installed
- At least 1 wallet created (required for node operation)
- Sufficient disk space for blockchain data (~10GB recommended)
- Open network ports for network mode (defaults: 51303 for P2P, 30030 for RPC)

## Node Installation

### Installing Kanari CLI

```bash
# Download and install Kanari CLI
curl -sSL https://get.kanari.site | bash

# Verify installation
kari --version
```

### Creating a Wallet

A wallet is required to run the node. If you don't have one:

```bash
# Create a new wallet
kari keytool generate

# List existing wallets
kari keytool list

# Check if wallets exist
kari keytool check
```

### Initial Configuration

```bash
# Initialize kanari.yaml configuration
kari server init

# Check configuration file
cat ~/.kari/kanari_config/kanari.yaml
```

## Basic Usage

### Starting the Node

Start the node with default settings:

```bash
kari server start
```

This command will:
- Start the blockchain node with RPC service on port 30030
- Use the default wallet for blockchain operations
- Enable TLS encryption by default
- Allow external connections (network mode)

### Stopping the Node

Stop the node by pressing `Enter` in the terminal where the node is running.

The node will automatically save blockchain state when shutting down.

## Configuration

Kanari node supports various configuration options:

### Changing RPC Port

Change the API port (default: 30030):

```bash
kari server start --port 30031
```

### Selecting a Wallet

Specify which wallet to use:

```bash
kari server start --wallet 0x1234567890abcdef
```

### Connecting to Peers

Connect to other nodes:

```bash
kari server start --peer node1.kanari.network:51303 --peer 192.168.1.100:51303
```

### Combined Commands

```bash
kari server start --port 30031 --wallet myWallet --peer 192.168.1.100:51303 --localhost false --use-tls
```

## Network Modes

### Localhost Only Mode

Run in localhost-only mode (no external connections):

```bash
kari server start --localhost
```

Suitable for:
- Development and testing
- Running multiple nodes on the same machine
- Enhanced security in private environments

### Network Mode

Run in network mode (default):

```bash
kari server start --localhost false
```

Allows your node to:
- Accept connections from other nodes
- Participate in peer discovery
- Forward blocks and transactions through the network

## Security Settings

### TLS Encryption

TLS encryption is enabled by default. To disable:

```bash
kari server start --use-tls=false
```

## Node Status Monitoring

While the node is running, you'll see real-time updates in the console:

- Creating or receiving new blocks
- Peer connections and disconnections
- Transaction processing
- Error messages and warnings

Example output:
```
Using network configuration:
  Port: 30030
  Localhost only: false
  Use TLS: false
  Peers: ["192.168.1.100:51303"]

Node network information:
  RPC API:   192.168.1.50:30030 (HTTP)
  P2P:       192.168.1.50:51303

Node will connect to the following peers:
  - 192.168.1.100:51303

Block status will be shown below. Press Enter to stop the node.
Block #1 created successfully
Block #2 received from peer
Transaction processed: 0xabc123...
```

## Troubleshooting

### Common Issues

1. **No Wallet Found**
```bash
No wallet found!
Please create a wallet first using:
kari keytool generate
```

**Solution**: Create a new wallet as instructed

2. **Port Already in Use**
```bash
Failed to start RPC server: Address already in use
```

**Solution**: Change port or stop the program using that port
```bash
kari server start --port 30031
```

3. **Cannot Connect to Peers**
```bash
Warning: No peers configured. Running in standalone mode.
```

**Solution**: Check peer IP and port or use localhost mode

4. **Empty or Invalid Chain ID**
```yaml
chain_id: ''
```

**Cause**: May be caused by empty `CHAIN_ID` constant in code

**Solutions**: 
```bash
# Method 1: Delete config file and recreate
rm ~/.kari/kanari_config/kanari.yaml
kari server init

# Method 2: Manual edit
nano ~/.kari/kanari_config/kanari.yaml
# Change chain_id: '' to chain_id: 'kari-local-001'

# Method 3: Force system auto-fix
kari server start --port 30030

# Method 4: Switch environment and back
kari env switch dev
kari env switch local
```

**Note**: After fixing, the system will display "Updated chain_id to: kari-local-001"

5. **Environment Mismatch with Chain ID**
```bash
# Check current environment
kari env list

# Switch environment to match usage
kari env switch local  # for development
kari env switch test   # for testing
```

### Status Checks

```bash
# Check configuration
cat ~/.kari/kanari_config/kanari.yaml

# Check current chain_id
grep "chain_id:" ~/.kari/kanari_config/kanari.yaml

# Check wallets
kari keytool list

# Check ports in use
netstat -an | grep :30030
```

## Advanced Configuration

### kanari.yaml Configuration File

The main configuration file is located at `~/.kari/kanari_config/kanari.yaml`:

```yaml
keystore_path: "C:\\Users\\YourName\\.kari\\kanari_config\\kanari.keystore"
active_address: "0xd00bdd88b00cb017950243f92afa3c1d0a0b75f22f5f4f738aebb58133235599"
envs:
  - alias: "local"
    rpc: "http://127.0.0.1:30030"
    ws: "ws://127.0.0.1:30031"
  - alias: "dev"
    rpc: "https://dev-seed.kanari.site"
    ws: "wss://dev-seed.kanari.site/websocket"
  - alias: "test"
    rpc: "https://test-seed.kanari.site"
    ws: "wss://test-seed.kanari.site/websocket"
  - alias: "main"
    rpc: "https://main-seed.kanari.site"
    ws: "wss://main-seed.kanari.site/websocket"
active_env: "local"
localhost_only: false
use_tls: true
rpc_port: 30030
chain_id: "kari-local-001"  # Must not be empty
peers: []
```

**Important Notes**: 
- `chain_id` must not be empty. If empty, delete the file and recreate
- For different environments, chain_id values are:
  - `local`: `kari-local-001`
  - `dev`: `kari-dev-001`
  - `test`: `kari-testnet-001`
  - `main`: `kari-mainnet-001`

### Configuration Management

```bash
# View all environments
kari env list

# Switch environment
kari env switch test
kari env switch dev
kari env switch main

# Add new environment
kari env add local_test http://127.0.0.1:30035

# Remove environment (cannot remove built-in environments: local, dev, test, main)
kari env remove local_test

# Change active wallet
kari keytool select

# List all wallets
kari keytool list
```

### Environment Management

Environments are different network configurations:

```bash
# View current and all environments
kari env list

# Example output:
# AVAILABLE ENVIRONMENTS
# NAME            RPC URL                                  STATUS
# local           http://127.0.0.1:30030                  ACTIVE
# dev             https://dev-seed.kanari.site
# test            https://test-seed.kanari.site
# main            https://main-seed.kanari.site

# Switch to testnet
kari env switch test

# Switch to mainnet
kari env switch main

# Return to local development
kari env switch local
```

### Wallet Management

```bash
# View all wallets
kari keytool list

# Select wallet to use
kari keytool select

# Create new wallet
kari keytool generate

# Import wallet from private key
kari keytool privatekey

# Import wallet from seed phrase
kari keytool import

# Check balance
kari keytool balance

# Transfer funds
kari keytool transfer
```

### Mnemonic and Session Management

```bash
# Save mnemonic phrase
kari keytool mnemonic save

# Load mnemonic phrase
kari keytool mnemonic load

# Check mnemonic status
kari keytool mnemonic status

# Remove mnemonic (careful!)
kari keytool mnemonic remove

# Manage session keys
kari keytool session set api_key your_api_key
kari keytool session get api_key
kari keytool session remove api_key
kari keytool session clear
```

### Backup Data

```bash
# Backup wallet
cp ~/.kari/kanari_config/kanari.keystore ~/backup/

# Backup configuration
cp ~/.kari/kanari_config/kanari.yaml ~/backup/

# Backup blockchain data
cp -r ~/.kari/blockchain/ ~/backup/
```

### Production Usage

1. **Use TLS encryption**:
```bash
kari server start --use-tls
```

2. **Configure firewall rules**:
```bash
# Allow RPC port for specific IPs only
ufw allow from 192.168.1.0/24 to any port 30030

# Allow P2P port for peer nodes
ufw allow 51303
```

3. **Use reverse proxy** for RPC API:
```nginx
server {
    listen 443 ssl;
    server_name api.yournode.com;
    
    location / {
        proxy_pass http://127.0.0.1:30030;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### Docker Usage

```dockerfile
FROM ubuntu:22.04
COPY kari /usr/local/bin/
COPY kanari.yaml /root/.kari/kanari_config/
EXPOSE 30030 51303
CMD ["kari", "server", "start"]
```

### Monitoring and Logging

```bash
# Start node with logging
kari server start 2>&1 | tee node.log

# Monitor with systemctl (for Linux service)
sudo systemctl status kanari-node
sudo journalctl -u kanari-node -f
```

## Updates

```bash
# Update Kanari CLI
curl -sSL https://get.kanari.site | bash

# Check for new version
kari --version

# Update configuration (if needed)
kari server init --update
```

For additional information and support, please visit [Kanari Documentation](https://docs.kanari.site) or contact [Community Discord](https://discord.gg/kanari)
