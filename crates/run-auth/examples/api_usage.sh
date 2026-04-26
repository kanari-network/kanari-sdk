#!/bin/bash
# Kanari Auth API - Example Usage Script
# This script demonstrates how to interact with the run-auth API server

API_BASE="http://localhost:3000/api/v1"

echo "=== Kanari Auth API Examples ==="
echo ""

# 1. Health Check
echo "1. Health Check"
curl -s $API_BASE/../health | jq .
echo ""

# 2. Register a new user
echo "2. Register User"
REGISTER_RESPONSE=$(curl -s -X POST "$API_BASE/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice@example.com",
    "password": "SecurePass123!",
    "curve_type": "ed25519"
  }')
echo $REGISTER_RESPONSE | jq .
echo ""

# Extract wallet address from response
WALLET_ADDRESS=$(echo $REGISTER_RESPONSE | jq -r '.data.wallet_address')
echo "Wallet Address: $WALLET_ADDRESS"
echo ""

# 3. Login
echo "3. Login"
LOGIN_RESPONSE=$(curl -s -X POST "$API_BASE/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice@example.com",
    "password": "SecurePass123!",
    "session_timeout_hours": 24
  }')
echo $LOGIN_RESPONSE | jq .
echo ""

# Extract session ID
SESSION_ID=$(echo $LOGIN_RESPONSE | jq -r '.data.session_id')
echo "Session ID: $SESSION_ID"
echo ""

# 4. Validate Session
echo "4. Validate Session"
curl -s "$API_BASE/session/validate/$SESSION_ID" | jq .
echo ""

# 5. List Users
echo "5. List Users"
curl -s "$API_BASE/users" | jq .
echo ""

# 6. Get User Count
echo "6. User Count"
curl -s "$API_BASE/users/count" | jq .
echo ""

# 7. Sign a Transfer Transaction (example)
echo "7. Sign Transfer Transaction"
curl -s -X POST "$API_BASE/sign/transfer" \
  -H "Content-Type: application/json" \
  -d "{
    \"session_id\": \"$SESSION_ID\",
    \"recipient\": \"0xRecipientAddressHere\",
    \"amount\": 1000000,
    \"gas_limit\": 100000,
    \"gas_price\": 1000
  }" | jq .
echo ""

# 8. Change Password
echo "8. Change Password"
curl -s -X POST "$API_BASE/change-password" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice@example.com",
    "old_password": "SecurePass123!",
    "new_password": "NewSecurePass456!"
  }' | jq .
echo ""

# 9. Logout
echo "9. Logout"
curl -s -X POST "$API_BASE/logout" \
  -H "Content-Type: application/json" \
  -d "{
    \"session_id\": \"$SESSION_ID\"
  }" | jq .
echo ""

# 10. Delete Account (cleanup)
echo "10. Delete Account"
curl -s -X POST "$API_BASE/delete-account" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice@example.com",
    "password": "NewSecurePass456!"
  }' | jq .
echo ""

echo "=== All examples completed ==="
