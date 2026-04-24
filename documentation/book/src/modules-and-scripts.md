# Modules and Scripts

Modules are the fundamental building blocks of Move programs. They encapsulate code, types, and functionality in a reusable way. This guide covers module structure, organization, and best practices.

## Module Basics

### Module Declaration

```move
module my_address::my_module {
    // Module contents
}
```

### Simple Module

```move
module 0x1::math_utils {
    public fun add(a: u64, b: u64): u64 {
        a + b
    }
    
    public fun multiply(a: u64, b: u64): u64 {
        a * b
    }
}
```

## Module Structure

### Complete Module Layout

```move
module my_address::token {
    // 1. Use statements
    use std::vector;
    use kanari_system::coin;
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    
    // 2. Constants
    const DECIMALS: u8 = 9;
    const MAX_SUPPLY: u64 = 1_000_000_000;
    
    // 3. Error codes
    const E_INVALID_AMOUNT: u64 = 0;
    const E_INSUFFICIENT_BALANCE: u64 = 1;
    
    // 4. Structs and enums
    struct TOKEN has drop {}
    
    struct Balance has key, store {
        id: UID,
        value: u64,
    }
    
    // 5. Public API functions
    public fun initialize(ctx: &mut TxContext) {
        // Initialize token
    }
    
    public entry fun transfer(
        coins: Coin<TOKEN>,
        recipient: address
    ) {
        transfer::public_transfer(coins, recipient);
    }
    
    // 6. Internal helper functions
    fun validate_amount(amount: u64) {
        assert!(amount > 0, E_INVALID_AMOUNT);
    }
    
    // 7. Tests
    #[test]
    fun test_initialization() {
        // Test code
    }
}
```

## Visibility Modifiers

### Public Functions

Accessible from anywhere:

```move
public fun public_function() {
    // Can be called by any module
}
```

### Public Entry Functions

Can be called directly from transactions:

```move
public entry fun transfer_tokens(
    coins: Coin<KANARI>,
    recipient: address
) {
    transfer::public_transfer(coins, recipient);
}
```

### Private Functions

Only accessible within the same module:

```move
fun private_helper() {
    // Only callable within this module
}

// Explicitly marked as private (same as above)
private fun another_private() {
    // Same visibility as 'fun'
}
```

## Module Dependencies

### Importing Other Modules

```move
module my_address::app {
    use std::vector;
    use kanari_system::coin::{self, Coin};
    use kanari_system::transfer;
    use my_address::token;
    
    public fun example() {
        let v = vector::empty<u64>();
        // Use imported modules
    }
}
```

### Selective Imports

```move
use kanari_system::{
    coin::{self, Coin},
    transfer,
    object::{self, UID},
};
```

## Module Organization Patterns

### Single Responsibility

Each module should have a clear, focused purpose:

```move
// token.move - Token management
module project::token {
    // Token creation, minting, burning
}

// nft.move - NFT management
module project::nft {
    // NFT collection, minting, metadata
}

// dex.move - Exchange functionality
module project::dex {
    // Swapping, liquidity pools
}
```

### Layered Architecture

```move
// Core business logic
module project::core {
    // Pure logic, no blockchain-specific code
}

// Blockchain integration
module project::blockchain {
    // Uses core, adds storage/transfer logic
}

// Public API
module project::api {
    // Entry points for users
}
```

## Common Module Patterns

### Factory Pattern

```move
module project::factory {
    use kanari_system::object::{UID, new};
    use kanari_system::tx_context::TxContext;
    
    struct Widget has key, store {
        id: UID,
        creator: address,
        value: u64,
    }
    
    public entry fun create_widget(value: u64, ctx: &mut TxContext) {
        let widget = Widget {
            id: new(ctx),
            creator: tx_context::sender(ctx),
            value,
        };
        
        transfer::public_transfer(widget, tx_context::sender(ctx));
    }
}
```

### Registry Pattern

```move
module project::registry {
    use std::vector;
    use kanari_system::object::{UID, new};
    use kanari_system::tx_context::TxContext;
    
    struct Registry has key, store {
        id: UID,
        addresses: vector<address>,
    }
    
    public entry fun register(registry: &mut Registry, addr: address) {
        if (!vector::contains(&registry.addresses, &addr)) {
            vector::push_back(&mut registry.addresses, addr);
        };
    }
    
    public fun is_registered(registry: &Registry, addr: address): bool {
        vector::contains(&registry.addresses, &addr)
    }
}
```

### Access Control Pattern

```move
module project::access_control {
    use kanari_system::object::{UID, new};
    use kanari_system::tx_context::TxContext;
    
    struct AdminCap has key, store {
        id: UID,
    }
    
    public entry fun create_admin_cap(ctx: &mut TxContext): AdminCap {
        AdminCap { id: new(ctx) }
    }
    
    public fun requires_admin(_cap: &AdminCap) {
        // Function that requires admin capability
    }
}
```

## Module Testing

### Unit Tests in Modules

```move
module my_address::calculator {
    public fun add(a: u64, b: u64): u64 {
        a + b
    }
    
    #[test]
    fun test_addition() {
        assert!(add(2, 3) == 5, 0);
        assert!(add(0, 0) == 0, 1);
        assert!(add(100, 200) == 300, 2);
    }
    
    #[test]
    fun test_edge_cases() {
        let max = 18446744073709551615; // u64::MAX
        // Test overflow handling
    }
}
```

### Test-Only Functions

```move
#[test_only]
public fun setup_test_environment(): TestEnv {
    // Helper function only available in tests
}

#[test]
fun test_with_setup() {
    let env = setup_test_environment();
    // Test logic
}
```

## Best Practices

### 1. Clear Naming

```move
// Good: Descriptive names
module project::liquidity_pool { }
module project::governance_token { }

// Bad: Unclear names
module project::lp { }
module project::gov { }
```

### 2. Minimal Public API

```move
// Expose only what's necessary
public entry fun user_function() { }

// Keep implementation details private
fun internal_helper() { }
```

### 3. Consistent Error Codes

```move
const E_INVALID_INPUT: u64 = 0;
const E_UNAUTHORIZED: u64 = 1;
const E_INSUFFICIENT_FUNDS: u64 = 2;

// Use consistently across module
```

### 4. Documentation

```move
/// Transfers tokens to recipient
/// 
/// # Arguments
/// * `coins` - Tokens to transfer
/// * `recipient` - Destination address
/// 
/// # Panics
/// * If recipient is zero address
public entry fun transfer(
    coins: Coin<TOKEN>,
    recipient: address
) {
    assert!(recipient != @0x0, E_INVALID_ADDRESS);
    transfer::public_transfer(coins, recipient);
}
```

### 5. Resource Management

```move
// Always handle resources properly
public fun process_resource(resource: MyResource) {
    // Transfer, delete, or return - don't leak
    transfer::public_transfer(resource, recipient);
}
```

## Module Deployment

### Development Workflow

```bash
# Build module
move build

# Run tests
move test

# Publish to network
move publish --profile devnet
```

### Version Management

```toml
# Move.toml
[package]
name = "MyModule"
version = "1.0.0"
published-at = "0x..."  # Set after deployment
```

## Common Pitfalls

### Circular Dependencies

```move
// Module A uses Module B
// Module B uses Module A
// This creates a circular dependency - avoid!

// Solution: Extract shared functionality to Module C
```

### Overly Large Modules

```move
// Bad: Module doing too much
module project::everything {
    // Token logic
    // NFT logic
    // DEX logic
    // Governance logic
}

// Good: Separated concerns
module project::token { }
module project::nft { }
module project::dex { }
module project::governance { }
```

### Missing Entry Points

```move
// Users can't call this directly
public fun internal_function() { }

// Add entry point for user interaction
public entry fun user_entry_point() {
    internal_function();
}
```

## Advanced Patterns

### Upgradeable Modules

```move
module project::proxy {
    use kanari_system::object::{UID, new};
    
    struct Implementation has key, store {
        id: UID,
        version: u64,
        code_hash: vector<u8>,
    }
    
    public entry fun upgrade_impl(
        impl: &mut Implementation,
        new_code_hash: vector<u8>
    ) {
        impl.version += 1;
        impl.code_hash = new_code_hash;
    }
}
```

### Module Composition

```move
module project::composed_feature {
    use project::feature_a;
    use project::feature_b;
    
    public entry fun combined_operation() {
        feature_a::do_something();
        feature_b::do_something_else();
    }
}
```

## Next Steps

- Learn about [Packages](packages.md) for managing multiple modules
- Study [Friends](friends.md) for controlled access between modules
- Explore [Standard Library](standard-library.md) for built-in modules
- Review [Coding Conventions](coding-conventions.md) for style guidelines
