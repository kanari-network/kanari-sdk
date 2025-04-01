# Kanari SDK RPC API Guide

Here are all the available API endpoints and how to call them using curl in Git Bash:

## 1. Blockchain Status
Get current blockchain status:
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "blockchain_status",
  "params": [],
  "id": 1
}' http://127.0.0.1:30031
```

## 1.1 Blockchain Status
Get all blocks:
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "get_all_blocks",
  "params": [],
  "id": 1
}' http://127.0.0.1:30031
```

## 2. Get Account
Check account details:
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", 
  "method": "get_account_details",
  "params": {
    "address": "YOUR_ACCOUNT_ADDRESS"
  },
  "id": 1
}' http://127.0.0.1:30031
```

## 3. List Accounts
Get all accounts and their balances:
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "list_accounts",
  "params": [],
  "id": 1
}' http://127.0.0.1:30031
```

## 4. Transfer Tokens
Send KARI tokens:
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "transfer",
  "params": {
    "from": "SENDER_ADDRESS",
    "to": "RECEIVER_ADDRESS",
    "amount": 1.0,
    "password": "WALLET_PASSWORD"
  },
  "id": 1
}' http://127.0.0.1:30031
```

Response includes a unique transaction ID (0x followed by 64 random hex characters) that can be used to track the transaction.

## 5. Get Wallets
List all available wallets:
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "get_wallets",
  "params": [],
  "id": 1
}' http://127.0.0.1:30031
```

## 6. Search Transactions
Search for transactions by address with pagination:
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "search_transactions",
  "params": {
    "address": "YOUR_ACCOUNT_ADDRESS",
    "limit": 10,
    "offset": 0
  },
  "id": 1
}' http://127.0.0.1:30031
```

## 7. Get Transaction by ID
Look up a transaction using its unique ID:
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "get_transaction_by_id",
  "params": "0xTRANSACTION_ID",
  "id": 1
}' http://127.0.0.1:30031
```

The response includes transaction details, the containing block, and current balances of both sender and receiver.

## 8. Upload File
Upload a file to storage:
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "upload_file",
  "params": {
    "filename": "example.txt",
    "data": "BASE64_ENCODED_FILE_CONTENT"
  },
  "id": 1
}' http://127.0.0.1:30031
```

## 9. Get File
Retrieve a file by ID:
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "get_file",
  "params": "FILE_ID",
  "id": 1
}' http://127.0.0.1:30031
```

## Placeholders
Replace these placeholders with actual values when making requests:
- `YOUR_ACCOUNT_ADDRESS`: Your wallet address
- `SENDER_ADDRESS`: Address sending tokens
- `RECEIVER_ADDRESS`: Address receiving tokens
- `WALLET_PASSWORD`: Your wallet password
- `0xTRANSACTION_ID`: Transaction ID (0x followed by 64 hex characters)
- `BASE64_ENCODED_FILE_CONTENT`: Base64 encoded file data
- `FILE_ID`: Unique file identifier

## Response Format
Successful response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    // Response data specific to each endpoint
  },
  "id": 1
}
```

Error response:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Error description"
  },
  "id": 1
}
```

## Transaction Structure
Transaction objects contain the following fields:
- `id`: Unique transaction identifier (0x + 64 hex characters)
- `sender`: Address of the sender
- `receiver`: Address of the receiver
- `amount`: Amount of tokens transferred (in KA units)
- `amount_formatted`: Human-readable amount in KARI
- `timestamp`: UNIX timestamp when the transaction was created
