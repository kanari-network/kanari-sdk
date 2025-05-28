# Move CLI Manual

The Move CLI is a comprehensive command-line interface for developing, testing, and deploying Move smart contracts on the Kanari blockchain platform.

## Table of Contents

- [Installation](#installation)
- [Basic Usage](#basic-usage)
- [Commands Overview](#commands-overview)
- [Development Workflow](#development-workflow)
- [Building and Testing](#building-and-testing)
- [Publishing Modules](#publishing-modules)
- [Calling Functions](#calling-functions)
- [Documentation and Analysis](#documentation-and-analysis)
- [Token Module Examples](#token-module-examples)
- [Advanced Features](#advanced-features)
- [Troubleshooting](#troubleshooting)

## Installation

The Move CLI is included with the Kanari SDK. Ensure you have the `kari` binary in your PATH.

```bash
# Verify installation
kari --version
```

## Basic Usage

```bash
kari move <command> [options]
```

To see all available commands:

```bash
kari move
```

## Commands Overview

| Command | Description |
|---------|-------------|
| `new` | Create a new Move package |
| `build` | Build the Move package |
| `test` | Run Move unit tests |
| `publish` | Publish Move module to blockchain |
| `call` | Call a function in a deployed Move module |
| `doc` | Generate documentation |
| `info` | Print address information |
| `coverage` | Inspect test coverage |
| `disassemble` | Disassemble Move bytecode |
| `errmap` | Generate error map |
| `migrate` | Migrate Move module |
| `sandbox` | Execute sandbox commands |

## Development Workflow

### 1. Create New Project

Create a new Move package:

```bash
kari move new my_token_project
```

This creates a directory structure:
```
my_token_project/
├── Move.toml
├── sources/
│   └── example.move
└── tests/
    └── example_tests.move
```

**Example Move.toml:**
```toml
[package]
name = "MyTokenProject"
version = "1.0.0"

[dependencies]
MoveStdlib = { git = "https://github.com/move-language/move.git", subdir = "language/move-stdlib", rev = "main" }

[addresses]
std = "0x1"
my_addr = "0x42"
```

### 2. Write Move Code

**Example token module (sources/token.move):**
```move
module my_addr::token {
    use std::signer;
    
    struct Token has key {
        value: u64,
    }
    
    public fun mint(account: &signer, value: u64) {
        move_to(account, Token { value });
    }
    
    public fun get_value(addr: address): u64 acquires Token {
        borrow_global<Token>(addr).value
    }
}
```

## Building and Testing

### Build Package

Compile your Move code:

```bash
cd my_token_project
kari move build
```

**Build output:**
```
BUILDING MyTokenProject
Including dependency MoveStdlib
Compiling 1 modules in MyTokenProject
```

### Run Tests

Execute unit tests:

```bash
kari move test
```

**Example test file (tests/token_tests.move):**
```move
#[test_only]
module my_addr::token_tests {
    use my_addr::token;
    use std::signer;
    
    #[test(account = @0x1)]
    public fun test_mint(account: signer) {
        token::mint(&account, 100);
        assert!(token::get_value(signer::address_of(&account)) == 100, 0);
    }
}
```

### Test with Coverage

Run tests with coverage analysis:

```bash
kari move test --coverage
kari move coverage
```

## Publishing Modules

### Basic Publishing

Publish your module to the blockchain:

```bash
kari move publish
```

### Publishing with Parameters

```bash
kari move publish --gas-budget 5000000 --address 0x123abc --password mypassword
```

**Parameters:**
- `--gas-budget <amount>` - Gas limit (default: 3,000,000)
- `--address <address>` - Publisher address (uses wallet if not specified)
- `--password <password>` - Wallet password
- `--skip-verify` - Skip bytecode verification

**Example output:**
```
Publishing package MyTokenProject...
Transaction executed successfully
Module ID: 0x123abc456def::token
Gas used: 2,847,592
Transaction hash: 0xabcdef1234567890...
```

### Publishing Prerequisites

1. **Ensure wallet is configured:**
```bash
kari keytool list
```

2. **Check balance:**
```bash
kari keytool balance
```

3. **Verify blockchain connection:**
```bash
# Check if node is running
curl http://127.0.0.1:30031
```

## Calling Functions

### Basic Function Call

Call a function in a deployed module:

```bash
kari move call --module-id 0x123abc::token --function get_value --args address:0x456def
```

### Function Call with Multiple Arguments

```bash
kari move call --module-id 0x123abc::token --function mint --args address:0x456def,u64:1000
```

### Call Parameters

| Parameter | Description | Example |
|-----------|-------------|---------|
| `--module-id` | Module address and name | `0x123::token` |
| `--function` | Function name | `mint` |
| `--args` | Function arguments | `u64:100,address:0x456` |
| `--gas-budget` | Gas limit | `1000000` |
| `--address` | Caller address | `0x123abc` |
| `--password` | Wallet password | `mypassword` |

### Argument Types

| Type | Format | Example |
|------|--------|---------|
| `u8` | `u8:value` | `u8:255` |
| `u64` | `u64:value` | `u64:1000000` |
| `u128` | `u128:value` | `u128:340282366920938463463374607431768211455` |
| `address` | `address:0x...` | `address:0x1` |
| `bool` | `bool:true/false` | `bool:true` |
| `vector<u8>` | `vector<u8>:hex` | `vector<u8>:0x48656c6c6f` |

## Documentation and Analysis

### Generate Documentation

Create HTML documentation for your modules:

```bash
kari move doc
```

**Generated files:**
```
doc/
├── index.html
├── my_addr_token.html
└── assets/
```

### Print Address Information

Display address mapping:

```bash
kari move info
```

### Disassemble Bytecode

View compiled bytecode:

```bash
kari move disassemble --interactive
```

### Generate Error Map

Create error code mapping:

```bash
kari move errmap
```

## Token Module Examples

### Complete Token Module

```move
module my_addr::advanced_token {
    use std::signer;
    use std::error;
    
    /// Token resource stored under user accounts
    struct Token has key {
        value: u64,
    }
    
    /// Capability for minting tokens
    struct MintCapability has key {}
    
    /// Error codes
    const EINSUFFICIENT_BALANCE: u64 = 1;
    const ENOT_AUTHORIZED: u64 = 2;
    const EALREADY_HAS_TOKEN: u64 = 3;
    
    /// Initialize the token module
    public fun initialize(account: &signer) {
        move_to(account, MintCapability {});
    }
    
    /// Mint tokens to an account
    public fun mint(
        _mint_cap: &MintCapability,
        account: &signer,
        amount: u64
    ) {
        let addr = signer::address_of(account);
        if (exists<Token>(addr)) {
            let token = borrow_global_mut<Token>(addr);
            token.value = token.value + amount;
        } else {
            move_to(account, Token { value: amount });
        }
    }
    
    /// Transfer tokens between accounts
    public fun transfer(
        from: &signer,
        to_addr: address,
        amount: u64
    ) acquires Token {
        let from_addr = signer::address_of(from);
        assert!(exists<Token>(from_addr), error::not_found(EINSUFFICIENT_BALANCE));
        
        let from_token = borrow_global_mut<Token>(from_addr);
        assert!(from_token.value >= amount, error::invalid_argument(EINSUFFICIENT_BALANCE));
        
        from_token.value = from_token.value - amount;
        
        if (exists<Token>(to_addr)) {
            let to_token = borrow_global_mut<Token>(to_addr);
            to_token.value = to_token.value + amount;
        } else {
            // This would require the recipient to sign, so we'll just abort
            abort error::invalid_argument(EALREADY_HAS_TOKEN)
        }
    }
    
    /// Get token balance
    public fun balance(addr: address): u64 acquires Token {
        if (exists<Token>(addr)) {
            borrow_global<Token>(addr).value
        } else {
            0
        }
    }
}
```

### Deployment and Usage Examples

1. **Deploy the module:**
```bash
kari move publish --gas-budget 5000000
```

2. **Initialize token system:**
```bash
kari move call --module-id 0x123::advanced_token --function initialize
```

3. **Mint tokens:**
```bash
kari move call --module-id 0x123::advanced_token --function mint --args address:0x456,u64:1000
```

4. **Check balance:**
```bash
kari move call --module-id 0x123::advanced_token --function balance --args address:0x456
```

5. **Transfer tokens:**
```bash
kari move call --module-id 0x123::advanced_token --function transfer --args address:0x789,u64:100
```

## Advanced Features

### Working with Capabilities

Move uses capability-based security. Here's an example:

```move
module my_addr::capability_example {
    struct AdminCap has key, store {}
    
    public fun create_admin(account: &signer) {
        move_to(account, AdminCap {});
    }
    
    public fun admin_only_function(_admin_cap: &AdminCap) {
        // Only callable with admin capability
    }
}
```

### Resource Management

Resources in Move are linear types that must be moved or destroyed:

```move
module my_addr::resource_example {
    struct Coin has key {
        value: u64,
    }
    
    public fun withdraw(account: &signer, amount: u64): Coin acquires Coin {
        let addr = signer::address_of(account);
        let coin = borrow_global_mut<Coin>(addr);
        coin.value = coin.value - amount;
        Coin { value: amount }
    }
    
    public fun deposit(account: &signer, coin: Coin) acquires Coin {
        let Coin { value } = coin; // Destructure to extract value
        let addr = signer::address_of(account);
        let existing_coin = borrow_global_mut<Coin>(addr);
        existing_coin.value = existing_coin.value + value;
    }
}
```

### Generic Programming

Move supports generic types and functions:

```move
module my_addr::generic_example {
    struct Container<T: store> has key {
        item: T,
    }
    
    public fun store_item<T: store>(account: &signer, item: T) {
        move_to(account, Container { item });
    }
    
    public fun retrieve_item<T: store>(addr: address): T acquires Container {
        let Container { item } = move_from<Container<T>>(addr);
        item
    }
}
```

### Module Dependencies

**Move.toml with dependencies:**
```toml
[package]
name = "MyDApp"
version = "1.0.0"

[dependencies]
MoveStdlib = { git = "https://github.com/move-language/move.git", subdir = "language/move-stdlib", rev = "main" }
AptosFramework = { git = "https://github.com/aptos-labs/aptos-core.git", subdir = "aptos-move/framework/aptos-framework", rev = "main" }

[addresses]
std = "0x1"
aptos_framework = "0x1"
my_addr = "0x42"
```

## Troubleshooting

### Common Build Errors

#### "Module not found"
```
error: Unbound module 'my_addr::nonexistent'
```
**Solution:** Check module names and addresses in Move.toml

#### "Address not found"
```
error: Invalid address '0xinvalid'
```
**Solution:** Ensure addresses are properly formatted (0x prefix, correct length)

#### "Resource not found"
```
error: Resource does not exist at address
```
**Solution:** Ensure resources are properly initialized before access

### Common Publishing Errors

#### "Insufficient gas"
```
Transaction failed: Out of gas
```
**Solution:** Increase gas budget:
```bash
kari move publish --gas-budget 10000000
```

#### "Module verification failed"
```
error: Bytecode verification error
```
**Solution:** Check for compilation errors or use `--skip-verify`:
```bash
kari move publish --skip-verify
```

#### "Insufficient balance"
```
error: Account balance too low
```
**Solution:** Check account balance and add funds:
```bash
kari keytool balance
```

### Common Function Call Errors

#### "Function not found"
```
error: Function 'nonexistent_function' not found
```
**Solution:** Verify function name and module address

#### "Type mismatch"
```
error: Expected u64, found u8
```
**Solution:** Check argument types:
```bash
# Wrong
kari move call --module-id 0x123::token --function mint --args u8:100

# Correct
kari move call --module-id 0x123::token --function mint --args u64:100
```

#### "Permission denied"
```
error: EINSUFFICIENT_PRIVILEGE
```
**Solution:** Ensure you have the required capabilities or are calling from the correct address

### Debugging Tips

1. **Use verbose mode:**
```bash
kari move build --verbose
```

2. **Check bytecode:**
```bash
kari move disassemble --debug
```

3. **Run tests frequently:**
```bash
kari move test --verbose
```

4. **Use print statements in tests:**
```move
#[test]
fun debug_test() {
    std::debug::print(&string::utf8(b"Debug message"));
}
```

### Environment Issues

#### "Node not running"
```
error: Connection refused (127.0.0.1:30031)
```
**Solution:** Start the Kanari node:
```bash
kari node start
```

#### "Wallet not configured"
```
error: No wallet found
```
**Solution:** Create or import a wallet:
```bash
kari keytool generate
```

### Performance Optimization

1. **Optimize gas usage:**
   - Use efficient data structures
   - Minimize storage operations
   - Batch operations when possible

2. **Module size optimization:**
   - Split large modules
   - Remove unused dependencies
   - Use generic functions

3. **Testing strategy:**
   - Write comprehensive unit tests
   - Use property-based testing
   - Test edge cases

## Best Practices

### Code Organization

1. **Module structure:**
```
sources/
├── core/
│   ├── token.move
│   └── governance.move
├── utils/
│   └── math.move
└── tests/
    ├── token_tests.move
    └── governance_tests.move
```

2. **Naming conventions:**
   - Modules: `snake_case`
   - Functions: `snake_case`
   - Structs: `PascalCase`
   - Constants: `UPPER_CASE`

### Security Considerations

1. **Capability-based access control**
2. **Resource linear typing**
3. **Input validation**
4. **Error handling**
5. **Test coverage**

### Development Workflow

1. **Write tests first** (TDD approach)
2. **Compile frequently** to catch errors early
3. **Use version control** for your Move projects
4. **Document your code** with comments
5. **Review before publishing** to mainnet

---

**Version**: 1.0.0  
**Last Updated**: 2024  
**For Support**: Refer to the Kanari SDK documentation or Move language documentation
