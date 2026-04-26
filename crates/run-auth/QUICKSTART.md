# Quick Start Guide - Kanari Auth API

## Prerequisites

- Rust toolchain (latest stable)
- curl or Postman for testing
- jq (optional, for JSON formatting)

## Step 1: Start the Server

```bash
# Navigate to project root
cd c:\Users\Pukpuy\Desktop\kanari-sdk

# Start the API server (default port 3000)
cargo run -p run-auth

# Or with custom configuration
$env:AUTH_DB_PATH="data/auth.db"
$env:AUTH_API_PORT=8080
cargo run -p run-auth
```

The server will start and display:

```
INFO Starting Kanari Auth API server...
INFO Kanari Auth API listening on 0.0.0.0:3000
```

## Step 2: Test Health Check

Open a new terminal and test the health endpoint:

```bash
curl http://localhost:3000/health
```

Expected response:

```json
{
  "status": "healthy",
  "service": "kanari-auth-api",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

## Step 3: Register a User

```bash
curl -X POST http://localhost:3000/api/v1/register \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"test@example.com\",\"password\":\"Test1234!\"}"
```

Expected response (201 Created):

```json
{
  "success": true,
  "data": {
    "success": true,
    "wallet_address": "0x1234...",
    "message": "User registered successfully"
  },
  "error": null
}
```

## Step 4: Login

```bash
curl -X POST http://localhost:3000/api/v1/login \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"test@example.com\",\"password\":\"Test1234!\"}"
```

Expected response (200 OK):

```json
{
  "success": true,
  "data": {
    "success": true,
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "user_email": "test@example.com",
    "wallet_address": "0x1234...",
    "expires_at": "2024-01-16T10:30:00Z"
  },
  "error": null
}
```

**Save the `session_id` for subsequent requests!**

## Step 5: Validate Session

```bash
curl http://localhost:3000/api/v1/session/validate/YOUR_SESSION_ID
```

Expected response:

```json
{
  "success": true,
  "data": {
    "valid": true,
    "session_id": "550e8400-e29b-41d4-a716-446655440000"
  },
  "error": null
}
```

## Step 6: List Users

```bash
curl http://localhost:3000/api/v1/users
```

Expected response:

```json
{
  "success": true,
  "data": {
    "users": ["test@example.com"],
    "count": 1
  },
  "error": null
}
```

## Step 7: Sign a Transaction

```bash
curl -X POST http://localhost:3000/api/v1/sign/transfer \
  -H "Content-Type: application/json" \
  -d "{\"session_id\":\"YOUR_SESSION_ID\",\"recipient\":\"0xRecipient...\",\"amount\":1000000,\"gas_limit\":100000,\"gas_price\":1000}"
```

Expected response:

```json
{
  "success": true,
  "data": {
    "signed_transaction": "{...}"
  },
  "error": null
}
```

## Step 8: Logout

```bash
curl -X POST http://localhost:3000/api/v1/logout \
  -H "Content-Type: application/json" \
  -d "{\"session_id\":\"YOUR_SESSION_ID\"}"
```

Expected response:

```json
{
  "success": true,
  "data": {
    "message": "Logged out successfully"
  },
  "error": null
}
```

## Testing with PowerShell (Windows)

If you're on Windows using PowerShell, here are the equivalent commands:

```powershell
# Health Check
Invoke-RestMethod -Uri "http://localhost:3000/health" -Method Get

# Register
$body = @{
    email = "test@example.com"
    password = "Test1234!"
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://localhost:3000/api/v1/register" `
    -Method Post `
    -ContentType "application/json" `
    -Body $body

# Login
$loginBody = @{
    email = "test@example.com"
    password = "Test1234!"
} | ConvertTo-Json

$response = Invoke-RestMethod -Uri "http://localhost:3000/api/v1/login" `
    -Method Post `
    -ContentType "application/json" `
    -Body $loginBody

$sessionId = $response.data.session_id
Write-Host "Session ID: $sessionId"

# Validate Session
Invoke-RestMethod -Uri "http://localhost:3000/api/v1/session/validate/$sessionId" -Method Get
```

## Using the Example Script

A bash script is provided in `examples/api_usage.sh`:

```bash
chmod +x examples/api_usage.sh
./examples/api_usage.sh
```

This script demonstrates all API endpoints in sequence.

## Troubleshooting

### Server won't start

1. Check if port 3000 is already in use:

   ```bash
   netstat -ano | findstr :3000
   ```

2. Change the port:

   ```bash
   $env:AUTH_API_PORT=8080
   cargo run -p run-auth
   ```

### Database errors

1. Ensure the data directory exists:

   ```bash
   mkdir data
   ```

2. Check file permissions on the database file

### Invalid credentials

Make sure your password meets the requirements:

- Minimum 8 characters
- At least one uppercase letter
- At least one lowercase letter
- At least one digit
- At least one special character (!@#$%^&*)

Example valid passwords: `SecurePass123!`, `Test1234!`, `MyP@ssw0rd`

## Next Steps

- Read the full [API Documentation](README.md)
- Explore the [kanari-auth library documentation](../kanari-auth/README.md)
- Check out the [Kanari SDK main README](../../README.md)

## Support

For issues or questions:

- Check the logs for detailed error messages
- Review the API documentation
- Open an issue on GitHub
