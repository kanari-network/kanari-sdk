# Kanari Wallet Guide

Complete guide for wallet management, key generation, and transaction signing in Kanari blockchain.

## Table of Contents

- [Wallet System Overview](#wallet-system-overview)
- [Creating a Wallet](#creating-a-wallet)
- [Loading a Wallet](#loading-a-wallet)
- [Publishing Move Modules](#publishing-move-modules)
- [Calling Move Functions](#calling-move-functions)
- [RPC Integration](#rpc-integration)
- [Security Best Practices](#security-best-practices)

## Wallet System Overview

Kanari uses a hierarchical deterministic (HD) wallet system with support for:

- **Ed25519** and **Secp256k1** cryptographic curves
- **BIP39** mnemonic seed phrases (12/15/18/21/24 words)
- **BIP32/BIP44** derivation paths
- **Encrypted wallet storage** with password protection
- **Post-quantum cryptography** support (Dilithium3, Kyber1024)

### Wallet Structure

Each wallet contains:

- **Address**: Hex-encoded account address (e.g., `0x1234...`)
- **Private Key**: Encrypted and stored securely
- **Seed Phrase**: BIP39 mnemonic for wallet recovery
- **Curve Type**: Ed25519, Secp256k1, Dilithium3, or Kyber1024

## Creating a Wallet

### Using Rust API

```rust
use kanari_crypto::{
    keys::{generate_keypair, CurveType},
    wallet::{generate_mnemonic, keypair_from_mnemonic, save_wallet},
};
use kanari_types::address::Address;

// Generate a new mnemonic (12 words by default)
let mnemonic = generate_mnemonic(12)?;
println!("Seed phrase: {}", mnemonic);
println!("⚠️  SAVE THIS PHRASE SECURELY!");

// Derive keypair from mnemonic
let keypair = keypair_from_mnemonic(&mnemonic, CurveType::Ed25519)?;

// Get address
let address = Address::from_public_key(&keypair.public_key, CurveType::Ed25519)?;
println!("Address: {}", address.to_hex());

// Save wallet with password
let password = "your_secure_password";
save_wallet(&address.to_hex(), &keypair, &mnemonic, password)?;
println!("✅ Wallet saved successfully!");
```

### Using CLI (Example)

```bash
# Generate new wallet
kanari wallet create --password mypassword

# Output:
# 🔑 Generating new wallet...
# 
# Seed Phrase (SAVE THIS SECURELY):
# abandon ability able about above absent absorb abstract absurd abuse access accident
# 
# Address: 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8
# Curve: Ed25519
# 
# ✅ Wallet saved to: ~/.kanari/wallets/0x742d35...bEb8.json
```

## Loading a Wallet

### Using Rust API

```rust
use kanari_crypto::wallet::load_wallet;

let address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8";
let password = "mypassword";

let wallet = load_wallet(address, password)?;
println!("✅ Wallet loaded");
println!("   Address: {}", wallet.address.to_hex());
println!("   Curve: {}", wallet.curve_type);
```

### Wallet Storage Location

Wallets are stored as encrypted JSON files:

**Linux/Mac:**

```
~/.kanari/wallets/<address>.json
```

**Windows:**

```
C:\Users\<username>\.kanari\wallets\<address>.json
```

## Publishing Move Modules

### Using CLI

```bash
# Publish with wallet authentication
kanari move publish \
  --package-path ./my_project \
  --sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --password mypassword \
  --gas-limit 1000000 \
  --gas-price 1000

# Test mode (skip signature)
kanari move publish \
  --package-path ./my_project \
  --sender 0x1 \
  --skip-signature
```

### Using Rust API

```rust
use kanari_crypto::wallet::load_wallet;
use kanari_move_runtime::{BlockchainEngine, SignedTransaction, Transaction};
use move_package::BuildConfig;
use std::path::PathBuf;

// Load wallet
let wallet = load_wallet("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", "mypassword")?;

// Compile Move package
let config = BuildConfig::default();
let package_path = PathBuf::from("./my_project");
let compiled = config.compile_package(&package_path, &mut std::io::stderr())?;

// Create blockchain engine
let engine = BlockchainEngine::new()?;

// Publish each module
for module_unit in compiled.all_modules() {
    let module = &module_unit.unit.module;
    
    // Serialize module bytecode
    let mut bytecode = vec![];
    module.serialize(&mut bytecode)?;
    
    // Create transaction
    let tx = Transaction::PublishModule {
        sender: wallet.address.to_hex(),
        module_bytes: bytecode,
        module_name: module.self_id().name().to_string(),
        gas_limit: 1_000_000,
        gas_price: 1_000,
    };
    
    // Sign transaction
    let mut signed_tx = SignedTransaction::new(tx);
    signed_tx.sign(&wallet.private_key, wallet.curve_type)?;
    
    // Submit to blockchain
    let tx_hash = engine.submit_transaction(signed_tx)?;
    println!("✅ Module published: {}", hex::encode(tx_hash));
}
```

## Calling Move Functions

### Using CLI

```bash
# Call function with wallet
kanari move call \
  --package 0x2 \
  --module coin \
  --function transfer \
  --sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --password mypassword \
  --args "0x3,1000" \
  --gas-limit 200000

# With type arguments
kanari move call \
  --package 0x2 \
  --module coin \
  --function transfer_generic \
  --type-args "0x1::aptos_coin::AptosCoin" \
  --args "0x3,1000" \
  --sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --password mypassword

# Dry run (gas estimation)
kanari move call \
  --package 0x2 \
  --module coin \
  --function transfer \
  --args "0x3,1000" \
  --sender 0x1 \
  --skip-signature \
  --dry-run
```

### Using Rust API

```rust
use kanari_crypto::wallet::load_wallet;
use kanari_move_runtime::{BlockchainEngine, SignedTransaction, Transaction};

// Load wallet
let wallet = load_wallet("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", "mypassword")?;

// Create transaction
let tx = Transaction::ExecuteFunction {
    sender: wallet.address.to_hex(),
    module: "0x2".to_string(),
    function: "transfer".to_string(),
    type_args: vec![],
    args: vec![
        bcs::to_bytes(&"0x3")?,  // recipient
        bcs::to_bytes(&1000u64)?, // amount
    ],
    gas_limit: 200_000,
    gas_price: 1_000,
};

// Sign transaction
let mut signed_tx = SignedTransaction::new(tx);
signed_tx.sign(&wallet.private_key, wallet.curve_type)?;

// Submit
let engine = BlockchainEngine::new()?;
let tx_hash = engine.submit_transaction(signed_tx)?;
println!("✅ Transaction submitted: {}", hex::encode(tx_hash));
```

### Argument Parsing

The CLI supports multiple argument types:

**Address:**

```bash
--args "0x1"              # Full hex address
--args "0x01"             # Auto-padded to 32 bytes
```

**Numbers:**

```bash
--args "1000"             # u64
--args "1000u128"         # u128
```

**Boolean:**

```bash
--args "true"             # bool true
--args "false"            # bool false
```

**String:**

```bash
--args '"hello world"'    # String (with quotes)
```

**Hex bytes:**

```bash
--args "0xdeadbeef"       # Raw bytes
```

**Vector:**

```bash
--args "[1,2,3]"          # Vector of u64
--args '["0x1","0x2"]'    # Vector of addresses
```

## RPC Integration

### Publishing via RPC

```rust
use kanari_rpc_client::RpcClient;
use kanari_rpc_api::PublishModuleRequest;
use kanari_crypto::wallet::load_wallet;

// Setup client
let client = RpcClient::new("http://127.0.0.1:3000");

// Load wallet
let wallet = load_wallet("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", "mypassword")?;

// Sign and prepare request
let request = PublishModuleRequest {
    sender: wallet.address.to_hex(),
    module_bytes: compiled_bytecode,
    module_name: "MyModule".to_string(),
    gas_limit: 1_000_000,
    gas_price: 1_000,
    signature: Some(signature_bytes),
};

// Submit
let status = client.publish_module(request).await?;
println!("✅ TX Hash: {}", status.hash);
```

### Calling Functions via RPC

```rust
use kanari_rpc_client::RpcClient;
use kanari_rpc_api::CallFunctionRequest;

let client = RpcClient::new("http://127.0.0.1:3000");
let wallet = load_wallet("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", "mypassword")?;

let request = CallFunctionRequest {
    sender: wallet.address.to_hex(),
    package: "0x2".to_string(),
    module: "coin".to_string(),
    function: "transfer".to_string(),
    type_args: vec![],
    args: vec![
        bcs::to_bytes(&"0x3")?,
        bcs::to_bytes(&1000u64)?,
    ],
    gas_limit: 200_000,
    gas_price: 1_000,
    signature: Some(signature_bytes),
};

let status = client.call_function(request).await?;
println!("✅ TX Hash: {}", status.hash);
```

### Available RPC Methods

```rust
// Account operations
client.get_account("0x1").await?;
client.get_balance("0x1").await?;

// Block operations
client.get_block(100).await?;
client.get_block_height().await?;

// Blockchain stats
client.get_stats().await?;

// Contract operations
client.publish_module(request).await?;
client.call_function(request).await?;
client.get_contract("0x2", "coin").await?;
client.list_contracts().await?;

// Transaction operations
client.submit_transaction(tx_data).await?;
```

## Security Best Practices

### 1. Seed Phrase Management

**DO:**

- ✅ Write down seed phrase on paper
- ✅ Store in a secure location (safe, bank vault)
- ✅ Never share with anyone
- ✅ Make multiple backups in different locations

**DON'T:**

- ❌ Store seed phrase digitally (text file, email, cloud)
- ❌ Take photos of seed phrase
- ❌ Share over messaging apps
- ❌ Enter seed phrase on untrusted websites

### 2. Password Protection

```rust
// Use strong passwords
let strong_password = "MyS3cur3P@ssw0rd!2024#";

// Avoid weak passwords
let weak_password = "password123";  // ❌ Don't use
```

**Password Requirements:**

- Minimum 12 characters
- Mix of uppercase, lowercase, numbers, symbols
- No common words or patterns
- Unique per wallet

### 3. Transaction Signing

```rust
// Always verify transaction details before signing
println!("⚠️  Review transaction:");
println!("   To: {}", recipient);
println!("   Amount: {}", amount);
println!("   Gas: {}", gas_limit);

// Require user confirmation in production
let confirmed = user_confirms_transaction()?;
if !confirmed {
    return Err(anyhow::anyhow!("Transaction cancelled by user"));
}

// Then sign
signed_tx.sign(&wallet.private_key, wallet.curve_type)?;
```

### 4. Key Storage

```rust
// Wallets are encrypted at rest
// Password never stored, only used for encryption/decryption

// For production applications, consider:
// - Hardware security modules (HSM)
// - Secure enclaves (TPM, SGX)
// - Multi-signature schemes
```

### 5. Network Security

```bash
# Use HTTPS for RPC endpoints
kanari move call --rpc https://rpc.kanari.network/v1 ...

# Avoid HTTP in production
kanari move call --rpc http://unsecure.com ...  # ❌ Insecure
```

## Advanced Features

### Multi-Signature Wallets

```rust
// Coming soon: Multi-sig support
// Require M-of-N signatures for transactions
```

### Hardware Wallet Integration

```rust
// Coming soon: Ledger/Trezor support
```

### Post-Quantum Cryptography

```rust
use kanari_crypto::keys::CurveType;

// Use quantum-resistant algorithms
let keypair = generate_keypair(CurveType::Dilithium3)?;
// or
let keypair = generate_keypair(CurveType::Kyber1024)?;
```

## Troubleshooting

### Wallet Not Found

```
Error: Failed to load wallet
```

**Solution:**

- Check wallet file exists in `~/.kanari/wallets/`
- Verify address format (must include `0x` prefix)
- Ensure correct password

### Invalid Signature

```
Error: Invalid transaction signature
```

**Solution:**

- Verify wallet password is correct
- Check sender address matches wallet
- Ensure curve type matches wallet configuration

### Insufficient Balance

```
Error: Insufficient balance for gas
```

**Solution:**

- Check account balance: `kanari account balance 0xYourAddress`
- Reduce gas limit or gas price
- Add funds to account

### Module Already Exists

```
Error: Module already exists at address
```

**Solution:**

- Use a different address for deployment
- Or upgrade the existing module (if supported)

## Examples

### Complete Workflow Example

```rust
use kanari_crypto::{
    keys::CurveType,
    wallet::{generate_mnemonic, keypair_from_mnemonic, save_wallet, load_wallet},
};
use kanari_types::address::Address;
use kanari_move_runtime::{BlockchainEngine, SignedTransaction, Transaction};

fn main() -> anyhow::Result<()> {
    // 1. Create wallet
    let mnemonic = generate_mnemonic(12)?;
    let keypair = keypair_from_mnemonic(&mnemonic, CurveType::Ed25519)?;
    let address = Address::from_public_key(&keypair.public_key, CurveType::Ed25519)?;
    save_wallet(&address.to_hex(), &keypair, &mnemonic, "mypassword")?;
    
    println!("✅ Wallet created: {}", address.to_hex());
    
    // 2. Load wallet
    let wallet = load_wallet(&address.to_hex(), "mypassword")?;
    
    // 3. Create and sign transaction
    let tx = Transaction::Transfer {
        from: wallet.address.to_hex(),
        to: "0x3".to_string(),
        amount: 1000,
        gas_limit: 100_000,
        gas_price: 1_000,
    };
    
    let mut signed_tx = SignedTransaction::new(tx);
    signed_tx.sign(&wallet.private_key, wallet.curve_type)?;
    
    // 4. Submit to blockchain
    let engine = BlockchainEngine::new()?;
    let tx_hash = engine.submit_transaction(signed_tx)?;
    
    println!("✅ Transaction submitted: {}", hex::encode(tx_hash));
    
    Ok(())
}
```

## Next Steps

- Read [MOVE_CLI_GUIDE.md](./MOVE_CLI_GUIDE.md) for Move development
- Check [CONTRACT_GUIDE.md](./crates/kanari-move-runtime/CONTRACT_GUIDE.md) for smart contracts
- See [RPC API Documentation](./crates/kanari-rpc-api/README.md) for server integration

## Support

For issues or questions:

- GitHub Issues: <https://github.com/jamesatomc/kanari-cp/issues>
- Documentation: <https://docs.kanari.network>
