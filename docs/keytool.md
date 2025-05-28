# Keytool CLI Manual

The Keytool CLI is a comprehensive command-line interface for managing cryptocurrency wallets, keys, and blockchain operations in the Kanari SDK.

## Table of Contents

- [Installation](#installation)
- [Basic Usage](#basic-usage)
- [Commands Overview](#commands-overview)
- [Wallet Management](#wallet-management)
- [Mnemonic Management](#mnemonic-management)
- [Session Key Management](#session-key-management)
- [Transaction Operations](#transaction-operations)
- [Security Best Practices](#security-best-practices)
- [Troubleshooting](#troubleshooting)

## Installation

The keytool is included with the Kanari SDK. Ensure you have the `kari` binary in your PATH.

```bash
# Verify installation
kari --version
```

## Basic Usage

```bash
kari keytool <command> [options]
```

To see all available commands:

```bash
kari keytool
```

## Commands Overview

| Command | Description |
|---------|-------------|
| `generate` | Generate a new wallet address |
| `balance` | Check balance of an address |
| `transfer` | Transfer coins to another address |
| `select` | Select an active wallet |
| `wallet` | Load and display wallet information |
| `list` | List all available wallets |
| `import` | Import wallet from seed phrase |
| `privatekey` | Import wallet from private key |
| `mnemonic` | Manage BIP39 mnemonic phrases |
| `session` | Manage temporary session keys |

## Wallet Management

### Generate New Wallet

Create a new wallet with a randomly generated seed phrase:

```bash
kari keytool generate
```

**Interactive prompts:**
1. Choose mnemonic length (12 or 24 words)
2. Select curve type:
   - `1` - K-256 (secp256k1) - Bitcoin-compatible
   - `2` - P-256 (secp256r1) - NIST standard
   - `3` - Ed25519 - Modern, fast curve
3. Set wallet password (with confirmation)

**Example output:**
```
Enter mnemonic length (12 or 24):
12
Select curve type:
1. K-256 (secp256k1)
2. P-256 (secp256r1)
3. Ed25519
1
New address generated:
Private Key: kanari1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0
Public Address: 0x1234567890abcdef1234567890abcdef12345678
Seed Phrase: word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12
Curve Type: K256
Wallet saved successfully!
```

### Import from Seed Phrase

Import an existing wallet using a BIP39 mnemonic phrase:

```bash
kari keytool import
```

**Steps:**
1. Enter your seed phrase (12 or 24 words)
2. Select curve type
3. Set wallet password

### Import from Private Key

Import a wallet using a private key:

```bash
kari keytool privatekey
```

**Steps:**
1. Enter your private key
2. Select curve type
3. Set wallet password

### List Wallets

Display all available wallets:

```bash
kari keytool list
```

**Example output:**
```
Available Wallets:
──────────────────────────────────────────────────────────────────────
➤ 0x1234567890abcdef1234567890abcdef12345678 [ACTIVE]
  0x9876543210fedcba9876543210fedcba98765432
  0xabcdef1234567890abcdef1234567890abcdef12
──────────────────────────────────────────────────────────────────────
Total wallets: 3
```

### Select Active Wallet

Choose which wallet to use for transactions:

```bash
kari keytool select
```

**Interactive selection:**
```
Available wallets:
────────────────────────────────────────────────────────────────
No.   Address                                    Status
────────────────────────────────────────────────────────────────
1     0x1234567890abcdef1234567890abcdef12345678  ACTIVE
2     0x9876543210fedcba9876543210fedcba98765432
3     0xabcdef1234567890abcdef1234567890abcdef12
────────────────────────────────────────────────────────────────

Enter wallet number to select (or press Enter to cancel):
2
✓ Wallet selected: 0x9876543210fedcba9876543210fedcba98765432
```

### Load Wallet Information

Display detailed information about a specific wallet:

```bash
kari keytool wallet
```

**Example output:**
```
Wallet Information:
────────────────────────────────────────────────────────────────
Address:        0x1234567890abcdef1234567890abcdef12345678
Curve Type:     K256
Private Key:    kanari1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0
Seed Phrase:    word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12
────────────────────────────────────────────────────────────────

✓ Wallet loaded successfully
```

## Mnemonic Management

The keytool supports BIP39 mnemonic phrase management for enhanced security and backup.

### Save Mnemonic

Store a mnemonic phrase securely in the keystore:

```bash
kari keytool mnemonic save
```

**Steps:**
1. Enter your BIP39 mnemonic phrase (12 or 24 words)
2. Enter associated wallet addresses (optional)
3. Set encryption password

### Load Mnemonic

Retrieve and display stored mnemonic:

```bash
kari keytool mnemonic load
```

**Example output:**
```
Mnemonic Information:
────────────────────────────────────────────────────────────────
Mnemonic:       word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12
Addresses:      
                0x1234567890abcdef1234567890abcdef12345678
                0x9876543210fedcba9876543210fedcba98765432
────────────────────────────────────────────────────────────────
```

### Check Mnemonic Status

Check if a mnemonic exists in the keystore:

```bash
kari keytool mnemonic status
```

### Remove Mnemonic

Permanently delete stored mnemonic:

```bash
kari keytool mnemonic remove
```

**Safety confirmation required:**
```
⚠️  WARNING: This will permanently delete your mnemonic phrase!
Type 'CONFIRM' to proceed:
CONFIRM
✓ Mnemonic removed successfully
```

## Session Key Management

Manage temporary session data like authentication tokens.

### Set Session Key

Store a temporary key-value pair:

```bash
kari keytool session set <key> <value>
```

**Example:**
```bash
kari keytool session set auth_token abc123xyz
✓ Session key 'auth_token' saved
```

### Get Session Key

Retrieve a session key value:

```bash
kari keytool session get <key>
```

**Example:**
```bash
kari keytool session get auth_token
auth_token: abc123xyz
```

### Remove Session Key

Delete a specific session key:

```bash
kari keytool session remove <key>
```

### Clear All Session Keys

Remove all stored session keys:

```bash
kari keytool session clear
```

**Confirmation required:**
```
⚠️  This will remove ALL session keys. Continue? (y/n)
y
✓ All session keys cleared
```

## Transaction Operations

### Check Balance

Check the balance of any address:

```bash
kari keytool balance
```

**Example:**
```
Enter public address:
0x1234567890abcdef1234567890abcdef12345678
Balance for 0x1234567890abcdef1234567890abcdef12345678: 1,250.500000000 Kari
```

### Transfer Funds

Send KARI tokens to another address:

```bash
kari keytool transfer
```

**Interactive process:**
1. Select or confirm sender wallet
2. View current balance
3. Enter recipient address
4. Enter amount to send
5. Confirm transaction details
6. Enter wallet password
7. Transaction submitted

**Example flow:**
```
Your balance: 1,250.500000000 KARI
Enter recipient address:
0x9876543210fedcba9876543210fedcba98765432
Enter amount to send (in KARI):
100.5

Transaction details:
  From: 0x1234567890abcdef1234567890abcdef12345678
  To:   0x9876543210fedcba9876543210fedcba98765432
  Amount: 100.5 KARI

Confirm transfer? (y/n)
y
Enter wallet password:
Sending transaction...
Transfer initiated successfully!
Transaction will be included in the next block.
Transaction ID: 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890
```

## Security Best Practices

### Password Security

1. **Use strong passwords** (minimum 12 characters)
2. **Include mixed case, numbers, and symbols**
3. **Never reuse passwords** across wallets
4. **Store passwords securely** (use password managers)

### Private Key Safety

1. **Never share private keys** with anyone
2. **Never store private keys** in plain text
3. **Keep backups** in secure, offline locations
4. **Use hardware wallets** for large amounts

### Mnemonic Phrase Security

1. **Write down seed phrases** on paper, never digital
2. **Store in multiple secure locations**
3. **Never take photos** of seed phrases
4. **Test recovery** before storing large amounts

### General Security

1. **Verify addresses** before sending transactions
2. **Start with small amounts** when testing
3. **Keep software updated**
4. **Use secure networks** (avoid public WiFi)

## Troubleshooting

### Common Issues

#### "No wallets found!"
- **Solution**: Generate or import a wallet first
- **Command**: `kari keytool generate`

#### "Invalid password"
- **Solution**: Ensure correct password for the wallet
- **Tip**: Passwords are case-sensitive

#### "Failed to load blockchain"
- **Solution**: Ensure the Kanari node is running
- **Check**: Node should be accessible at `http://127.0.0.1:30031`

#### "Transfer failed"
- **Possible causes**:
  - Insufficient balance
  - Invalid recipient address
  - Network connectivity issues
  - Incorrect password

#### "HTTP request failed"
- **Solution**: Install `curl` on your system
- **Windows**: Install via chocolatey or download
- **Linux/Mac**: Usually pre-installed

### Error Codes

| Error | Description | Solution |
|-------|-------------|----------|
| `WalletError::NotFound` | Wallet file doesn't exist | Check wallet address |
| `WalletError::InvalidPassword` | Wrong password | Verify password |
| `WalletError::EncryptionError` | Encryption/decryption failed | Check password and file integrity |
| `KeystoreError::IoError` | File system error | Check permissions and disk space |

### Getting Help

1. **Check this manual** for command usage
2. **Use help command**: `kari keytool`
3. **Check logs** for detailed error messages
4. **Verify environment** (node running, network connectivity)

### Backup and Recovery

#### Create Backup
1. **Export private keys** from important wallets
2. **Save mnemonic phrases** securely
3. **Backup keystore file** (located in `~/.kari/kanari_config/`)

#### Recovery Process
1. **Restore from mnemonic**: `kari keytool import`
2. **Import private key**: `kari keytool privatekey`
3. **Copy keystore file** to config directory

## Advanced Usage

### Batch Operations

For multiple operations, consider scripting:

```bash
#!/bin/bash
# Example script for checking multiple balances

addresses=(
    "0x1234567890abcdef1234567890abcdef12345678"
    "0x9876543210fedcba9876543210fedcba98765432"
)

for addr in "${addresses[@]}"; do
    echo "Checking balance for $addr"
    echo "$addr" | kari keytool balance
done
```

### Configuration

The keytool uses configuration files in `~/.kari/kanari_config/`:

- `kanari.yaml` - General configuration
- `kanari.keystore` - Encrypted wallet storage

### Integration

The keytool can be integrated into larger applications:

```bash
# Check if wallet exists
if kari keytool list | grep -q "0x1234..."; then
    echo "Wallet exists"
fi

# Get balance programmatically
balance=$(echo "0x1234..." | kari keytool balance | grep "Balance" | cut -d: -f2)
```

---

**Version**: 1.0.0  
**Last Updated**: 2024  
**For Support**: Refer to the Kanari SDK documentation
