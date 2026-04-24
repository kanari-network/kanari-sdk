# Global Storage Structure

Move's global storage is a key-value store where resources (structs with `key` ability) are stored. Understanding this structure is crucial for building blockchain applications.

## Storage Model

### Key-Value Store

```
Global Storage:
├── Address 0x1
│   ├── Object ID 0xabc... → Resource A
│   └── Object ID 0xdef... → Resource B
├── Address 0x2
│   └── Object ID 0x123... → Resource C
└── Address 0x3
    └── Object ID 0x456... → Resource D
```

### Resources

Only structs with `key` ability can be stored:

```move
use kanari_system::object::{UID, new};

struct Account has key, store {
    id: UID,           // Must have UID field
    owner: address,
    balance: u64,
}

public fun create_account(ctx: &mut TxContext): Account {
    Account {
        id: new(ctx),
        owner: tx_context::sender(ctx),
        balance: 0,
    }
}
```

## Storage Operations

### Store Resource

```move
use kanari_system::transfer;

public entry fun save_account(account: Account) {
    // Transfer to sender's address (stores it)
    transfer::public_transfer(account, tx_context::sender(ctx));
}
```

### Retrieve Resource

```move
use kanari_system::transfer;

public entry fun withdraw_from_account(
    account: Account,
    amount: u64,
    ctx: &mut TxContext
) {
    let sender = tx_context::sender(ctx);
    
    // Verify ownership
    assert!(account.owner == sender, E_UNAUTHORIZED);
    assert!(account.balance >= amount, E_INSUFFICIENT_BALANCE);
    
    account.balance -= amount;
    
    // Save back to storage
    transfer::public_transfer(account, sender);
}
```

### Delete Resource

```move
use kanari_system::object;

public fun close_account(account: Account) {
    assert!(account.balance == 0, E_NONZERO_BALANCE);
    
    // Extract and delete UID
    let Account { id, owner: _, balance: _ } = account;
    object::delete(id);
}
```

## Object IDs

### Unique Identifiers

Every resource has a unique ID:

```move
use kanari_system::object;

public fun get_object_id(resource: &Account): address {
    object::id_to_address(&resource.id)
}

public fun get_uid(resource: &Account): &UID {
    &resource.id
}
```

### Object Metadata

```move
use kanari_system::object;

public fun get_owner(resource: &Account): address {
    object::owner(&resource.id)
}

public fun is_frozen(resource: &Account): bool {
    object::is_frozen(&resource.id)
}
```

## Common Patterns

### Single Owner Pattern

```move
struct NFT has key, store {
    id: UID,
    owner: address,
    uri: vector<u8>,
}

public fun mint_nft(uri: vector<u8>, ctx: &mut TxContext): NFT {
    NFT {
        id: new(ctx),
        owner: tx_context::sender(ctx),
        uri,
    }
}

public fun transfer_nft(nft: NFT, new_owner: address) {
    transfer::public_transfer(nft, new_owner);
}
```

### Shared State Pattern

```move
struct GlobalConfig has key, store {
    id: UID,
    admin: address,
    max_supply: u64,
    is_active: bool,
}

public fun create_config(ctx: &mut TxContext): GlobalConfig {
    GlobalConfig {
        id: new(ctx),
        admin: tx_context::sender(ctx),
        max_supply: 1_000_000,
        is_active: true,
    }
}

public fun update_config(
    config: &mut GlobalConfig,
    new_max: u64,
    ctx: &mut TxContext
) {
    let sender = tx_context::sender(ctx);
    assert!(sender == config.admin, E_UNAUTHORIZED);
    
    config.max_supply = new_max;
}
```

### Collection Pattern

```move
use kanari_system::table::{self, Table};

struct Collection has key, store {
    id: UID,
    items: Table<u64, Item>,
    next_id: u64,
}

struct Item has copy, drop, store {
    name: vector<u8>,
    value: u64,
}

public fun create_collection(ctx: &mut TxContext): Collection {
    Collection {
        id: new(ctx),
        items: table::new<u64, Item>(ctx),
        next_id: 0,
    }
}

public fun add_item(collection: &mut Collection, item: Item) {
    let id = collection.next_id;
    table::add(&mut collection.items, id, item);
    collection.next_id = id + 1;
}
```

## Storage Best Practices

### 1. Minimize Storage Writes

```move
// Bad: Multiple separate writes
public fn bad_pattern() {
    update_field1();
    update_field2();
    update_field3();
}

// Good: Batch updates
public fn good_pattern() {
    update_all_fields();
}
```

### 2. Clean Up Unused Resources

```move
public fun cleanup_empty_table(table: Table<u64, address>) {
    assert!(table::length(&table) == 0, E_NOT_EMPTY);
    table::destroy_empty(table);
}
```

### 3. Use Appropriate Data Structures

```move
// Small collections: vector
let small_list = vector[1u64, 2, 3];

// Large collections: Table
let mut large_map = table::new<u64, address>(ctx);

// Heterogeneous data: Bag
let mut bag = bag::new(ctx);
```

### 4. Validate Before Storage

```move
public fun save_validated_data(data: ValidatedData, ctx: &mut TxContext) {
    // Validate first
    assert!(data.is_valid(), E_INVALID_DATA);
    assert!(data.size() <= MAX_SIZE, E_TOO_LARGE);
    
    // Then store
    transfer::public_transfer(data, tx_context::sender(ctx));
}
```

## Storage Costs

### Gas Considerations

```move
// Expensive: Large struct
struct LargeResource has key, store {
    id: UID,
    data: vector<u8>,  // Large vectors cost more
}

// Cheaper: Compact struct
struct CompactResource has key, store {
    id: UID,
    hash: vector<u8>,  // Store hash instead of full data
}
```

### Optimization Tips

```move
// Store references instead of copies
struct Reference has key, store {
    id: UID,
    target_id: address,  // Reference to another object
}

// Use events for historical data
public fun record_action(action: vector<u8>) {
    event::emit(ActionEvent { action });
    // Don't store in global state
}
```

## Security Considerations

### Access Control

```move
struct ProtectedResource has key, store {
    id: UID,
    owner: address,
    data: u64,
}

public fun update_protected(
    resource: &mut ProtectedResource,
    new_value: u64,
    ctx: &mut TxContext
) {
    let sender = tx_context::sender(ctx);
    assert!(sender == resource.owner, E_UNAUTHORIZED);
    
    resource.data = new_value;
}
```

### Reentrancy Protection

```move
struct SecureAccount has key, store {
    id: UID,
    owner: address,
    balance: u64,
    locked: bool,
}

public fun withdraw(
    account: &mut SecureAccount,
    amount: u64,
    ctx: &mut TxContext
) {
    assert!(!account.locked, E_REENTRANCY);
    account.locked = true;
    
    account.balance -= amount;
    
    account.locked = false;
}
```

### Freeze Objects

```move
public fun freeze_immutable(resource: MyResource) {
    // Make object immutable
    transfer::public_freeze_object(resource);
}

// Frozen objects cannot be modified or moved
```

## Testing Storage

```move
#[test]
fun test_resource_storage() {
    let ctx = &mut tx_context::dummy();
    let sender = tx_context::sender(ctx);
    
    // Create resource
    let account = create_account(ctx);
    assert!(account.owner == sender, 0);
    
    // Simulate storage by transferring
    transfer::public_transfer(account, sender);
    
    // In real scenario, would retrieve from storage
}

#[test]
fun test_resource_deletion() {
    let ctx = &mut tx_context::dummy();
    let account = create_empty_account(ctx);
    
    // Delete resource
    close_account(account);
    
    // Resource should be deleted
}
```

## Next Steps

- Learn about [Storage Operators](global-storage-operators.md)
- Study [Object Management](usage-examples.md#object-management)
- Explore [Storage Patterns](../patterns/storage-patterns.md)
