# Kari Node Domain Configuration Guide

This guide explains how to configure domain names for your Kari blockchain node, allowing users to connect to your node using memorable domains instead of an IP address.

## Table of Contents

1. [Domain Registration](#domain-registration)
2. [DNS Configuration](#dns-configuration)
3. [Setting up Kanari Network Domains](#setting-up-kanari-network-domains)
4. [Node Configuration](#node-configuration)
5. [HTTPS Configuration](#https-configuration)
6. [Testing Your Domains](#testing-your-domains)
7. [Troubleshooting](#troubleshooting)

## Domain Registration

### Step 1: Choose a Domain Registrar

Select a domain registrar service to purchase your domain. Popular options include:
- [Namecheap](https://www.namecheap.com/)
- [Google Domains](https://domains.google/)
- [GoDaddy](https://www.godaddy.com/)
- [Cloudflare](https://www.cloudflare.com/products/registrar/)

### Step 2: Register Your Domain

1. Search for your desired domain name (e.g., "kanari.site")
2. Complete the registration process
3. Ensure you have access to the domain's DNS settings

### Step 3: Plan Your Subdomains

For a Kari network, you need two types of subdomains for each environment:

**P2P Network Domains** (for node-to-node communication):
- `devnet.kanari.site` - Development network
- `testnet.kanari.site` - Test network
- `mainnet.kanari.site` - Main production network

**RPC API Domains** (for client-to-node API access):
- `api.devnet.kanari.site` - Development network API
- `api.testnet.kanari.site` - Test network API
- `api.mainnet.kanari.site` - Main production network API

## DNS Configuration

### Basic DNS Concepts

DNS (Domain Name System) records map your domain name to your server's IP address:

- **A Record**: Maps a domain directly to an IPv4 address
- **CNAME Record**: Maps a subdomain to another domain
- **AAAA Record**: Maps a domain to an IPv6 address
- **TXT Record**: Stores text information (often used for verification)

### Setting Up A Records for Both Domain Types

To point your domains to your Kari node server:

1. Log into your domain registrar's dashboard
2. Navigate to the DNS management section
3. Add A records for both P2P and API domains:
   
   **For P2P Domain:**
   - **Type**: A
   - **Host/Name**: Use a subdomain name like "devnet"
   - **Value/Points to**: Your server's public IP address
   - **TTL**: 3600 (or as recommended by your provider)
   
   **For RPC API Domain:**
   - **Type**: A
   - **Host/Name**: Use a subdomain like "api.devnet"
   - **Value/Points to**: Your server's public IP address (same as P2P)
   - **TTL**: 3600 (or as recommended by your provider)

Example A record configuration:

| Type | Host/Name | Value/Points to | TTL |
|------|-----------|----------------|-----|
| A    | devnet    | 203.0.113.42   | 3600|
| A    | api.devnet| 203.0.113.42   | 3600|

### Alternative: Using CNAME Records

If you prefer, you can create the P2P domain as an A record, then use a CNAME for the API domain:

| Type  | Host/Name | Value/Points to        | TTL |
|-------|-----------|------------------------|-----|
| A     | devnet    | 203.0.113.42          | 3600|
| CNAME | api.devnet| devnet.kanari.site    | 3600|

This allows `api.devnet.kanari.site` to point to the same server as `devnet.kanari.site`.

## Setting up Kanari Network Domains

This section provides specific examples for setting up the official Kanari network domains.

### Step 1: Register or Use kanari.site Domain

To use the official Kanari domains:

1. If you're part of the official Kanari team:
   - Request access to the domain from the DevOps team
   - Provide your server's IP address for DNS configuration
   
2. If you're setting up your own domain:
   - Register your domain (e.g., "yourname.site") with any registrar
   - Create similar subdomains for development purposes

### Step 2: DNS Configuration for Kanari Domains

To configure DNS records for the official domains (or your equivalents):

1. Access the DNS management through your registrar or DNS provider
2. Create A records for both P2P and API domains:

   **For P2P domains:**
   - Create A record for "devnet.kanari.site" pointing to your server IP
   - Create A record for "testnet.kanari.site" pointing to your server IP
   - Create A record for "mainnet.kanari.site" pointing to your server IP
   
   **For API domains:**
   - Create A record for "api.devnet.kanari.site" pointing to your server IP
   - Create A record for "api.testnet.kanari.site" pointing to your server IP
   - Create A record for "api.mainnet.kanari.site" pointing to your server IP

3. If needed, also create TXT records for verification (if using Let's Encrypt for SSL)

### Step 3: Testing DNS Configuration

After setting up DNS records:

1. Check DNS propagation for both P2P and API domains:
   ```bash
   # Using dig tool
   dig devnet.kanari.site A
   dig api.devnet.kanari.site A
   
   # Using nslookup
   nslookup devnet.kanari.site
   nslookup api.devnet.kanari.site
   ```

2. The responses should show your server's IP address

### Step 4: Configuring Your Node

Update your node's configuration with both domain types:

```yaml
# In ~/.kari/config.yaml
chain_id: "kari-testnet-001"
rpc_port: 30030
address: "0x7a1c8f19cAE0A90d4A4E445793eB0BED2FaA9ecF"
domain_peer: "devnet.kanari.site"      # For P2P node connections
domain: "api.devnet.kanari.site"       # For RPC API access
use_tls: true
```

When other nodes connect to yours via P2P, they will use:
```bash
kari start --peer devnet.kanari.site:51303
```

For RPC API access, clients will use:
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "blockchain_status",
  "params": [],
  "id": 1
}' https://api.devnet.kanari.site
```

### Step 5: Operating Multiple Environment Domains

For different network environments, standardize on these domain patterns:

1. **Development Network**
   - P2P: `devnet.kanari.site` - For node-to-node communication
   - API: `api.devnet.kanari.site` - For RPC API access
   
2. **Test Network**
   - P2P: `testnet.kanari.site` - For node-to-node communication
   - API: `api.testnet.kanari.site` - For RPC API access
   
3. **Production Network**
   - P2P: `mainnet.kanari.site` - For node-to-node communication
   - API: `api.mainnet.kanari.site` - For RPC API access

## Node Configuration

### Configuring Your Kari Node to Use Your Domains

Add both domain types to the node configuration:

1. Edit your node configuration file:
   ```yaml
   # In ~/.kari/config.yaml
   chain_id: "kari-testnet-001"
   rpc_port: 30030
   address: "0x7a1c8f19cAE0A90d4A4E445793eB0BED2FaA9ecF"
   domain_peer: "devnet.kanari.site"      # For P2P connections
   domain: "api.devnet.kanari.site"       # For RPC API access
   use_tls: true
   ```

2. Update the discovery nodes in your code to use P2P domains:
   ```rust
   // In node configuration
   discovery_nodes: vec![
       "devnet.kanari.site:51303".to_string(),
       "testnet.kanari.site:51303".to_string(),
       "mainnet.kanari.site:51303".to_string(),
       // Add other nodes as needed
   ],
   ```

### Port Forwarding for Your Domains

1. Configure your router/firewall to forward ports to your node server:
   - Forward port 30030 for RPC API access
   - Forward port 51303 for P2P communication

2. For cloud servers (like AWS EC2, Digital Ocean, etc.):
   - Configure security groups/firewall rules to allow:
     - Inbound TCP traffic on port 30030 (RPC)
     - Inbound TCP traffic on port 51303 (P2P)

## HTTPS Configuration

### Step 1: Generate TLS Certificates

For HTTPS to work properly with your domains, you need valid certificates for each domain:

```bash
# Option 1: Using Let's Encrypt (recommended for production)
sudo apt-get install certbot
sudo certbot certonly --standalone -d devnet.kanari.site
sudo certbot certonly --standalone -d api.devnet.kanari.site

# Option 2: Using Kari's built-in certificate generator (for testing only)
kari certificate generate
```

### Step 2: Set Up a Reverse Proxy

Set up a reverse proxy for each domain (the P2P domain typically doesn't need HTTPS, but the API domain does):

```nginx
# /etc/nginx/sites-available/api.devnet.kanari.site.conf
server {
    listen 443 ssl;
    server_name api.devnet.kanari.site;

    # For Let's Encrypt certificates
    ssl_certificate /etc/letsencrypt/live/api.devnet.kanari.site/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.devnet.kanari.site/privkey.pem;
    
    # Important security settings
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_prefer_server_ciphers on;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256;

    location / {
        proxy_pass http://localhost:30030;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}

server {
    # Redirect HTTP to HTTPS
    listen 80;
    server_name api.devnet.kanari.site;
    return 301 https://$host$request_uri;
}
```

### Step 3: Activate and Test the Proxy

```bash
# For Nginx
sudo ln -s /etc/nginx/sites-available/api.devnet.kanari.site.conf /etc/nginx/sites-enabled/
sudo nginx -t  # Test configuration
sudo systemctl restart nginx
```

### Step 4: Firewall Configuration

Ensure your firewall allows the following ports:

```bash
# For Ubuntu/Debian
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw allow 30030/tcp  # Only needed if accessing RPC directly
sudo ufw allow 51303/tcp  # For P2P node connections

# For Windows
netsh advfirewall firewall add rule name="HTTPS" dir=in action=allow protocol=TCP localport=443
netsh advfirewall firewall add rule name="HTTP" dir=in action=allow protocol=TCP localport=80
netsh advfirewall firewall add rule name="Kari P2P" dir=in action=allow protocol=TCP localport=51303
```

### Step 5: Testing Your HTTPS Connection

To test if your HTTPS setup is working properly:

```bash
# Test domain DNS resolution
nslookup devnet.kanari.site
nslookup api.devnet.kanari.site

# Test HTTPS connection
curl -v https://api.devnet.kanari.site/health

# Test RPC API over HTTPS
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "blockchain_status",
  "params": [],
  "id": 1
}' https://api.devnet.kanari.site
```

### Troubleshooting HTTPS Issues

If you're seeing errors like:
- Certificate mismatch
- Connection refused
- Invalid SSL certificate

Check the following:
1. Verify your DNS records point to the correct IP address.
2. Ensure your certificates are valid and match your domain name.
3. Confirm your reverse proxy is running and properly configured.
4. Test your server's ports using online tools or `netstat`.

## Testing Your Domains

### Testing RPC API Endpoint

Use curl to test your RPC API endpoint:

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "blockchain_status",
  "params": [],
  "id": 1
}' https://api.devnet.kanari.site
```

### Testing P2P Connection

From another node, connect as a peer using the P2P domain:

```bash
kari start --peer devnet.kanari.site:51303
```

## Troubleshooting

### Domain Not Resolving

1. Verify DNS records are correctly set up
2. Check if DNS propagation is complete (can take up to 48 hours)
3. Use `nslookup devnet.kanari.site` to check if the domain resolves to the correct IP

### Cannot Connect to RPC

1. Ensure your firewall allows incoming connections on port 30030
2. Check that your Kari node is running properly
3. Verify that port forwarding is correctly configured on your router
4. Test direct connection to the IP:Port to rule out DNS issues

### Certificate Issues

1. Ensure your certificates are valid and not expired
2. Check correct paths to certificate files in your configuration
3. Verify that certificates match your domain name
4. Run `kari certificate status` to verify certificate setup

### P2P Connection Failures

1. Verify port 51303 is open on your firewall/router
2. Check that your Kari node is accepting P2P connections
3. Run a port check service to confirm port 51303 is publicly accessible
