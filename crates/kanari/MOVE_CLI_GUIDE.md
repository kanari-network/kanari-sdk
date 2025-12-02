# Kanari Move CLI Guide

Complete guide for developing, building, testing, publishing, and calling Move smart contracts on Kanari blockchain.

## Table of Contents

- [Overview](#overview)
- [Installation](#installation)
- [Move Commands](#move-commands)
  - [new](#new---create-new-move-project)
  - [build](#build---compile-move-package)
  - [test](#test---run-unit-tests)
  - [docgen](#docgen---generate-documentation)
  - [publish](#publish---deploy-modules)
  - [call](#call---execute-functions)
- [Wallet Integration](#wallet-integration)
- [Complete Workflow](#complete-workflow)
- [Examples](#examples)
- [Best Practices](#best-practices)

## Overview

Kanari Move CLI provides comprehensive tools for Move smart contract development:

- **Project Management**: Create and organize Move projects
- **Compilation**: Build Move modules with dependency resolution
- **Testing**: Run comprehensive unit tests with coverage
- **Documentation**: Auto-generate API documentation
- **Deployment**: Publish modules with wallet authentication
- **Execution**: Call functions with type safety and gas estimation

### Prerequisites

```bash
# Install Kanari CLI
cargo install --path crates/kanari

# Verify installation
kanari --version
```

## Installation

### From Source

```bash
git clone https://github.com/jamesatomc/kanari-cp.git
cd kanari-cp
cargo build --release
export PATH="$PWD/target/release:$PATH"
```

### Verify Setup

```bash
kanari move --help
```

## Move Commands

### new - Create New Move Project

Create a new Move package with standard structure.

**Usage:**

```bash
kanari move new <PROJECT_NAME> [OPTIONS]
```

**Options:**

- `<PROJECT_NAME>` - Name of the new project (required)

**Example:**

```bash
# Create new project
kanari move new my_token

# Output:
# ✅ Created new Move project: my_token
# 
# Project structure:
#   my_token/
#   ├── Move.toml
#   ├── sources/
#   │   └── my_token.move
#   └── tests/
#       └── my_token_tests.move
```

**Generated Move.toml:**

```toml
[package]
name = "my_token"
version = "0.1.0"

[dependencies]
MoveStdlib = { local = "../move-stdlib" }

[addresses]
my_token = "0x1"
```

**Generated Module Template:**

```move
module my_token::my_token {
    use std::signer;

    struct MyToken has key {
        value: u64
    }

    public fun initialize(account: &signer) {
        move_to(account, MyToken { value: 0 });
    }

    public fun increment(account: &signer) acquires MyToken {
        let token = borrow_global_mut<MyToken>(signer::address_of(account));
        token.value = token.value + 1;
    }

    public fun get_value(addr: address): u64 acquires MyToken {
        borrow_global<MyToken>(addr).value
    }
}
```

---

### build - Compile Move Package

Compile Move modules and check for errors.

**Usage:**

```bash
kanari move build [OPTIONS]
```

**Options:**

- `--package-path <PATH>` - Path to Move package (default: current directory)
- `--dev` - Build in development mode
- `--doc` - Generate documentation during build

**Example:**

```bash
# Build current package
cd my_token
kanari move build

# Output:
# 📦 Building Move package...
# INCLUDING DEPENDENCY MoveStdlib
# BUILDING my_token
# ✅ Build successful
#    Modules: 1
#    Time: 1.23s

# Build specific package
kanari move build --package-path ./my_token

# Build with documentation
kanari move build --doc
```

**Build Output:**

```
build/
├── my_token/
│   ├── bytecode_modules/
│   │   └── my_token.mv
│   ├── source_maps/
│   └── BuildInfo.yaml
```

---

### test - Run Unit Tests

Execute Move unit tests with detailed reporting.

**Usage:**

```bash
kanari move test [OPTIONS]
```

**Options:**

- `--package-path <PATH>` - Path to Move package
- `--filter <PATTERN>` - Run only tests matching pattern
- `--coverage` - Generate coverage report

**Example:**

```bash
# Run all tests
kanari move test

# Output:
# 🧪 Running Move tests...
# 
# Running Move unit tests
# [ PASS    ] 0x1::my_token::test_initialize
# [ PASS    ] 0x1::my_token::test_increment
# [ PASS    ] 0x1::my_token::test_get_value
# 
# Test result: OK. Total tests: 3; passed: 3; failed: 0
# ✅ All tests passed!

# Run specific test
kanari move test --filter increment

# Generate coverage
kanari move test --coverage
```

**Test Example:**

```move
#[test_only]
module my_token::my_token_tests {
    use std::signer;
    use my_token::my_token;

    #[test(account = @0x1)]
    fun test_initialize(account: &signer) {
        my_token::initialize(account);
        assert!(my_token::get_value(@0x1) == 0, 0);
    }

    #[test(account = @0x1)]
    fun test_increment(account: &signer) {
        my_token::initialize(account);
        my_token::increment(account);
        assert!(my_token::get_value(@0x1) == 1, 0);
    }

    #[test]
    #[expected_failure]
    fun test_get_value_not_initialized() {
        my_token::get_value(@0x1);
    }
}
```

---

### docgen - Generate Documentation

Generate HTML documentation from Move modules.

**Usage:**

```bash
kanari move docgen [OPTIONS]
```

**Options:**

- `--package-path <PATH>` - Path to Move package
- `--output-dir <DIR>` - Output directory (default: `./docs`)

**Example:**

```bash
# Generate documentation
kanari move docgen

# Output:
# 📚 Generating documentation...
# ✅ Documentation generated
#    Location: ./docs/index.html
#    Modules: 1

# Custom output directory
kanari move docgen --output-dir ./api-docs

# Open in browser
open docs/index.html
```

---

### publish - Deploy Modules

Publish Move modules to the blockchain with wallet authentication.

**Usage:**

```bash
kanari move publish [OPTIONS]
```

**Options:**

- `--package-path <PATH>` - Path to Move package (default: current directory)
- `--sender <ADDRESS>` - Account address from wallet (required)
- `--password <PASSWORD>` - Wallet password for signing (required unless --skip-signature)
- `--skip-signature` - Skip signature for testing only
- `--gas-limit <AMOUNT>` - Maximum gas units (default: 1000000)
- `--gas-price <PRICE>` - Gas price in Mist (default: 1000)
- `--rpc <URL>` - RPC endpoint (default: <http://127.0.0.1:3000>)

**Examples:**

```bash
# Publish with wallet authentication
kanari move publish \
  --sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --password mypassword

# Output:
# 📦 Building Move package...
# ✅ Package compiled successfully!
#    Modules: 1
# 
# 🔐 Wallet loaded: 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 (curve: Ed25519)
# 
# 📤 Publishing modules to blockchain...
#    RPC: http://127.0.0.1:3000
# 
#   📝 Module: my_token
#      Size: 1234 bytes
#      Address: 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8
#      Functions: 3
#      Estimated Gas: 62340 units
#      Estimated Cost: 62340000 Mist (0.062340 KANARI)
#      🔑 Signing transaction with Ed25519 key...
#      ⚠️  Not yet implemented: Blockchain submission
# 
# ✅ Package build and validation complete!

# Test mode (skip signature)
kanari move publish \
  --sender 0x1 \
  --skip-signature

# Custom gas settings
kanari move publish \
  --sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --password mypassword \
  --gas-limit 2000000 \
  --gas-price 2000

# Publish to custom RPC
kanari move publish \
  --sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --password mypassword \
  --rpc https://rpc.kanari.network/v1
```

**With Blockchain Engine (Feature Flag):**

```rust
// In code with "blockchain" feature enabled
use kanari::command::move_cli::Publish;

let publish = Publish {
    package_path: Some(PathBuf::from("./my_token")),
    gas_limit: 1_000_000,
    gas_price: 1_000,
    sender: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8".to_string(),
    password: Some("mypassword".to_string()),
    skip_signature: false,
    rpc_endpoint: "http://127.0.0.1:3000".to_string(),
};

// Direct engine integration
publish.execute_with_engine(None, BuildConfig::default())?;
```

---

### call - Execute Functions

Call Move functions on deployed modules with type safety.

**Usage:**

```bash
kanari move call [OPTIONS]
```

**Options:**

- `--package <ADDRESS>` - Object ID/address of the package (required)
- `--module <NAME>` - Module name (required)
- `--function <NAME>` - Function name (required)
- `--sender <ADDRESS>` - Caller address from wallet (required)
- `--password <PASSWORD>` - Wallet password for signing (required unless --skip-signature)
- `--skip-signature` - Skip signature for testing
- `--type-args <TYPE>...` - Generic type arguments
- `--args <ARG>...` - Function arguments
- `--gas-limit <AMOUNT>` - Maximum gas units (default: 200000)
- `--gas-price <PRICE>` - Gas price in Mist (default: 1000)
- `--rpc <URL>` - RPC endpoint (default: <http://localhost:9944>)
- `--dry-run` - Estimate gas without executing

**Argument Types:**

| Type | Format | Example |
|------|--------|---------|
| Address | `0x...` | `0x1` or `0x742d35Cc...` |
| u64 | Number | `1000` |
| u128 | Number with suffix | `1000u128` |
| bool | `true`/`false` | `true` |
| String | Quoted | `"hello"` |
| Vector | `[...]` | `[1,2,3]` |
| Hex bytes | `0x...` | `0xdeadbeef` |

**Examples:**

**Simple Transfer:**

```bash
kanari move call \
  --package 0x1 \
  --module coin \
  --function transfer \
  --sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --password mypassword \
  --args "0x3" "1000"

# Output:
# 📞 Preparing function call...
# 
# 📋 Call Details:
#    Package: 0x1
#    Module: coin
#    Function: transfer
#    Sender: 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8
#    Gas Limit: 200000
#    Gas Price: 1000
#    🔐 Wallet loaded: 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 (curve: Ed25519)
#    Arguments: 2 args provided
#      [0]: 0x3
#      [1]: 1000
# 
# ⛽ Gas Estimation:
#    Estimated: 35800 units
#    Limit: 200000 units
#    Total Cost: 35800000 Mist
# 
# 🔨 Creating transaction...
#    🔑 Signing transaction with Ed25519 key...
#    ⚠️  Not yet implemented: Blockchain submission
# 
# ✅ Function call prepared!
# 
# 💡 Next steps:
#    • Check transaction status
#    • View execution results on explorer
```

**With Generic Types:**

```bash
kanari move call \
  --package 0x1 \
  --module coin \
  --function transfer_generic \
  --sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --password mypassword \
  --type-args "0x1::aptos_coin::AptosCoin" \
  --args "0x3" "1000"
```

**Multiple Type Arguments:**

```bash
kanari move call \
  --package 0x1 \
  --module swap \
  --function swap_exact \
  --type-args "0x1::coin::USDC" "0x1::coin::USDT" \
  --args "1000" "950"
```

**Complex Arguments:**

```bash
# Vector of addresses
kanari move call \
  --package 0x2 \
  --module multisig \
  --function initialize \
  --args '["0x1","0x2","0x3"]' "2"

# Boolean and string
kanari move call \
  --package 0x2 \
  --module nft \
  --function mint \
  --args "true" '"My NFT Name"' "0xabcd"
```

**Dry Run (Gas Estimation):**

```bash
kanari move call \
  --package 0x1 \
  --module coin \
  --function transfer \
  --sender 0x1 \
  --skip-signature \
  --args "0x3" "1000" \
  --dry-run

# Output:
# 📞 Preparing function call...
# ...
# ⛽ Gas Estimation:
#    Estimated: 35800 units
#    Limit: 200000 units
#    Total Cost: 35800000 Mist
# 
# 🧪 Dry run mode - not executing
```

**Test Mode:**

```bash
# Skip wallet authentication for testing
kanari move call \
  --package 0x2 \
  --module test \
  --function test_func \
  --sender 0x1 \
  --skip-signature \
  --args "100"
```

---

## Wallet Integration

All `publish` and `call` commands support wallet-based authentication.

### Setup Wallet

```bash
# Create wallet (example - implement in your app)
kanari wallet create --password mypassword

# Wallet is saved to: ~/.kanari/wallets/0x742d35...bEb8.json
```

### Wallet Structure

```json
{
  "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8",
  "encrypted_private_key": "...",
  "curve_type": "Ed25519",
  "created_at": "2025-11-28T10:30:00Z"
}
```

### Using Wallet

```bash
# Publish with wallet
kanari move publish \
  --sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --password mypassword

# Call with wallet
kanari move call \
  --package 0x1 \
  --module coin \
  --function transfer \
  --sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --password mypassword \
  --args "0x3" "1000"
```

### Security

- Wallets are encrypted with password
- Private keys never exposed in logs
- Signatures created in-memory only
- Use `--skip-signature` only for testing

---

## Complete Workflow

### 1. Create New Project

```bash
kanari move new defi_swap
cd defi_swap
```

### 2. Write Move Code

```move
// sources/swap.move
module defi_swap::swap {
    use std::signer;

    struct LiquidityPool has key {
        token_a: u64,
        token_b: u64,
    }

    public fun initialize(account: &signer) {
        move_to(account, LiquidityPool {
            token_a: 0,
            token_b: 0,
        });
    }

    public fun add_liquidity(
        account: &signer,
        amount_a: u64,
        amount_b: u64
    ) acquires LiquidityPool {
        let pool = borrow_global_mut<LiquidityPool>(
            signer::address_of(account)
        );
        pool.token_a = pool.token_a + amount_a;
        pool.token_b = pool.token_b + amount_b;
    }

    public fun get_reserves(addr: address): (u64, u64) acquires LiquidityPool {
        let pool = borrow_global<LiquidityPool>(addr);
        (pool.token_a, pool.token_b)
    }
}
```

### 3. Write Tests

```move
// tests/swap_tests.move
#[test_only]
module defi_swap::swap_tests {
    use std::signer;
    use defi_swap::swap;

    #[test(account = @0x1)]
    fun test_initialize(account: &signer) {
        swap::initialize(account);
        let (reserve_a, reserve_b) = swap::get_reserves(@0x1);
        assert!(reserve_a == 0, 0);
        assert!(reserve_b == 0, 1);
    }

    #[test(account = @0x1)]
    fun test_add_liquidity(account: &signer) {
        swap::initialize(account);
        swap::add_liquidity(account, 1000, 2000);
        let (reserve_a, reserve_b) = swap::get_reserves(@0x1);
        assert!(reserve_a == 1000, 0);
        assert!(reserve_b == 2000, 1);
    }
}
```

### 4. Build & Test

```bash
# Build
kanari move build
# ✅ Build successful

# Run tests
kanari move test
# ✅ All tests passed!

# Generate docs
kanari move docgen
# ✅ Documentation generated
```

### 5. Publish to Blockchain

```bash
kanari move publish \
  --sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --password mypassword \
  --gas-limit 1500000

# ✅ Module published
# TX Hash: 0xabcd1234...
```

### 6. Call Functions

```bash
# Initialize pool
kanari move call \
  --package 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --module swap \
  --function initialize \
  --sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --password mypassword

# Add liquidity
kanari move call \
  --package 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --module swap \
  --function add_liquidity \
  --sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --password mypassword \
  --args "1000" "2000"

# Check reserves (view function)
kanari move call \
  --package 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8 \
  --module swap \
  --function get_reserves \
  --sender 0x1 \
  --skip-signature \
  --args "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8"
```

---

## Examples

### Token Contract

```move
module my_token::token {
    use std::signer;

    struct Token has key {
        balance: u64
    }

    public fun mint(account: &signer, amount: u64) {
        let addr = signer::address_of(account);
        if (!exists<Token>(addr)) {
            move_to(account, Token { balance: amount });
        } else {
            let token = borrow_global_mut<Token>(addr);
            token.balance = token.balance + amount;
        }
    }

    public fun transfer(from: &signer, to: address, amount: u64) acquires Token {
        let from_addr = signer::address_of(from);
        let from_token = borrow_global_mut<Token>(from_addr);
        assert!(from_token.balance >= amount, 1);
        from_token.balance = from_token.balance - amount;

        if (!exists<Token>(to)) {
            // Create new account - in real implementation, handle differently
            assert!(false, 2);
        } else {
            let to_token = borrow_global_mut<Token>(to);
            to_token.balance = to_token.balance + amount;
        }
    }

    public fun balance(addr: address): u64 acquires Token {
        borrow_global<Token>(addr).balance
    }
}
```

**Deploy & Use:**

```bash
# Publish
kanari move publish --sender 0x742d35... --password mypass

# Mint tokens
kanari move call \
  --package 0x742d35... --module token --function mint \
  --sender 0x742d35... --password mypass \
  --args "1000000"

# Transfer
kanari move call \
  --package 0x742d35... --module token --function transfer \
  --sender 0x742d35... --password mypass \
  --args "0x3" "100"

# Check balance
kanari move call \
  --package 0x742d35... --module token --function balance \
  --sender 0x1 --skip-signature \
  --args "0x742d35..."
```

---

## Best Practices

### Development

1. **Start with Tests**

   ```bash
   # Write tests first
   kanari move test --filter test_name
   ```

2. **Use Type Safety**

   ```move
   // Good: Strong typing
   public fun transfer(from: &signer, to: address, amount: u64)
   
   // Avoid: Weak typing
   public fun transfer(from: &signer, to: vector<u8>, amount: u64)
   ```

3. **Document Your Code**

   ```move
   /// Transfer tokens from sender to recipient
   /// 
   /// # Arguments
   /// * `from` - Sender account
   /// * `to` - Recipient address
   /// * `amount` - Amount to transfer
   /// 
   /// # Panics
   /// * If sender has insufficient balance
   public fun transfer(from: &signer, to: address, amount: u64)
   ```

### Testing

1. **Test All Edge Cases**

   ```move
   #[test]
   #[expected_failure(abort_code = 1)]
   fun test_insufficient_balance() {
       // Test failure scenarios
   }
   ```

2. **Use Test-Only Code**

   ```move
   #[test_only]
   module my_token::test_helpers {
       // Helper functions for tests only
   }
   ```

### Deployment

1. **Always Test Before Publishing**

   ```bash
   kanari move build && kanari move test
   ```

2. **Use Appropriate Gas Limits**

   ```bash
   # Start conservative
   --gas-limit 1000000
   
   # Adjust based on actual usage
   --gas-limit 500000
   ```

3. **Verify on Testnet First**

   ```bash
   kanari move publish \
     --rpc https://testnet-rpc.kanari.network \
     --sender 0x...
   ```

### Security

1. **Never Hardcode Secrets**

   ```bash
   # Bad
   kanari move publish --password mypassword
   
   # Good
   read -s PASSWORD
   kanari move publish --password "$PASSWORD"
   ```

2. **Use `--dry-run` for Testing**

   ```bash
   kanari move call ... --dry-run
   ```

3. **Validate All Inputs**

   ```move
   public fun transfer(from: &signer, to: address, amount: u64) {
       assert!(amount > 0, ERR_INVALID_AMOUNT);
       assert!(to != @0x0, ERR_INVALID_ADDRESS);
       // ...
   }
   ```

---

## Troubleshooting

### Build Errors

**Problem:** `MODULE_ADDRESS_DOES_NOT_MATCH_SENDER`

```bash
# Fix: Update Move.toml addresses
[addresses]
my_module = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8"
```

**Problem:** `UNBOUND_MODULE`

```bash
# Fix: Add dependency in Move.toml
[dependencies]
MoveStdlib = { local = "../move-stdlib" }
```

### Test Failures

**Problem:** Tests timeout

```bash
# Increase timeout
kanari move test --timeout 60
```

### Publish Issues

**Problem:** `Failed to load wallet`

```bash
# Check wallet exists
ls ~/.kanari/wallets/

# Verify address format
--sender 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8  # Correct
--sender 742d35Cc6634C0532925a3b844Bc9e7595f0bEb8   # Missing 0x
```

**Problem:** `Insufficient balance for gas`

```bash
# Check balance first
kanari account balance 0x742d35...

# Reduce gas limit
--gas-limit 500000
```

### Call Issues

**Problem:** `Invalid argument type`

```bash
# Check argument format
--args "0x3" "1000"        # Correct
--args 0x3 1000            # May fail - use quotes

# For strings, use double quotes inside single quotes
--args '"My String"'
```

---

## Additional Resources

- **Wallet Guide**: See [WALLET_GUIDE.md](../../WALLET_GUIDE.md) for wallet management
- **Contract System**: See [CONTRACT_GUIDE.md](../kanari-move-runtime/CONTRACT_GUIDE.md) for advanced contracts
- **Move Language**: <https://move-language.github.io/move/>
- **Kanari Docs**: <https://docs.kanari.network>

---

## Support

For help:

- GitHub Issues: <https://github.com/jamesatomc/kanari-cp/issues>
- Discord: <https://discord.gg/kanari>
- Documentation: <https://docs.kanari.network>
