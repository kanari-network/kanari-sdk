# Kanari Network Node Setup Guide for Windows

This guide explains how to set up and run Kanari blockchain nodes on Windows in a real network environment.

## Prerequisites

- Kanari SDK installed
- Public IP address or domain name
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

### Step 2: Configure TLS Certificates (Optional)

For secure node-to-node communication:

1. Generate self-signed certificates:
   ```cmd
   kari certificate generate
   ```

2. Verify certificate status:
   ```cmd
   kari certificate status
   ```

3. If OpenSSL is not installed, download and install it from [Shining Light Productions](https://slproweb.com/products/Win32OpenSSL.html)

4. Enable TLS in your configuration by editing `%USERPROFILE%\.kari\config.yaml`:
   ```yaml
   use_tls: true
   ```

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

For example, to use Nginx with HTTPS:

1. Install Nginx for Windows from the official site.

2. Generate a certificate using the Kari CLI (preferred) or OpenSSL directly:
   ```cmd
   kari certificate generate
   ```
   
   Or using OpenSSL directly:
   ```cmd
   openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout nginx.key -out nginx.crt
   ```

3. Create or update your Nginx configuration (e.g., `nginx.conf`) with the following:
   ```nginx
   server {
       listen       443 ssl;
       server_name  your.domain.com;

       ssl_certificate      "C:/path/to/nginx.crt";
       ssl_certificate_key  "C:/path/to/nginx.key";

       location / {
           proxy_pass http://localhost:30030;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
           proxy_set_header X-Forwarded-Proto https;
       }
   }
   ```

4. Open port 443 in Windows Firewall:
   ```cmd
   netsh advfirewall firewall add rule name="Nginx HTTPS" dir=in action=allow protocol=TCP localport=443
   ```

Your Kanari RPC endpoint will now be accessible via HTTPS at your configured domain.

### Configuring Domain Names on Windows

If you're setting up a domain name (like "devnet.kanari.site") on Windows:

1. First, check if your domain resolves correctly:
   ```cmd
   nslookup devnet.kanari.site
   ```
   
   The response should show your server's IP address.

2. Edit the configuration file:
   ```cmd
   notepad %USERPROFILE%\.kari\config.yaml
   ```
   
   Add your domain to the configuration:
   ```yaml
   domain: "devnet.kanari.site"
   ```

3. To set up HTTPS with your domain on Windows:
   
   a) Using Let's Encrypt with Windows:
      - Install [Certify The Web](https://certifytheweb.com/) (GUI tool)
      - Create a new certificate for your domain
      - Select "Save to file" with private key
      - Save to your Kari certs directory

   b) Using self-signed certificates:
      ```cmd
      kari certificate generate
      ```

4. Configure IIS or Nginx for Windows as a reverse proxy:

   ```cmd
   REM Install Nginx for Windows if not already installed
   choco install nginx

   REM Edit nginx.conf
   notepad "C:\Program Files\nginx\conf\nginx.conf"
   ```

   Add this server block to the nginx.conf file:
   ```
   server {
       listen 443 ssl;
       server_name devnet.kanari.site;

       ssl_certificate "C:/Users/YourUser/.kari/certs/node.crt";
       ssl_certificate_key "C:/Users/YourUser/.kari/certs/node.key";

       location / {
           proxy_pass http://localhost:30030;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
       }
   }
   ```

5. Open Windows Firewall for HTTP(S) and P2P ports:
   ```cmd
   netsh advfirewall firewall add rule name="HTTPS for Kari" dir=in action=allow protocol=TCP localport=443
   ```

For detailed instructions, see the [Domain Configuration Guide](domain_setup_guide.md).

### Certificate Management

1. Certificates are stored in `%USERPROFILE%\.kari\certs\`
2. View certificate status with `kari certificate status`
3. If you need to regenerate certificates, delete the existing ones first

## System Requirements

- CPU: 4+ cores
- RAM: 8GB minimum
- Storage: 100GB (SSD or HDD)
- Network: Stable connection with a public IP address or domain name

For more details, also see the [General Kanari Network Node Setup Guide](network-setup.md).
