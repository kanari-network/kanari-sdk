# Kari Node Domain Configuration Guide

This guide explains how to configure a domain name (such as devnet.kanari.site) for your Kari blockchain node, allowing users to connect to your node using a memorable domain instead of an IP address.

## Table of Contents

1. [Domain Registration](#domain-registration)
2. [DNS Configuration](#dns-configuration)
3. [Setting up devnet.kanari.site](#setting-up-devnetkanarisite)
4. [Node Configuration](#node-configuration)
5. [HTTPS Configuration](#https-configuration)
6. [Testing Your Domain](#testing-your-domain)
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

For a Kari network, you might want to create different subdomains:
- `devnet.kanari.site` - Development network
- `testnet.kanari.site` - Test network
- `mainnet.kanari.site` - Main production network

## DNS Configuration

### Basic DNS Concepts

DNS (Domain Name System) records map your domain name to your server's IP address:

- **A Record**: Maps a domain directly to an IPv4 address
- **CNAME Record**: Maps a subdomain to another domain
- **AAAA Record**: Maps a domain to an IPv6 address
- **TXT Record**: Stores text information (often used for verification)

### Setting Up A Records

To point your domain to your Kari node server:

1. Log into your domain registrar's dashboard
2. Navigate to the DNS management section
3. Add an A record:
   - **Type**: A
   - **Host/Name**: Use @ for the root domain, or a subdomain name like "devnet"
   - **Value/Points to**: Your server's public IP address
   - **TTL**: 3600 (or as recommended by your provider)

Example A record configuration:

| Type | Host/Name | Value/Points to | TTL |
|------|-----------|----------------|-----|
| A    | devnet    | 203.0.113.42   | 3600|

### Setting Up CNAME Records

If you want to create multiple subdomains pointing to the same server:

1. Set up one A record for your main subdomain
2. Create CNAME records for additional subdomains pointing to your main subdomain

Example CNAME configuration:

| Type  | Host/Name | Value/Points to        | TTL |
|-------|-----------|------------------------|-----|
| A     | devnet    | 203.0.113.42          | 3600|
| CNAME | api       | devnet.kanari.site    | 3600|

This allows `api.kanari.site` to point to the same server as `devnet.kanari.site`.

## Setting up devnet.kanari.site

This section provides a specific example for setting up "devnet.kanari.site" for a Kari blockchain network.

### Step 1: Register or Use kanari.site Domain

To use "devnet.kanari.site":

1. If you're part of the official Kanari team:
   - Request access to the domain from the DevOps team
   - Provide your server's IP address for DNS configuration
   
2. If you're setting up your own domain:
   - Register your domain (e.g., "yourname.site") with any registrar
   - Create a "devnet" subdomain for development purposes

### Step 2: DNS Configuration for devnet.kanari.site

To configure DNS records for "devnet.kanari.site" (or your equivalent):

1. Access the DNS management through your registrar or DNS provider:
   - For Cloudflare: Go to DNS → Records
   - For AWS Route 53: Go to Hosted Zones → [your domain] → Create Record
   - For GoDaddy: Domain Details → DNS Management

2. Create an A record:
   - **Type**: A
   - **Name/Host**: devnet
   - **Value/Points to**: Your server's public IP address (e.g., 203.0.113.42)
   - **TTL**: 3600 (1 hour) or as recommended

3. If needed, also create these additional records:
   - An A record for "api.devnet.kanari.site" pointing to the same IP (or use a CNAME)
   - A TXT record for verification (if using Let's Encrypt for SSL)

### Step 3: Nameserver Configuration (For DNS Provider)

If using an external DNS provider like Cloudflare:

1. In your domain registrar account:
   - Find nameserver settings
   - Replace default nameservers with your DNS provider's nameservers
   
   For Cloudflare, typically:
   - ns1.cloudflare.com
   - ns2.cloudflare.com

2. Wait 24-48 hours for nameserver propagation

3. In Cloudflare:
   - Add the A record for "devnet" pointing to your server IP
   - Enable proxy protection (orange cloud) for DDoS protection
   - Consider enabling "Always Use HTTPS" to redirect HTTP to HTTPS

### Step 4: Testing DNS Configuration

After setting up DNS records:

1. Check DNS propagation:
   ```bash
   # Using dig tool
   dig devnet.kanari.site A
   
   # Using nslookup
   nslookup devnet.kanari.site
   ```

2. The response should show your server's IP address

3. Check from multiple locations using online tools:
   - https://www.whatsmydns.net/#A/devnet.kanari.site
   - https://dnschecker.org/#A/devnet.kanari.site

### Step 5: Configuring Your Node

Update your node's configuration with the domain name:

```yaml
# In ~/.kari/config.yaml
chain_id: "kari-testnet-001"
rpc_port: 30030
address: "0x7a1c8f19cAE0A90d4A4E445793eB0BED2FaA9ecF"
domain: "devnet.kanari.site"
use_tls: true
```

When other nodes connect to yours, they can use:
```bash
kari start --peer devnet.kanari.site:51303
```

### Step 6: Operating Multiple Environment Subdomains

For different network environments, consider standardizing on these subdomains:

1. **devnet.kanari.site** - Development testing network
   - For testing new features
   - Reset frequently
   
2. **testnet.kanari.site** - Stable test network
   - More stable than devnet
   - Less frequent resets
   
3. **mainnet.kanari.site** - Production network
   - Stable production environment
   - Never reset

Each subdomain should have its own server with appropriate scaling for its purpose.

## Node Configuration

### Configuring Your Kari Node to Use Your Domain

Add your domain name to the node configuration:

1. Edit your node configuration file:
   ```yaml
   # In ~/.kari/config.yaml
   chain_id: "kari-testnet-001"
   rpc_port: 30030
   address: "0x7a1c8f19cAE0A90d4A4E445793eB0BED2FaA9ecF"
   domain: "devnet.kanari.site"
   use_tls: true
   ```

2. Update the discovery nodes in your code:
   ```rust
   // In node configuration
   discovery_nodes: vec![
       "devnet.kanari.site:51303".to_string(),
       // Add other nodes as needed
   ],
   ```

### Port Forwarding for Your Domain

1. Configure your router/firewall to forward ports to your node server:
   - Forward port 30030 for RPC
   - Forward port 51303 for P2P communication

2. For cloud servers (like AWS EC2, Digital Ocean, etc.):
   - Configure security groups/firewall rules to allow:
     - Inbound TCP traffic on port 30030 (RPC)
     - Inbound TCP traffic on port 51303 (P2P)

## HTTPS Configuration

### Step 1: Generate TLS Certificate

Generate a certificate for your domain:

```bash
kari certificate generate
```

Or use Let's Encrypt to get a free, trusted certificate:

```bash
sudo apt-get install certbot
sudo certbot certonly --standalone -d devnet.kanari.site
```

### Step 2: Set Up a Reverse Proxy

For HTTPS support, set up Nginx as a reverse proxy:

```nginx
server {
    listen 443 ssl;
    server_name devnet.kanari.site;

    ssl_certificate /etc/letsencrypt/live/devnet.kanari.site/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/devnet.kanari.site/privkey.pem;

    location / {
        proxy_pass http://localhost:30030;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}

server {
    listen 80;
    server_name devnet.kanari.site;
    return 301 https://$host$request_uri;
}
```

### Step 3: HTTP to HTTPS Redirection

The Nginx configuration above includes automatic HTTP to HTTPS redirection, ensuring all traffic uses secure connections.

## Testing Your Domain

### Testing RPC Endpoint

Use curl to test your RPC endpoint:

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "blockchain_status",
  "params": [],
  "id": 1
}' https://devnet.kanari.site
```

### Testing P2P Connection

From another node, connect as a peer:

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
