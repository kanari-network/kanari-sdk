# Kanari Network Node Setup Guide

This guide explains how to set up and run Kanari blockchain nodes in a real network environment.

## Prerequisites

- Kanari SDK installed
- Public IP address or domain name
- Open ports in your firewall
- At least one wallet created with `kari keytool generate`

## Basic Network Setup

### Step 1: Configure Your Network

1. Open the following ports in your firewall:
   - RPC Port (default: 30031)
   - P2P Port (default: 30303)

2. Get your public IP address or domain name

### Step 2: Start a Bootstrap Node

Start your first node which will act as a bootstrap node:

```bash
kari start --port 30031 --public-ip YOUR.PUBLIC.IP.ADDRESS
```

This node will be accessible to other nodes on the network.

### Step 3: Connect Additional Nodes

From other machines, connect to the bootstrap node:

```bash
kari start --port 30031 --peer YOUR.BOOTSTRAP.IP.ADDRESS:30303
```

## Advanced Configuration

### Using Domain Names

If you have a domain name, you can use it instead of IP:

```bash
kari start --port 30031 --domain your.domain.name
```

### Multiple Peer Connections

Connect to multiple peers for better network resilience:

```bash
kari start --port 30031 --peer node1.example.com:30303 --peer node2.example.com:30303
```

## Network Security

### Securing RPC Endpoints

1. Configure CORS settings in your config.yaml:
```yaml
rpc:
  cors_domains:
    - https://your-domain.com
    - https://api.your-domain.com
```

2. Use SSL/TLS for RPC connections:
```bash
kari start --port 30031 --ssl-cert /path/to/cert.pem --ssl-key /path/to/key.pem
```

### Validator Node Setup

To run a validator node:

1. Stake required tokens (minimum 32 KARI):
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
}' https://your.node.address:30031
```

2. Start node in validator mode:
```bash
kari start --port 30031 --validator --domain your.domain.name
```

## Production Deployment Tips

1. Use a process manager (e.g., systemd, PM2)
2. Set up monitoring and alerts
3. Regular backups of wallet and configuration
4. Use SSL certificates for all endpoints
5. Configure proper firewalls and access controls

## Network Topology Example

```
                   ┌─────────────────┐
                   │  Bootstrap Node │
                   │  Public IP/DNS  │
                   └─────────┬───────┘
                            │
            ┌──────────────┼──────────────┐
            │              │              │
    ┌───────▼────┐  ┌─────▼─────┐  ┌────▼──────┐
    │ Validator 1 │  │ Validator 2│  │ Validator 3│
    │ Domain/IP 1 │  │ Domain/IP 2│  │ Domain/IP 3│
    └────────────┘  └───────────┘  └───────────┘
```

## Health Monitoring

Check node status:
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "get_node_status",
  "params": [],
  "id": 1
}' https://your.node.address:30031
```

## Troubleshooting

1. Connection Issues:
   - Verify ports are open using `netstat -an | grep 3003*`
   - Check firewall rules
   - Verify DNS resolution if using domains

2. Sync Issues:
   - Check network connectivity
   - Verify peer connections
   - Check system time synchronization

3. Performance Issues:
   - Monitor system resources
   - Check network bandwidth
   - Verify hardware requirements

## System Requirements

- CPU: 4+ cores
- RAM: 8GB minimum
- Storage: 100GB SSD
- Network: 100Mbps+ stable connection
- Public IP or domain name
- Open ports: 30031 (RPC), 30303 (P2P)

## Monitoring Tools

Recommended monitoring setup:
- Prometheus for metrics
- Grafana for visualization
- Node exporter for system metrics
- Network monitoring
- Log aggregation

## Backup and Recovery

1. Regular backups of:
   - Wallet files
   - Configuration files
   - Blockchain data

2. Recovery procedure:
   - Restore from backup
   - Resync with network
   - Verify node status
