# Move CLI Guide — Complete Move Development Guide

This guide provides comprehensive documentation for developing, testing, and deploying Move smart contracts on the Kanari blockchain using the Kanari CLI.

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [Creating a New Package](#creating-a-new-package)
3. [Project Structure](#project-structure)
4. [Building Packages](#building-packages)
5. [Running Tests](#running-tests)
6. [Publishing Modules](#publishing-modules)
7. [Verifying Modules](#verifying-modules)
8. [Generating Documentation](#generating-documentation)
9. [Calling Functions](#calling-functions)
10. [Advanced Topics](#advanced-topics)
11. [Troubleshooting](#troubleshooting)

---

## Getting Started

### Prerequisites

Before you begin, ensure you have:

- **Rust & Cargo** (stable channel recommended)
- **Kanari SDK** cloned and built
- Basic understanding of the Move programming language

### Quick Setup

```bash
# Build the Kanari CLI
cargo build -p kanari

# Verify installation
cargo run -p kanari -- move --help
```

---

## Creating a New Package

### Basic Package Creation

Create a new Move package with a single command:

```bash
cargo run -p kanari -- move new my_token
```

This creates a complete project structure with:

- `Move.toml` — Package manifest with Kanari dependencies pre-configured
- `sources/` — Directory for your Move modules
- `tests/` — Directory for unit tests
- `.gitignore` — Standard ignore rules for Move projects

### What's Generated

The CLI automatically configures:

**Move.toml:**

```toml
[package]
name = "my_token"
version = "0.0.1"

[dependencies]
# Dependencies point to the Kanari SDK for framework and stdlib
KanariSystem = { git = "https://github.com/kanari-network/kanari-sdk.git", subdir = "crates/kanari-frameworks/packages/kanari-system", rev = "kanari-sdk" }
MoveStdlib = { git = "https://github.com/kanari-network/kanari-sdk.git", subdir = "crates/kanari-frameworks/packages/move-stdlib", rev = "kanari-sdk" }

[addresses]
my_token = "0x0"  # Will be replaced during publish
std = "0x1"
kanari_system = "0x2"

[dev-dependencies]
# Test-only dependencies can be added here

[dev-addresses]
# Override addresses for test/dev modes
```

**Initial Module File (`sources/my_token.move`):**

```move
module my_token::my_token {
    // Your code here
}
```

**Test File (`tests/my_token_tests.move`):**

```move
#[test_only]
module my_token::my_token_tests {
    // Your tests here
}
```

---

## Project Structure

A typical Kanari Move project follows this structure:

```
my_package/
├── Move.toml              # Package manifest
├── sources/               # Move source modules
│   ├── token.move
│   ├── nft.move
│   └── dex.move
├── tests/                 # Unit tests
│   ├── token_tests.move
│   └── integration_tests.move
├── doc_templates/         # Optional: Documentation templates
└── build/                 # Generated: Compiled bytecode
    ├── BytecodeModules/
    └── Sources/
```

### Key Directories

- **`sources/`**: Contains all `.move` files that will be compiled and published
- **`tests/`**: Contains test modules (only compiled in test mode)
- **`build/`**: Auto-generated directory containing compiled bytecode
- **`.move/`**: Cache directory (auto-generated, should be in .gitignore)

---

## Building Packages

### Basic Build

Compile your Move package to bytecode:

```bash
cargo run -p kanari -- move build
```

### Build with Custom Path

Specify a different package path:

```bash
cargo run -p kanari -- move build --package-path ./my_custom_package
```

### Build Output

After successful compilation:

- Bytecode modules are stored in `build/BytecodeModules/`
- Source maps are stored in `build/Sources/`
- Each module is compiled as a separate `.mv` file

### Build Modes

The CLI supports different build modes through the underlying Move compiler:

- **Debug mode** (default): Includes debug information for better error messages
- **Release mode**: Optimized bytecode (use `--release` flag if supported)

---

## Running Tests

### Basic Testing

Run all unit tests in your package:

```bash
cargo run -p kanari -- move test
```

### Filter Tests

Run only tests matching a specific pattern:

```bash
cargo run -p kanari -- move test --filter test_transfer
```

This runs tests containing "test_transfer" in their fully qualified name (e.g., `0x2::token::test_transfer_success`).

### Parallel Execution

Control the number of threads for parallel test execution:

```bash
cargo run -p kanari -- move test --threads 4
```

Default is 8 threads. Adjust based on your system capabilities.

### List All Tests

View all available tests without running them:

```bash
cargo run -p kanari -- move test --list
```

### Code Coverage

Collect coverage information (debug builds only):

```bash
cargo run -p kanari -- move test --coverage
```

Coverage data is stored for later analysis with `move coverage` tools.

### Verbose Mode

Get detailed test output:

```bash
cargo run -p kanari -- move test --verbose
```

### Gas Limit

Set a gas limit for each test:

```bash
cargo run -p kanari -- move test --gas-limit 1000000
```

### Test Statistics

Generate test statistics report:

```bash
cargo run -p kanari -- move test --statistics
# Or save to CSV
cargo run -p kanari -- move test --statistics=csv
```

### Writing Tests

Example test module:

```move
#[test_only]
module my_token::token_tests {
    use std::signer;
    use my_token::token;

    #[test]
    fun test_mint() {
        let admin = @0x1;
        let recipient = @0x2;
        
        token::mint(&admin, recipient, 1000);
        
        assert!(token::balance(recipient) == 1000, 1);
    }

    #[test(expected_failure)]
    fun test_unauthorized_mint() {
        let unauthorized = @0x3;
        let recipient = @0x2;
        
        // This should fail
        token::mint(&unauthorized, recipient, 1000);
    }
}
```

---

## Publishing Modules

### Prerequisites

Before publishing:

1. **Build your package** successfully
2. **Load the correct wallet** that matches your module address
3. **Ensure sufficient balance** for gas fees

### Basic Publish

Publish modules from the current directory:

```bash
cargo run -p kanari -- move publish
```

### Publish with Options

```bash
cargo run -p kanari -- move publish \
  --package-path ./my_package \
  --gas-limit 200000 \
  --gas-price 0 \
  --rpc http://127.0.0.1:6767
```

### Important: Address Matching

**Critical Rule**: The sender address MUST match the module's declared address in `Move.toml`.

**Common Error:**

```
MODULE_ADDRESS_DOES_NOT_MATCH_SENDER
```

**Solution:**

1. Check your wallet address:

   ```bash
   cargo run -p kanari -- keytool list
   ```

2. Update `Move.toml` to match:

   ```toml
   [addresses]
   my_package = "0xd731071eeda9c6dfa11ad6c2757875c9f30237ae164f59a4aa36e718de3a239c"
   ```

3. Rebuild the package:

   ```bash
   cargo run -p kanari -- move build
   ```

4. Load the matching wallet:

   ```bash
   cargo run -p kanari -- keytool load --address 0xd731071eeda9c6dfa11ad6c2757875c9f30237ae164f59a4aa36e718de3a239c
   ```

5. Publish again

### Hex Address vs Tagged Address

When publishing modules:

- Use **hex addresses** (e.g., `0xd731...`) for module identity
- **Tagged addresses** (e.g., `Ed25519:abc123...`) are only for transaction signing

The CLI handles this automatically, but be aware when debugging.

### Sequence Number Management

The CLI automatically manages account sequence numbers:

- Fetches base sequence before publishing
- Increments for each module published
- Prevents race conditions in multi-module packages

### Gas Estimation

The CLI estimates gas for each module:

```
Total estimated gas for all modules: 150000
```

Adjust `--gas-limit` if needed.

### Multi-Module Publishing

If your package contains multiple modules:

- Only modules matching the sender address are published
- Dependency modules are skipped automatically
- Each module gets its own transaction with incremented sequence

---

## Verifying Modules

Verify module bytecode locally via RPC before publishing:

### Verify from File

```bash
cargo run -p kanari -- move verify --file build/BytecodeModules/my_module.mv
```

### Verify from Package

Build and verify the first module:

```bash
cargo run -p kanari -- move verify --package-path ./my_package
```

### Custom RPC Endpoint

```bash
cargo run -p kanari -- move verify \
  --package-path ./my_package \
  --rpc http://127.0.0.1:6767
```

### Why Verify?

- Catch compilation errors early
- Ensure bytecode compatibility
- Validate module structure
- Security check before publishing

---

## Generating Documentation

Auto-generate API documentation from your Move source code:

```bash
cargo run -p kanari -- move docgen
```

### With Custom Path

```bash
cargo run -p kanari -- move docgen --package-path ./my_package
```

### Documentation Templates

Create custom documentation templates in `doc_templates/`:

```
my_package/
├── doc_templates/
│   ├── overview.md
│   └── module_template.md
```

### Generated Output

Documentation includes:

- Module descriptions
- Function signatures
- Type definitions
- Constants
- Struct fields

---

## Calling Functions

Call published Move functions on the blockchain:

```bash
cargo run -p kanari -- move call \
  --function my_module::transfer \
  --args @0x1 @0x2 1000 \
  --from 0xd731... \
  --gas-limit 50000
```

### Common Options

- `--function MODULE::FUNCTION`: Function to call
- `--args ARG1 ARG2 ...`: Function arguments
- `--from ADDRESS`: Transaction sender
- `--gas-limit N`: Gas limit
- `--gas-price N`: Gas price in Mist
- `--rpc URL`: RPC endpoint

### Example: Token Transfer

```bash
cargo run -p kanari -- move call \
  --function token::transfer \
  --args @0xRECIPIENT 500 \
  --from @0xSENDER \
  --gas-limit 100000
```

---

## Advanced Topics

### Named Addresses

Use named addresses in Move.toml for flexibility:

```toml
[addresses]
my_addr = "_"  # Placeholder, resolved at compile time
```

Override at compile time:

```bash
cargo run -p kanari -- move build --named-addresses my_addr=0x123
```

### Dev Dependencies

Add test-only dependencies:

```toml
[dev-dependencies]
TestFramework = { git = "...", rev = "main" }

[dev-addresses]
test_admin = "0xA"
test_user = "0xB"
```

These are only active in `--test` and `--dev` modes.

### Custom Native Functions

Kanari extends Move with system native functions:

- Event emission
- Object transfer tracking
- Cryptographic operations

Available under `kanari_system` (0x2) namespace.

### Hybrid PQC Support

Kanari supports hybrid post-quantum cryptography:

- Ed25519 + Dilithium3 signatures
- Seamless wallet integration
- Quantum-resistant transactions

Generate hybrid wallet:

```bash
cargo run -p kanari -- keytool create --curve hybrid
```

---

## Troubleshooting

### Common Issues

#### 1. MODULE_ADDRESS_DOES_NOT_MATCH_SENDER

**Problem**: Sender address doesn't match module address in Move.toml

**Solution**:

```bash
# Check wallet
cargo run -p kanari -- keytool list

# Update Move.toml address to match wallet
# Rebuild
cargo run -p kanari -- move build

# Load correct wallet
cargo run -p kanari -- keytool load --address <ADDRESS>

# Publish
cargo run -p kanari -- move publish
```

#### 2. No Selected Wallet

**Problem**: No wallet loaded for signing

**Solution**:

```bash
cargo run -p kanari -- keytool load --address <ADDRESS>
```

#### 3. Compilation Errors

**Problem**: Move syntax or type errors

**Solution**:

- Check error messages carefully
- Verify imports are correct
- Ensure dependencies are available
- Run `move build` to see full error output

#### 4. Test Failures

**Problem**: Tests failing unexpectedly

**Solution**:

```bash
# Run with verbose output
cargo run -p kanari -- move test --verbose

# Run specific test
cargo run -p kanari -- move test --filter test_name

# Check gas limits
cargo run -p kanari -- move test --gas-limit 10000000
```

#### 5. RPC Connection Failed

**Problem**: Cannot connect to RPC endpoint

**Solution**:

```bash
# Specify RPC explicitly
cargo run -p kanari -- move publish --rpc http://127.0.0.1:6767

# Check if node is running
curl http://127.0.0.1:6767
```

#### 6. Insufficient Gas

**Problem**: Not enough balance for transaction

**Solution**:

```bash
# Get faucet tokens (testnet)
cargo run -p kanari -- client faucet --address <ADDRESS>

# Check balance
cargo run -p kanari -- client balance --address <ADDRESS>
```

### Debug Mode

Enable debug logging:

```bash
RUST_LOG=debug cargo run -p kanari -- move publish
```

### Reset State

If encountering persistent issues:

```bash
# Remove local state
rm -rf ~/.kanari/kanari-db/

# Restart node
# Re-run genesis
cargo run -p kanari -- keytool list
```

---

## Best Practices

### 1. Version Control

Always commit `Move.toml` and source files, but exclude:

```gitignore
build/
.move/
*.mv
```

### 2. Testing Strategy

- Write tests for all public functions
- Test both success and failure cases
- Use `#[expected_failure]` for error paths
- Test edge cases and boundary conditions

### 3. Module Design

- Keep modules focused and small
- Use meaningful names
- Document public APIs
- Follow Move security patterns

### 4. Address Management

- Use hex addresses for module publishing
- Keep wallet addresses secure
- Never share private keys
- Use separate wallets for development and production

### 5. Gas Optimization

- Estimate gas before publishing
- Set appropriate gas limits
- Monitor actual gas usage
- Optimize expensive operations

---

## Additional Resources

- [Move Language Book](https://move-book.com/)
- [Kanari Core Documentation](../kanari-core/README.md)
- [Kanari System Modules](../../kanari-frameworks/packages/kanari-system/)
- [Move Standard Library](../../kanari-frameworks/packages/move-stdlib/)

---

## Command Reference

Quick reference for all Move CLI commands:

| Command | Description |
|---------|-------------|
| `move new <name>` | Create new Move package |
| `move build` | Compile package to bytecode |
| `move test` | Run unit tests |
| `move publish` | Publish modules to blockchain |
| `move verify` | Verify module bytecode |
| `move docgen` | Generate documentation |
| `move call` | Call published function |

### Global Options

- `--package-path PATH`: Specify package directory
- `--rpc URL`: RPC endpoint (default: <http://127.0.0.1:6767>)
- `--help`: Show help message

---

## Support

For issues or questions:

- **GitHub Issues**: [kanari-sdk/issues](https://github.com/kanari-network/kanari-sdk/issues)
- **Documentation**: [docs.kanarinetwork.site](https://docs.kanarinetwork.site)
- **Community**: Join our Discord server

Happy building! 🚀
