# Global Storage Operators

Global storage operators in Move allow you to interact with resources stored on the blockchain. This guide covers all storage operations and their usage patterns.

## Core Operators

### move_to

Store a resource at an address:

```move
use kanari_system::transfer;

public entry fun store_resource(resource: MyResource, ctx: &mut TxContext) {
    // Store resource at sender's address
    transfer::public_transfer(resource, tx_context::sender(ctx));
}
```

### move_from

Retrieve a resource from an address:

```move
public entry fun retrieve_resource(
    resource: MyResource,
    ctx: &mut TxContext
) {
    let sender = tx_context::sender(ctx);
    
    // Verify ownership
    assert!(object::owner(&resource.id) == sender, E_UNAUTHORIZED);
    
    // Use resource (it's moved from storage)
    process_resource(resource);
}
```

### borrow_global

Borrow a resource immutably:

```move
use kanari_system::object;

public fun read_resource(resource_addr: address): &MyResource {
    // Borrow from global storage (read-only)
    borrow_global<MyResource>(resource_addr)
}

// Usage
let resource = read_resource(@0x123);
let value = resource.some_field;
```

### borrow_global_mut

Borrow a resource mutably:

```move
public fun update_resource(
    resource_addr: address,
    new_value: u64,
    ctx: &mut TxContext
) {
    let sender = tx_context::sender(ctx);
    
    // Mutable borrow
    let resource = borrow_global_mut<MyResource>(resource_addr);
    
    // Verify ownership
    assert!(object::owner(&resource.id) == sender, E_UNAUTHORIZED);
    
    // Update
    resource.value = new_value;
}
```

### exists

Check if a resource exists at an address:

```move
public fun has_resource(addr: address): bool {
    exists<MyResource>(addr)
}

public fun get_or_create(
    addr: address,
    ctx: &mut TxContext
): &mut MyResource {
    if (!exists<MyResource>(addr)) {
        // Create new resource
        let resource = create_default_resource(ctx);
        transfer::public_transfer(resource, addr);
    };
    
    borrow_global_mut<MyResource>(addr)
}
```

## Advanced Operators

### exist_with_type

Check for specific resource type:

```move
public fun has_token(addr: address): bool {
    exists<Coin<TOKEN>>(addr)
}

public fun has_nft(addr: address): bool {
    exists<NFT>(addr)
}
```

### borrow_child_object

Borrow nested objects:

```move
struct Parent has key, store {
    id: UID,
}

struct Child has key, store {
    id: UID,
    parent_id: address,
    value: u64,
}

public fun get_child_value(parent: &Parent, child_id: address): u64 {
    let child = borrow_child_object<Child>(parent, child_id);
    child.value
}
```

### borrow_child_object_mut

Mutable borrow of nested objects:

```move
public fun update_child_value(
    parent: &mut Parent,
    child_id: address,
    new_value: u64
) {
    let child = borrow_child_object_mut<Child>(parent, child_id);
    child.value = new_value;
}
```

## Common Patterns

### Singleton Pattern

```move
struct Singleton has key, store {
    id: UID,
    value: u64,
}

const SINGLETON_ADDR: address = @0x1;

public fun get_singleton(): &Singleton {
    assert!(exists<Singleton>(SINGLETON_ADDR), E_NOT_INITIALIZED);
    borrow_global<Singleton>(SINGLETON_ADDR)
}

public fun update_singleton(value: u64, ctx: &mut TxContext) {
    let sender = tx_context::sender(ctx);
    assert!(sender == @0x1, E_UNAUTHORIZED); // Only admin
    
    let singleton = borrow_global_mut<Singleton>(SINGLETON_ADDR);
    singleton.value = value;
}

public fun initialize_singleton(ctx: &mut TxContext) {
    assert!(!exists<Singleton>(SINGLETON_ADDR), E_ALREADY_EXISTS);
    
    let singleton = Singleton {
        id: object::new(ctx),
        value: 0,
    };
    
    transfer::public_transfer(singleton, SINGLETON_ADDR);
}
```

### Registry Pattern

```move
struct Registry has key, store {
    id: UID,
    addresses: vector<address>,
}

public fun register_address(registry: &mut Registry, addr: address) {
    assert!(!vector::contains(&registry.addresses, &addr), E_ALREADY_REGISTERED);
    vector::push_back(&mut registry.addresses, addr);
}

public fun is_registered(registry: &Registry, addr: address): bool {
    vector::contains(&registry.addresses, &addr)
}
```

### Factory Pattern

```move
struct Factory has key, store {
    id: UID,
    created_count: u64,
}

public fun create_item(factory: &mut Factory, ctx: &mut TxContext): Item {
    let item = Item {
        id: object::new(ctx),
        creator: tx_context::sender(ctx),
        created_at: clock::timestamp_ms(),
    };
    
    factory.created_count += 1;
    
    item
}
```

## Error Handling

### Check Before Access

```move
public fun safe_borrow(addr: address): Option<&MyResource> {
    if (exists<MyResource>(addr)) {
        option::some(borrow_global<MyResource>(addr))
    } else {
        option::none()
    }
}

public fun safe_update(addr: address, value: u64): bool {
    if (!exists<MyResource>(addr)) {
        return false;
    };
    
    let resource = borrow_global_mut<MyResource>(addr);
    resource.value = value;
    true
}
```

### Assertion Pattern

```move
public fun must_exist(addr: address): &MyResource {
    assert!(exists<MyResource>(addr), E_RESOURCE_NOT_FOUND);
    borrow_global<MyResource>(addr)
}

public fun must_own(addr: address, ctx: &mut TxContext): &mut MyResource {
    let sender = tx_context::sender(ctx);
    let resource = borrow_global_mut<MyResource>(addr);
    
    assert!(object::owner(&resource.id) == sender, E_UNAUTHORIZED);
    resource
}
```

## Performance Optimization

### Batch Operations

```move
public fun batch_update(
    addresses: vector<address>,
    value: u64
) {
    let len = vector::length(&addresses);
    let mut i = 0;
    
    while (i < len) {
        let addr = *vector::borrow(&addresses, i);
        
        if (exists<MyResource>(addr)) {
            let resource = borrow_global_mut<MyResource>(addr);
            resource.value = value;
        };
        
        i = i + 1;
    };
}
```

### Cache Frequently Accessed Data

```move
struct Cache has key, store {
    id: UID,
    last_updated: u64,
    cached_value: u64,
}

public fun get_cached_or_compute(addr: address): u64 {
    if (exists<Cache>(addr)) {
        let cache = borrow_global<Cache>(addr);
        
        // Check if cache is fresh
        if (clock::timestamp_ms() - cache.last_updated < CACHE_TTL) {
            return cache.cached_value;
        };
    };
    
    // Compute and cache
    let value = expensive_computation(addr);
    update_cache(addr, value);
    value
}
```

## Security Best Practices

### Ownership Verification

```move
public fun secure_update(
    resource_addr: address,
    new_value: u64,
    ctx: &mut TxContext
) {
    let sender = tx_context::sender(ctx);
    let resource = borrow_global_mut<MyResource>(resource_addr);
    
    // Always verify ownership
    assert!(object::owner(&resource.id) == sender, E_UNAUTHORIZED);
    
    resource.value = new_value;
}
```

### Access Control Lists

```move
struct ACL has key, store {
    id: UID,
    authorized: vector<address>,
}

public fun check_access(acl: &ACL, addr: address): bool {
    vector::contains(&acl.authorized, &addr)
}

public fun require_access(acl: &ACL, addr: address) {
    assert!(check_access(acl, addr), E_ACCESS_DENIED);
}
```

### Reentrancy Guards

```move
struct GuardedResource has key, store {
    id: UID,
    locked: bool,
    value: u64,
}

public fun guarded_operation(resource: &mut GuardedResource) {
    assert!(!resource.locked, E_REENTRANCY);
    resource.locked = true;
    
    // Perform operation
    resource.value += 1;
    
    resource.locked = false;
}
```

## Testing Storage Operators

```move
#[test]
fun test_exists_operator() {
    let ctx = &mut tx_context::dummy();
    let addr = tx_context::sender(ctx);
    
    // Initially doesn't exist
    assert!(!exists<MyResource>(addr), 0);
    
    // Create resource
    let resource = create_resource(ctx);
    transfer::public_transfer(resource, addr);
    
    // Now exists
    assert!(exists<MyResource>(addr), 1);
}

#[test]
fun test_borrow_operators() {
    let ctx = &mut tx_context::dummy();
    let addr = tx_context::sender(ctx);
    
    let resource = create_resource_with_value(100, ctx);
    transfer::public_transfer(resource, addr);
    
    // Immutable borrow
    let borrowed = borrow_global<MyResource>(addr);
    assert!(borrowed.value == 100, 0);
    
    // Mutable borrow would require proper setup
}

#[test]
#[expected_failure(abort_code = E_RESOURCE_NOT_FOUND)]
fun test_borrow_nonexistent() {
    borrow_global<MyResource>(@0x999);
}
```

## Common Errors

### Resource Not Found

```move
// Wrong: Borrowing non-existent resource
// borrow_global<MyResource>(@0x999); // Panics!

// Correct: Check first
if (exists<MyResource>(addr)) {
    let resource = borrow_global<MyResource>(addr);
    // Use resource
};
```

### Ownership Violation

```move
// Wrong: Modifying without ownership
// let resource = borrow_global_mut<MyResource>(addr);
// resource.value = 100; // May fail if not owner

// Correct: Verify ownership
let sender = tx_context::sender(ctx);
let resource = borrow_global_mut<MyResource>(addr);
assert!(object::owner(&resource.id) == sender, E_UNAUTHORIZED);
resource.value = 100;
```

### Double Storage

```move
// Wrong: Storing same resource twice
// transfer::public_transfer(resource, addr1);
// transfer::public_transfer(resource, addr2); // Error: resource moved

// Correct: Create separate instances
let resource1 = create_resource(ctx);
let resource2 = create_resource(ctx);
transfer::public_transfer(resource1, addr1);
transfer::public_transfer(resource2, addr2);
```

## Next Steps

- Learn about [Storage Structure](global-storage-structure.md)
- Study [Object Lifecycle](usage-examples.md#object-management)
- Explore [Storage Security](../security/storage-security.md)
