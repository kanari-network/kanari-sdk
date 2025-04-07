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
   - RPC Port (default: 30030)
   - P2P Port (default: 51303)

2. Get your public IP address:
   - Visit https://whatismyip.com or
   - Run `curl ifconfig.me` in terminal
   - Example: 203.0.113.42

### Step 2: Configure Security (Optional)

Generate TLS certificates for secure communication:

```bash
# Generate self-signed certificates
kari certificate generate

# Check certificate status
kari certificate status
```

To enable TLS, edit your configuration file:

```yaml
use_tls: true
```

Note: The node will automatically disable TLS if certificates are missing.

### Step 3: Start a Bootstrap Node (First Node)

Start your first node which will act as a bootstrap node:

```bash
kari start --port 30030
```

The node will automatically detect your network interface and bind to the appropriate IP address.
You should see output similar to:

```
Configuration already exists. Skipping configuration process.
Using existing wallet as address: 0x28e442c54d872cea9415382e61559dde126da6d6aee8c70855bd6c8cbdeb40d8
Starting blockchain...
Running blockchain simulation...
Block status will be shown below. Press Enter to stop the node.
Starting RPC server on port 30030...
HTTP server running on http://192.168.1.103:30030
```

Note the IP address from the output (in this example, 192.168.1.103) - you'll need this to connect other nodes.

### Step 4: Set Up Additional Nodes

#### On a Different Machine

From another computer on the same network:

```bash
kari start --port 30032 --peer 192.168.1.103:51303
```

Replace `192.168.1.103` with the IP address of your first node. The port `51303` is the default P2P port.

#### Multiple Nodes on Same Machine

If running another node on the same machine:

```bash
# Create a different wallet first (optional but recommended)
kari keytool generate

# Start second node with different RPC port
kari start --port 30032 --peer 127.0.0.1:51303
```

You'll see connection messages in the console output when nodes connect successfully.

## Multi-Node Network Tips

### Connecting Multiple Peers

You can connect to multiple bootstrap nodes for redundancy:

```bash
kari start --port 30032 --peer NODE1_IP:51303 --peer NODE2_IP:51303
```

### Verifying Node Connections

To check if nodes are properly connected:

1. In the node logs, look for messages like:
   - "Connected to peer..." (on the second node)
   - "New peer connection from..." (on the first node)

2. Use the RPC API to check connected peers:
   ```bash
   curl -X POST -H "Content-Type: application/json" -d '{
     "jsonrpc": "2.0",
     "method": "blockchain_status",
     "params": [],
     "id": 1
   }' http://localhost:30030
   ```

### Port Configuration

- RPC Port: Used for API access (customizable with `--port` parameter)
- P2P Port: Used for node-to-node communication (default: 51303)

If running multiple nodes on one machine, each node must use:
- Different RPC ports
- Different P2P ports (automatically assigned if not specified)

## Advanced Configuration

### Using Domain Names

If you have a domain name pointing to your server:

```bash
# The domain will be used in configuration automatically
kari start --port 30030
```

For detailed instructions on setting up a domain name for your node:

1. See our [Domain Configuration Guide](domain_setup_guide.md) for complete instructions
2. Follow the specific examples for setting up "devnet.kanari.site"
3. Configure DNS records to point to your node's IP address
4. Set up HTTPS using the certificate management features:
   ```bash
   kari certificate generate
   ```

### Network Security

#### Certificate Management

Certificates are stored in your Kari data directory's `certs` folder:
- Linux/macOS: `~/.kari/certs/`
- Windows: `%USERPROFILE%\.kari\certs\`

The following files are used:
- `node.crt` - The node's certificate
- `node.key` - The node's private key

#### Securing RPC Endpoints

For production environments, we recommend using a reverse proxy (like Nginx or Caddy) to add TLS/SSL:

```nginx
# Example Nginx configuration
server {
    listen 443 ssl;
    server_name blockchain-api.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/yourdomain.com/privkey.pem;

    location / {
        proxy_pass http://localhost:30030;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Validator Node Setup

To run a validator node:

1. First, generate a wallet if you don't have one:
   ```bash
   kari keytool generate
   ```

2. Stake the required tokens (minimum 100,000 KARI):
   ```bash
   curl -X POST -H "Content-Type: application/json" -d '{
     "jsonrpc": "2.0",
     "method": "stake_tokens",
     "params": {
       "address": "YOUR_WALLET_ADDRESS",
       "amount": 100000.0,
       "password": "YOUR_WALLET_PASSWORD",
       "validator": true
     },
     "id": 1
   }' http://your.node.address:30030
   ```

3. The node will automatically run in validator mode once staking is confirmed.

## Network Topology Example

```
                         ┌───────────────────────┐
                         │    Bootstrap Node     │
                         │  192.168.1.103:30301  │
                         └──────────┬────────────┘
                                    │
           ┌────────────────────────┼────────────────────────┐
           │                        │                        │
┌──────────▼──────────┐  ┌──────────▼──────────┐  ┌──────────▼──────────┐
│       Node 2        │  │        Node 3       │  │       Node 4        │
│ 192.168.1.103:30302 │  │ 192.168.1.103:30305 │  │ 192.168.1.103:30304 │
└─────────────────────┘  └─────────────────────┘  └─────────────────────┘
```

## Troubleshooting

### Common Issues

1. **Can't connect to peers**:
   - Verify the P2P port (51303) is open on the first node
   - Check firewall settings on both machines
   - Make sure you're using the correct IP address
   - Try using `--peer 192.168.1.103:51303` format (IP:P2P_PORT)

2. **Node won't start**: 
   - Verify wallet exists (`kari keytool list`)
   - Check configuration file is properly formatted

3. **Certificate errors**:
   - Check certificate status with `kari certificate status`
   - If OpenSSL is not installed, install it first
   - Make sure certificate paths in configuration are correct

4. **RPC errors**: 
   - Ensure the RPC port is accessible and not blocked
   - Verify you're using the correct port in API calls

5. **Blockchain not syncing**: 
   - Verify connection to peers
   - Check logs for errors related to block propagation

### Checking Node Status

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "blockchain_status",
  "params": [],
  "id": 1
}' http://localhost:30030
```

### Viewing Connected Peers

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "get_peers",
  "params": [],
  "id": 1
}' http://localhost:30030
```

## System Requirements

- CPU: 4+ cores
- RAM: 8GB minimum
- Storage: 100GB SSD
- Network: 100Mbps+ stable connection
- Public IP or domain name
- Open ports: 30030 (RPC), 51303 (P2P)
