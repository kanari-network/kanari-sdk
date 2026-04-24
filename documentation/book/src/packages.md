# Packages

Packages in Move are collections of modules that can be compiled, deployed, and managed together. This guide covers package structure, dependencies, and best practices.

## Package Structure

### Basic Package Layout

```
my_package/
├── Move.toml              # Package manifest
├── sources/               # Move source files
│   ├── token.move
│   ├── nft.move
│   └── dex.move
├── tests/                 # Test files
│   └── token_tests.move
└── doc_templates/         # Documentation templates
    └── overview.md
```

### Move.toml Manifest

```toml
[package]
name = "MyPackage"
version = "0.1.0"
published-at = "0x0"  # Set after deployment

[dependencies]
MoveStdlib = { git = "https://github.com/move-language/move.git", subdir = "language/move-stdlib", rev = "main" }
KanariSystem = { local = "../kanari-system" }

[addresses]
my_package = "0x0"
std = "0x1"
kanari_system = "0x2"
```

## Creating a Package

### Initialize Package

```bash
move new my_package
cd my_package
```

### Add Modules

```move
// sources/token.move
module my_package::token {
    use kanari_system::coin;
    
    struct TOKEN has drop {}
    
    public fun initialize(ctx: &mut TxContext) {
        // Initialize token
    }
}
```

### Build Package

```bash
move build
```

### Run Tests

```bash
move test
```

## Dependencies

### Git Dependencies

```toml
[dependencies]
MoveStdlib = { 
    git = "https://github.com/move-language/move.git",
    subdir = "language/move-stdlib",
    rev = "main"
}

KanariFrameworks = {
    git = "https://github.com/kanari-blockchain/kanari-sdk.git",
    subdir = "crates/kanari-frameworks/packages/kanari-system",
    rev = "v1.0.0"
}
```

### Local Dependencies

```toml
[dependencies]
KanariSystem = { local = "../kanari-system" }
MyLibrary = { local = "../my-library" }
```

### Version Pinning

```toml
[dependencies]
# Specific commit
KanariSystem = { 
    git = "https://github.com/kanari/kanari-sdk.git",
    rev = "abc123def456"
}

# Tag
KanariSystem = { 
    git = "https://github.com/kanari/kanari-sdk.git",
    tag = "v1.2.3"
}

# Branch (use cautiously)
KanariSystem = { 
    git = "https://github.com/kanari/kanari-sdk.git",
    branch = "develop"
}
```

## Address Management

### Development Addresses

```toml
[addresses]
my_package = "0x0"  # Placeholder for development
```

### Production Addresses

```toml
[addresses]
my_package = "0x1234567890abcdef"  # Deployed address
```

### Named Addresses

```move
// Use named address in code
use my_package::token;

// Resolves to actual address at compile time
```

## Package Publishing

### Prepare for Publishing

```toml
[package]
name = "MyToken"
version = "1.0.0"
published-at = "0x1234567890abcdef"  # Set after deployment
```

### Publish Command

```bash
move publish --profile mainnet
```

### Verify Publication

```bash
move verify-package
```

## Multi-Package Projects

### Workspace Structure

```
workspace/
├── Move.toml              # Workspace manifest
├── packages/
│   ├── token/
│   │   ├── Move.toml
│   │   └── sources/
│   ├── nft/
│   │   ├── Move.toml
│   │   └── sources/
│   └── dex/
│       ├── Move.toml
│       └── sources/
└── tests/
    └── integration_tests/
```

### Cross-Package Dependencies

```toml
# dex/Move.toml
[dependencies]
Token = { local = "../token" }
NFT = { local = "../nft" }
KanariSystem = { local = "../../kanari-system" }
```

## Best Practices

### 1. Semantic Versioning

```toml
[package]
version = "1.2.3"  # MAJOR.MINOR.PATCH

# MAJOR: Breaking changes
# MINOR: New features (backward compatible)
# PATCH: Bug fixes (backward compatible)
```

### 2. Dependency Hygiene

```toml
# Good: Pin specific versions
KanariSystem = { 
    git = "...",
    rev = "specific-commit-hash"
}

# Bad: Floating dependencies
KanariSystem = { 
    git = "...",
    branch = "main"  # Can break unexpectedly
}
```

### 3. Minimal Dependencies

```toml
# Only include what you need
[dependencies]
MoveStdlib = { ... }
KanariSystem = { ... }

# Don't add unnecessary dependencies
# UnusedLib = { ... }
```

### 4. Document Dependencies

```toml
[package]
name = "DeFi Protocol"
version = "1.0.0"

# Document why each dependency is needed
# MoveStdlib: Standard library functions
# KanariSystem: Coin, transfer, object modules
```

## Testing Packages

### Unit Tests

```move
#[test]
fun test_token_mint() {
    let ctx = &mut tx_context::dummy();
    let coins = token::mint(1000, ctx);
    assert!(coin::value(&coins) == 1000, 0);
}
```

### Integration Tests

```move
#[test]
fun test_full_workflow() {
    // Test across multiple modules
    let ctx = &mut tx_context::dummy();
    
    // Create token
    let (cap, _) = token::create(ctx);
    
    // Mint tokens
    let coins = token::mint_with_cap(&cap, 1000, ctx);
    
    // Transfer
    token::transfer(coins, @0x2);
    
    // Verify
    assert!(token::balance_of(@0x2) == 1000, 0);
}
```

## Package Deployment

### Development Deployment

```bash
# Deploy to devnet
move publish --profile devnet
```

### Staging Deployment

```bash
# Deploy to testnet
move publish --profile testnet
```

### Production Deployment

```bash
# Deploy to mainnet
move publish --profile mainnet
```

### Update Published Address

After deployment, update `Move.toml`:

```toml
[package]
published-at = "0xACTUAL_DEPLOYED_ADDRESS"

[addresses]
my_package = "0xACTUAL_DEPLOYED_ADDRESS"
```

## Common Patterns

### Library Package

```toml
# library/Move.toml
[package]
name = "MyLibrary"
version = "1.0.0"

[dependencies]
MoveStdlib = { ... }

# No published-at (library only)
```

### Application Package

```toml
# app/Move.toml
[package]
name = "MyApp"
version = "1.0.0"
published-at = "0x..."  # Will be deployed

[dependencies]
MoveStdlib = { ... }
KanariSystem = { ... }
MyLibrary = { local = "../library" }
```

### Upgradeable Package

```toml
[package]
name = "UpgradeableContract"
version = "2.0.0"  # Increment on upgrade
published-at = "0x..."
```

## Troubleshooting

### Dependency Conflicts

```bash
# Check dependency tree
move dependency-tree

# Resolve conflicts by pinning versions
```

### Address Mismatches

```bash
# Verify addresses match
move verify-addresses

# Update Move.toml with correct addresses
```

### Build Errors

```bash
# Clean and rebuild
move clean
move build

# Verbose output for debugging
move build --verbose
```

## Next Steps

- Learn about [Modules](modules-and-scripts.md) within packages
- Study [Deployment Strategies](../deployment/strategies.md)
- Explore [Package Security](../security/package-security.md)
