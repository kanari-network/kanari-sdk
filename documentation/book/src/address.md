# Address

Addresses in Move represent accounts and object identifiers on the Kanari blockchain. They are 32-byte (256-bit) values typically displayed as hexadecimal strings.

## Address Literals

```move
// Hexadecimal address
let addr1: address = @0x1;
let addr2: address = @0x1234abcd;
let addr3: address = @0x0000000000000000000000000000000000000000000000000000000000000001;

// Special addresses
let zero_address: address = @0x0;
let system_address: address = @0x0; // Often used for system operations
```

## Address Operations

### Comparison

```move
let addr1: address = @0x100;
let addr2: address = @0x200;

// Equality check
if (addr1 == addr2) {
    // Same address
};

// Inequality
if (addr1 != @0x0) {
    // Not zero address
};
```

### Conversion

```move
use std::vector;

// Address to bytes (32 bytes)
let addr: address = @0x123;
let addr_bytes: vector<u8> = *(&addr as &vector<u8>);

// Note: Direct conversion requires unsafe operations
// Use helper functions from standard library when available
```

## Common Patterns

### Sender Address

```move
use kanari_system::tx_context::TxContext;

public fun get_sender(ctx: &TxContext): address {
    tx_context::sender(ctx)
}

public fun transfer_to_sender(coins: Coin, ctx: &mut TxContext) {
    let recipient = tx_context::sender(ctx);
    transfer::public_transfer(coins, recipient);
}
```

### Address Validation

```move
/// Check if address is valid (not zero address)
public fun is_valid_address(addr: address): bool {
    addr != @0x0
}

/// Check if address is system account
public fun is_system_account(addr: address): bool {
    addr == @0x0
}

/// Validate recipient before transfer
public fun validate_recipient(recipient: address, sender: address) {
    assert!(recipient != @0x0, E_INVALID_RECIPIENT);
    assert!(recipient != sender, E_CANNOT_SEND_TO_SELF);
}
```

### Access Control

```move
struct AdminCap has key, store {
    id: UID,
    admin_address: address,
}

public fun only_admin(cap: &AdminCap, caller: address) {
    assert!(caller == cap.admin_address, E_NOT_ADMIN);
}

public fun update_settings(
    cap: &AdminCap,
    new_value: u64,
    ctx: &mut TxContext
) {
    only_admin(cap, tx_context::sender(ctx));
    // Update settings
}
```

## Object Addresses

Objects in Move have unique addresses derived from transaction context:

```move
use kanari_system::object::{UID, new};
use kanari_system::tx_context::TxContext;

public fun create_object(ctx: &mut TxContext): UID {
    let uid = new(ctx);
    // Object gets unique address based on tx_hash and counter
    uid
}

/// Get object's address
public fun object_address(uid: &UID): address {
    object::id_to_address(uid)
}
```

## Multi-Signature Addresses

While Move doesn't natively support multi-sig addresses, you can implement the pattern:

```move
struct MultiSigWallet has key, store {
    id: UID,
    owners: vector<address>,
    threshold: u64,
}

public fun is_owner(wallet: &MultiSigWallet, addr: address): bool {
    let len = vector::length(&wallet.owners);
    let mut i = 0;
    let mut found = false;
    
    while (i < len) {
        if (*vector::borrow(&wallet.owners, i) == addr) {
            found = true;
        };
        i = i + 1;
    };
    
    found
}
```

## Address in Events

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

## Common Use Cases

### Whitelist

```move
struct Whitelist has key, store {
    id: UID,
    allowed_addresses: vector<address>,
}

public fun add_to_whitelist(list: &mut Whitelist, addr: address) {
    assert!(!contains(list, addr), E_ALREADY_WHITELISTED);
    vector::push_back(&mut list.allowed_addresses, addr);
}

public fun is_whitelisted(list: &Whitelist, addr: address): bool {
    contains(list, addr)
}

fun contains(list: &Whitelist, addr: address): bool {
    let len = vector::length(&list.allowed_addresses);
    let mut i = 0;
    
    while (i < len) {
        if (*vector::borrow(&list.allowed_addresses, i) == addr) {
            return true;
        };
        i = i + 1;
    };
    
    false
}
```

### Blacklist

```move
struct Blacklist has key, store {
    id: UID,
    blocked_addresses: vector<address>,
}

public fun is_blocked(list: &Blacklist, addr: address): bool {
    // Similar to whitelist but checks for blocked addresses
    vector::contains(&list.blocked_addresses, &addr)
}

public fun require_not_blocked(list: &Blacklist, addr: address) {
    assert!(!is_blocked(list, addr), E_ADDRESS_BLOCKED);
}
```

## Testing Addresses

```move
#[test]
fun test_address_operations() {
    let addr1: address = @0x1;
    let addr2: address = @0x2;
    let zero: address = @0x0;
    
    // Test equality
    assert!(addr1 == @0x1, 0);
    assert!(addr1 != addr2, 1);
    assert!(zero == @0x0, 2);
    
    // Test validation
    assert!(is_valid_address(addr1), 3);
    assert!(!is_valid_address(zero), 4);
}

#[test]
fun test_address_conversion() {
    let addr: address = @0x123;
    // Test address properties
    assert!(addr != @0x0, 0);
}
```

## Best Practices

1. **Always validate addresses**: Check for zero address before transfers
2. **Use constants for special addresses**: Define `SYSTEM_ADDRESS`, etc.
3. **Don't hardcode addresses**: Use configuration or constructor parameters
4. **Be careful with comparisons**: Address comparison is case-insensitive
5. **Document address purposes**: Comment what each address represents

## Security Considerations

### Reentrancy Protection

```move
struct Account has key, store {
    id: UID,
    owner: address,
    balance: u64,
    locked: bool, // Prevent reentrancy
}

public fun withdraw(account: &mut Account, amount: u64) {
    assert!(!account.locked, E_REENTRANCY);
    account.locked = true;
    
    // Perform withdrawal
    account.balance -= amount;
    
    account.locked = false;
}
```

### Address Spoofing

Never trust addresses passed as parameters without verification:

```move
// Bad: Trusts caller-provided address
public fun bad_pattern(provided_addr: address) {
    // Assumes provided_addr is legitimate
}

// Good: Uses transaction sender
public fun good_pattern(ctx: &TxContext) {
    let actual_sender = tx_context::sender(ctx);
    // Uses verified sender
}
```

## Next Steps

- Learn about [Signer](signer.md) for authorization
- Study [Object Management](usage-examples.md#object-management)
- Explore [Transaction Context](usage-examples.md#transaction-context)
