# Kanari Blockchain Node Running Guide

This guide explains how to run and manage a Kanari blockchain node, including configuration options, networking, and monitoring.

## Table of Contents
1. [Prerequisites](#prerequisites)
2. [Node Installation](#node-installation)
3. [Basic Node Operations](#basic-node-operations)
4. [Configuration Options](#configuration-options)
5. [Network Modes](#network-modes)
6. [Security Settings](#security-settings)
7. [Monitoring Your Node](#monitoring-your-node)
8. [Troubleshooting](#troubleshooting)
9. [Advanced Configuration](#advanced-configuration)

## Prerequisites

Before running a Kanari blockchain node, ensure you have:

- Installed the Kanari SDK and CLI tools
- Created at least one wallet (required for node operation)
- Sufficient disk space for blockchain data (~10GB recommended)
- Open network ports if running in network mode (default: 51303 for P2P, 30030 for RPC)

## Node Installation

### Installing the Kanari CLI

```bash
# Download and install the Kanari CLI
curl -sSL https://get.kanari.site | bash

# Verify installation
kari --version
```

### Creating a Wallet

A wallet is required to run a node. If you don't have one yet:

```bash
# Create a new wallet
kari keytool generate

# List available wallets
kari keytool list
```

## Basic Node Operations

### Starting a Node

To start a basic node with default settings:

```bash
kari server start
```

This will:
- Start a blockchain node with RPC service on port 30030
- Use your default wallet for blockchain operations
- Enable TLS encryption by default
- Allow external connections (network mode)

### Stopping a Node

To stop a running node, press `Ctrl+C` in the terminal where the node is running.

The node will automatically save the blockchain state when shutting down.

## Configuration Options

The Kanari node supports several configuration options:

### RPC Port

Change the API port (default: 30030):

```bash
kari server start --port 30031
```

### Wallet Selection

Specify which wallet to use:

```bash
kari server start --wallet 0x1234567890abcdef
```

### Connecting to Peers

Connect to specific peer nodes:

```bash
kari server start --peer node1.kanari.network:51303 --peer 192.168.1.100:51303
```

## Network Modes

### Localhost Only Mode

Run in localhost-only mode (no external connections):

```bash
kari server start --localhost true
```

This is useful for:
- Development and testing
- Running multiple nodes on one machine
- Enhanced security in private environments

### Network Mode

Run in network mode (default):

```bash
kari server start --localhost false
```

This allows your node to:
- Accept connections from other nodes
- Participate in the peer discovery protocol
- Propagate blocks and transactions across the network

## Security Settings

### TLS Encryption

By default, TLS encryption is enabled for node communications. To disable it:

```bash
kari server start --no-tls
```

### Generating TLS Certificates

If you need to regenerate TLS certificates:

```bash
kari server generate-certs
```

This will create self-signed certificates in your Kanari configuration directory.

## Monitoring Your Node

While your node is running, you'll see real-time updates in the console, including:

- New blocks being created or received
- Peer connections and disconnections
- Transaction processing
- Error messages and warnings

Example output:
