# Kanari Network Node Setup Guide for Windows

This guide explains how to set up and run Kanari blockchain nodes on Windows in a real network environment.

## Prerequisites

- Kanari SDK installed
- Public IP address
- Administrator privileges to configure Windows Firewall
- At least one wallet created with `kari keytool generate`

## Basic Network Setup on Windows

### Step 1: Configure Your Network

1. Open the following ports in Windows Firewall:
   - RPC Port (default: 30030)
   - P2P Port (default: 51303)

   To open these ports using Command Prompt (run as Administrator):
   ```cmd
   netsh advfirewall firewall add rule name="Kanari RPC Port" dir=in action=allow protocol=TCP localport=30030
   netsh advfirewall firewall add rule name="Kanari P2P Port" dir=in action=allow protocol=TCP localport=51303
   ```

2. Determine your public IP address:
   - Visit [whatismyip.com](https://whatismyip.com) or
   - Run `curl ifconfig.me` in PowerShell

### Step 2: Configure Security (Optional)

For secure node communication:

1. Set up a firewall to restrict access to required ports only
2. Consider using a VPN for secure communication between nodes

### Step 3: Start a Bootstrap Node

Start your first node which will act as a bootstrap node:

```cmd
kari start --port 30030
```

The node will automatically detect your network interface and bind to the appropriate IP address. Note the IP address from the output (for example, `192.168.1.103`)—you'll need this to connect additional nodes.

### Step 4: Set Up Additional Nodes

#### On a Different Machine

From another Windows computer on the same network, run:
```cmd
kari start --port 30032 --peer 192.168.1.103:51303
```
Replace `192.168.1.103` with the IP address of your bootstrap node.

#### Multiple Nodes on the Same Machine

To run an additional node on the same machine:
1. (Optional) Create a new wallet:
   ```cmd
   kari keytool generate
   ```
2. Start a second node with a different RPC port:
   ```cmd
   kari start --port 30032 --peer 127.0.0.1:51303
   ```

### Step 5: Verify Node Connections

- Monitor the node logs for messages indicating successful peer connections.
- Use the RPC API to check node status:
   ```cmd
   curl -X POST -H "Content-Type: application/json" -d "{
     \"jsonrpc\": \"2.0\",
     \"method\": \"blockchain_status\",
     \"params\": [],
     \"id\": 1
   }" http://localhost:30030
   ```

## Advanced Configuration and Troubleshooting

- **Firewall Issues**: Verify that Windows Firewall rules are configured correctly.
- **Port Conflicts**: If a port is already in use, specify a different one using the `--port` parameter.
- **RPC Security**: In production, consider using a reverse proxy (e.g., IIS or Nginx on Windows) with TLS/SSL enabled.

### Setting up HTTPS on Windows

For secure API access, set up a reverse proxy with HTTPS:

1. Install Nginx for Windows from the official site.

2. Create or update your Nginx configuration (e.g., `nginx.conf`) with HTTPS settings

3. Open port 443 in Windows Firewall:
   ```cmd
   netsh advfirewall firewall add rule name="Nginx HTTPS" dir=in action=allow protocol=TCP localport=443
   ```

Your Kanari RPC endpoint will now be accessible via HTTPS.

## System Requirements

- CPU: 4+ cores
- RAM: 8GB minimum
- Storage: 100GB (SSD or HDD)
- Network: Stable connection with a public IP address

For more details, also see the [General Kanari Network Node Setup Guide](network-setup.md).
