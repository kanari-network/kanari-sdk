# Kanari

The `kanari` module refers to the core functionality of the Kanari blockchain ecosystem. This section covers the Kanari-specific extensions, utilities, and best practices for developing on the Kanari platform.

## Overview

Kanari is a high-performance blockchain designed for DeFi applications. The Kanari system provides specialized modules and utilities beyond the standard Move language features.

## Kanari-Specific Modules

### Coin and Token Support

Kanari extends standard coin functionality with additional features:

```move
use kanari_system::coin;
use kanari_system::transfer;
use kanari_system::tx_context::TxContext;

struct KANARI has drop {}

// Create KANARI-based tokens
public fun create_token(ctx: &mut TxContext) {
    let (cap, metadata) = coin::create_currency<KANARI>(
        KANARI {},
        9,                              // decimals
        b"KANARI",                        // symbol
        b"Kanari",                      // name
        b"Kanari Network Token",        // description
        option::none(),                 // icon_url
        ctx
    );
    
    // Transfer mint/burn cap to sender
    transfer::public_transfer(cap, tx_context::sender(ctx));
    transfer::public_freeze_object(metadata);
}
```

### Object Management

Kanari provides enhanced object management:

```move
use kanari_system::object::{UID, new};
use kanari_system::tx_context::TxContext;

struct MyAsset has key, store {
    id: UID,
    owner: address,
    value: u64,
}

public fun create_asset(ctx: &mut TxContext): MyAsset {
    MyAsset {
        id: new(ctx),
        owner: tx_context::sender(ctx),
        value: 0,
    }
}

public fun transfer_asset(asset: MyAsset, recipient: address) {
    transfer::public_transfer(asset, recipient);
}
```

## Kanari-Specific Features

### Transaction Context

Kanari's transaction context provides additional information:

```move
use kanari_system::tx_context::TxContext;

public fun get_tx_info(ctx: &TxContext): (address, u64, u64) {
    let sender = tx_context::sender(ctx);
    let epoch = tx_context::epoch(ctx);
    let timestamp = tx_context::timestamp_ms(ctx);
    
    (sender, epoch, timestamp)
}
```

### Clock Module

Kanari provides time-based utilities:

```move
use kanari_system::clock;

public fun is_within_window(start: u64, end: u64): bool {
    let now = clock::timestamp_ms();
    now >= start && now <= end
}

public fun seconds_since(timestamp: u64): u64 {
    let now = clock::timestamp_ms();
    (now - timestamp) / 1000
}
```

### Enhanced Math Operations

```move
use kanari_system::math;

public fun calculate_percentage(total: u64, percent: u64): u64 {
    math::percentage(total, percent)
}

public fun safe_add(a: u64, b: u64): u64 {
    math::checked_add_u64(a, b)
}

public fun safe_multiply(a: u64, b: u64): u64 {
    math::checked_mul_u64(a, b)
}
```

## Best Practices for Kanari Development

### Gas Optimization

```move
// Use efficient data structures
use kanari_system::table::{self, Table};
use kanari_system::bag::{self, Bag};

// For homogeneous key-value: Table
struct UserMap has key, store {
    id: UID,
    data: Table<address, UserData>,
}

// For heterogeneous data: Bag
struct FlexibleContainer has key, store {
    id: UID,
    data: Bag,
}
```

### Security Patterns

```move
// Use capability pattern for access control
struct AdminCap has key, store {
    id: UID,
}

public fun admin_operation(cap: &AdminCap) {
    // Requires admin capability
}

// Implement reentrancy protection
struct SecureContract has key, store {
    id: UID,
    locked: bool,
}

public fun protected_operation(contract: &mut SecureContract) {
    assert!(!contract.locked, E_REENTRANCY);
    contract.locked = true;
    
    // Perform operation
    contract.locked = false;
}
```

### Error Handling

```move
// Use descriptive error codes
const E_INVALID_AMOUNT: u64 = 0;
const E_INSUFFICIENT_BALANCE: u64 = 1;
const E_UNAUTHORIZED: u64 = 2;
const E_INVALID_STATE: u64 = 3;

public fun validated_operation(amount: u64, balance: u64) {
    assert!(amount > 0, E_INVALID_AMOUNT);
    assert!(balance >= amount, E_INSUFFICIENT_BALANCE);
    
    // Operation logic
}
```

## Kanari Ecosystem Integration

### Interactions with Standard Libraries

```move
use std::vector;
use kanari_system::coin;
use kanari_system::transfer;

// Combine standard and Kanari modules
public fun batch_transfer(
    coins: vector<Coin<KANARI>>,
    recipients: vector<address>
) {
    let len = vector::length(&coins);
    assert!(len == vector::length(&recipients), E_LENGTH_MISMATCH);
    
    let mut i = 0;
    while (i < len) {
        let coin = vector::pop_back(&mut coins);
        let recipient = *vector::borrow(&recipients, i);
        transfer::public_transfer(coin, recipient);
        i = i + 1;
    };
}
```

### Event Emission

```move
use kanari_system::event;

struct TransferEvent has copy, drop {
    from: address,
    to: address,
    amount: u64,
}

public fun emit_transfer_event(from: address, to: address, amount: u64) {
    event::emit(TransferEvent { from, to, amount });
}
```

## Migration Considerations

### From Standard Move to Kanari

When migrating from other Move implementations:

```move
// Old way (other chains)
// use 0x1::account::create_resource_account;

// Kanari way
use kanari_system::object::{new};
use kanari_system::tx_context::TxContext;

public fun create_resource(ctx: &mut TxContext) {
    // Use Kanari's object system
    let id = new(ctx);
    // ... rest of implementation
}
```

## Testing on Kanari

### Unit Tests

```move
#[test]
fun test_kanari_features() {
    let ctx = &mut tx_context::dummy();
    let sender = tx_context::sender(ctx);
    
    // Test Kanari-specific functionality
    let asset = create_asset(ctx);
    assert!(asset.owner == sender, 0);
    
    // Clean up resources
    transfer::public_transfer(asset, sender);
}
```

### Integration Tests

```move
// Test interactions between multiple Kanari modules
#[test]
fun test_multi_module_interaction() {
    let ctx = &mut tx_context::dummy();
    
    // Create coin
    let (cap, _) = coin::create_currency<KANARI>(
        KANARI {}, 9, b"TEST", b"Test", b"", option::none(), ctx
    );
    
    // Mint tokens
    let coins = coin::mint(&mut cap, 1000, ctx);
    
    // Transfer using Kanari transfer
    transfer::public_transfer(coins, @0x1);
}
```

## Performance Considerations

### Efficient Storage Patterns

```move
// Use tables for large mappings
struct EfficientStorage has key, store {
    id: UID,
    // Use table instead of large vector for key-value mapping
    user_data: Table<address, UserData>,
}

// Batch operations when possible
public entry fun batch_process(items: vector<ProcessItem>) {
    let len = vector::length(&items);
    let mut i = 0;
    while (i < len) {
        process_single(vector::borrow(&items, i));
        i = i + 1;
    };
}
```

### Gas Cost Awareness

- Object creation: Higher gas cost than simple storage
- Table operations: Efficient for large datasets
- Cross-object references: Enable complex data structures
- Event emissions: Relatively low cost, good for logging

## Common Pitfalls and Solutions

### Avoiding Common Mistakes

```move
// Wrong: Not handling resource requirements
// transfer::public_transfer(resource, recipient); // May fail if recipient can't accept

// Right: Ensure recipient can accept resource
public fun safe_transfer(resource: MyResource, recipient: address) {
    // Check if recipient can accept, or use alternative approach
    transfer::public_transfer(resource, recipient);
}
```

### Resource Management

```move
// Always handle resources properly
public fun process_resource(resource: MyResource) {
    // Either transfer, delete, or return the resource
    transfer::public_transfer(resource, @0x1);
    // Or: object::delete(resource.id);
    // Or: return resource;
}
```

## Next Steps

- Explore [Usage Examples](usage-examples.md) for comprehensive patterns
- Learn about [Kanari System Modules](kanari-system/overview.md)
- Study [DeFi Patterns](defi-staking-tutorial.md) for advanced usage
- Review [Security Best Practices](coding-conventions.md)
- Check out the [NFT Tutorial](nft-tutorial.md) for digital asset creation
