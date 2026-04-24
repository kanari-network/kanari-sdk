# Abilities

Type abilities in Move control what operations can be performed on values. There are four abilities: `copy`, `drop`, `store`, and `key`.

## Overview

Abilities are annotations that specify how types can be used:

```move
// Struct with all abilities
struct Data has copy, drop, store, key {
    value: u64,
}

// Struct with specific abilities
struct Resource has key, store {
    id: UID,
}

// Simple data struct
#[derive(copy, drop)]
struct Metadata {
    name: vector<u8>,
}
```

## The Four Abilities

### Copy

Allows values to be copied:

```move
#[derive(copy, drop)]
struct Point {
    x: u64,
    y: u64,
}

let p1 = Point { x: 10, y: 20 };
let p2 = p1; // Copy - p1 still valid
let p3 = p1; // Can copy again

assert!(p1.x == 10, 0);
assert!(p2.x == 10, 1);
```

**Requirements:**

- All fields must also have `copy` ability
- Cannot have `UID` field (resources can't be copied)

**Common Uses:**

- Configuration data
- Metadata
- Simple value types

### Drop

Allows values to be discarded:

```move
#[derive(drop)]
struct Temporary {
    message: vector<u8>,
}

let temp = Temporary { message: b"hello" };
// temp is automatically dropped at end of scope
```

**Requirements:**

- All fields must have `drop` ability OR be explicitly handled

**Common Uses:**

- Temporary computation results
- Event data
- Intermediate calculations

### Store

Allows values to be stored inside resources:

```move
struct Token has store {
    amount: u64,
}

struct Wallet has key, store {
    id: UID,
    balance: Token, // Token must have 'store'
}
```

**Requirements:**

- All fields must have `store` ability
- Cannot contain types with only `key` ability

**Common Uses:**

- Fields inside resources
- Nested data structures
- Serializable data

### Key

Allows values to be stored in global storage (makes it a Resource):

```move
use kanari_system::object::{UID, new};

struct NFT has key, store {
    id: UID,        // Must have UID field
    owner: address,
    uri: vector<u8>,
}

public fun create_nft(uri: vector<u8>, ctx: &mut TxContext): NFT {
    NFT {
        id: new(ctx),
        owner: tx_context::sender(ctx),
        uri,
    }
}
```

**Requirements:**

- Must have exactly one `UID` field
- All other fields must have `store` ability
- Cannot have `copy` or `drop` abilities directly

**Common Uses:**

- Digital assets
- Accounts
- Persistent state

## Ability Rules

### Inheritance Rules

Fields constrain the parent struct's abilities:

```move
// If field doesn't have 'copy', struct can't have 'copy'
struct NoCopyField {
    resource: UID, // UID doesn't have copy
}
// struct Container has copy {  // Error!
//     data: NoCopyField,
// }

// If field doesn't have 'drop', struct can't have 'drop'
struct NoDrop {
    value: u64,
}
// This would require explicit handling
```

### Ability Combinations

Different combinations serve different purposes:

```move
// Pure data - can be copied and dropped
#[derive(copy, drop)]
struct Config {
    max_supply: u64,
    decimals: u8,
}

// Storable data - can be inside resources
struct Balance has store {
    amount: u64,
}

// Resource - lives in global storage
struct Account has key, store {
    id: UID,
    balance: Balance,
}

// Temporary result
#[derive(drop)]
struct CalculationResult {
    value: u64,
    timestamp: u64,
}
```

## Common Patterns

### Witness Pattern

Use `drop` to consume witness:

```move
struct TOKEN has drop {}

public fun initialize(_witness: TOKEN) {
    // Witness is consumed (dropped)
    // Proves caller has authority
}

// Usage
initialize(TOKEN {});
```

### Capability Pattern

Resources as capabilities:

```move
struct AdminCap has key, store {
    id: UID,
}

public fun admin_only(_cap: &AdminCap) {
    // Requires holding capability
}

// Transfer capability
transfer::public_transfer(cap, new_admin);
```

### Phantom Type with Abilities

```move
struct Coin<phantom T: drop> has key, store {
    id: UID,
    value: u64,
}

struct KANARI has drop {}
struct USD has drop {}

let kanari = Coin<KANARI> { id: new(ctx), value: 100 };
let usd = Coin<USD> { id: new(ctx), value: 50 };
```

## Checking Abilities

### Compile-Time Checks

Move enforces abilities at compile time:

```move
struct NoCopy {
    value: u64,
}

public fun test() {
    let x = NoCopy { value: 10 };
    // let y = x; // Error: cannot copy
    let y = x; // Move - x is no longer valid
}
```

### Runtime Behavior

Abilities affect runtime behavior:

```move
// With 'drop': automatically cleaned up
#[derive(drop)]
struct Temp { value: u64 }
let t = Temp { value: 10 };
// t dropped here

// Without 'drop': must be explicitly handled
struct NoDrop { value: u64 }
let nd = NoDrop { value: 10 };
// Must use nd or compiler error
```

## Best Practices

### 1. Minimal Abilities

Only grant necessary abilities:

```move
// Bad: Too many abilities
struct Data has copy, drop, store, key {
    value: u64,
}

// Good: Only what's needed
#[derive(copy, drop, store)]
struct Config {
    value: u64,
}

struct Asset has key, store {
    id: UID,
    config: Config,
}
```

### 2. Document Ability Choices

```move
/// Configuration data - safe to copy and share
#[derive(copy, drop, store)]
struct TokenConfig {
    decimals: u8,
    symbol: vector<u8>,
}

/// Unique asset - cannot be copied
struct NFT has key, store {
    id: UID,
    config: TokenConfig,
}
```

### 3. Use Appropriate Patterns

```move
// For simple data: copy + drop
#[derive(copy, drop)]
struct Point { x: u64, y: u64 }

// For storable data: store
struct Record has store {
    timestamp: u64,
    value: u64,
}

// For unique assets: key + store
struct Certificate has key, store {
    id: UID,
    record: Record,
}
```

### 4. Handle Resources Carefully

```move
// Always transfer or delete resources
public fun cleanup(resource: MyResource) {
    // Option 1: Transfer
    transfer::public_transfer(resource, recipient);
    
    // Option 2: Delete (if has delete function)
    // object::delete(resource.id);
}
```

## Common Errors

### Missing Copy Ability

```move
struct NoCopy { value: u64 }

let x = NoCopy { value: 10 };
// let y = x; // Error: 'NoCopy' does not have 'copy'
let y = x; // Move semantics
```

### Missing Drop Ability

```move
struct NoDrop { value: u64 }

public fn leak() {
    let x = NoDrop { value: 10 };
    // Error: 'NoDrop' does not have 'drop' and wasn't used
}

public fn no_leak() {
    let x = NoDrop { value: 10 };
    use_value(x.value); // Properly used
}
```

### Invalid Key Struct

```move
// Wrong: Missing UID
// struct BadKey has key, store {
//     value: u64,
// }

// Correct: Has UID
struct GoodKey has key, store {
    id: UID,
    value: u64,
}
```

### Store Inside Non-Storable

```move
struct NotStorable {
    value: u64,
}

// Wrong: Can't store non-storable type
// struct Container has key, store {
//     id: UID,
//     data: NotStorable, // Error!
// }

// Correct: Make field storable
struct StorableData has store {
    value: u64,
}

struct Container has key, store {
    id: UID,
    data: StorableData, // OK
}
```

## Testing Abilities

```move
#[test]
fun test_copy_ability() {
    #[derive(copy, drop)]
    struct Copyable { value: u64 }
    
    let c1 = Copyable { value: 10 };
    let c2 = c1;
    let c3 = c1;
    
    assert!(c1.value == 10, 0);
    assert!(c2.value == 10, 1);
}

#[test]
fun test_drop_ability() {
    #[derive(drop)]
    struct Droppable { value: u64 }
    
    let d = Droppable { value: 10 };
    // d is automatically dropped
}

#[test]
fun test_resource_creation(ctx: &mut TxContext) {
    struct MyResource has key, store {
        id: UID,
        value: u64,
    }
    
    let r = MyResource {
        id: object::new(ctx),
        value: 100,
    };
    
    assert!(r.value == 100, 0);
    // Must handle resource - can't just drop
    transfer::public_transfer(r, @0x1);
}
```

## Performance Considerations

- `copy` ability may incur copying overhead for large structs
- `drop` ability enables automatic cleanup
- `store` ability required for serialization
- `key` ability adds storage overhead

Choose abilities based on actual needs, not convenience.

## Next Steps

- Learn about [Structs](structs-and-resources.md)
- Study [Global Storage](global-storage-structure.md)
- Explore [Resource Patterns](usage-examples.md)
