# Move Quick Reference Card

A handy reference for common Move operations in Kanari.

## Imports

```move
// Standard library
use std::vector;
use std::option;
use std::string;
use std::ascii;

// Kanari System modules
use kanari_system::coin;
use kanari_system::transfer;
use kanari_system::object;
use kanari_system::tx_context::TxContext;
use kanari_system::collection;
use kanari_system::balance;
use kanari_system::table;
use kanari_system::bag;
```

## Object Creation

```move
use kanari_system::object::{UID, new};

struct MyObject has key, store {
    id: UID,
    value: u64,
}

public fun create_object(ctx: &mut TxContext): MyObject {
    MyObject {
        id: new(ctx),
        value: 100,
    }
}
```

## Token Operations

### Create Token

```move
struct MY_TOKEN has drop {}

let (cap, meta) = coin::create_currency<MY_TOKEN>(
    MY_TOKEN {},
    9,              // decimals
    b"MTK",         // symbol
    b"My Token",    // name
    b"Description",
    option::none(), // icon
    ctx
);
```

### Mint Tokens

```move
let coins = coin::mint(&mut cap, 1000, ctx);
// or
coin::mint_and_transfer(&mut cap, 1000, recipient, ctx);
```

### Burn Tokens

```move
let burned = coin::burn(&mut cap, coins);
```

### Transfer Tokens

```move
transfer::public_transfer(coins, recipient);
```

### Split/Merge

```move
let split = coin::split(&mut coins, 500, ctx);
coin::join(&mut coins1, coins2);
```

## NFT Operations

### Create Collection

```move
let (col, cap) = collection::create_collection(
    b"My NFTs",
    b"Description",
    10000,  // max supply
    ctx
);
```

### Mint NFT

```move
let nft = collection::mint(&mut cap, ctx);
transfer::public_transfer(nft, recipient);
```

### Check Supply

```move
let remaining = collection::remaining(&cap);
let issued = collection::issued_counter(&cap);
```

## Data Structures

### Table (Homogeneous)

```move
let mut tbl = table::new<u64, address>(ctx);
table::add(&mut tbl, 1, @0x123);
let val = table::borrow(&tbl, 1);
table::remove(&mut tbl, 1);
```

### Bag (Heterogeneous)

```move
let mut bag = bag::new(ctx);
bag::add(&mut bag, b"key", value);
let val = bag::borrow<Type>(&bag, b"key");
bag::remove<Type>(&mut bag, b"key");
```

### Dynamic Fields

```move
dynamic_field::add(&parent_id, b"key", value);
let val = dynamic_field::borrow<Type>(&parent_id, b"key");
dynamic_field::remove<Type>(&parent_id, b"key");
```

## Transaction Context

```move
let sender = tx_context::sender(ctx);
let tx_hash = tx_context::hash(ctx);
let epoch = tx_context::epoch(ctx);
let ids = tx_context::ids_created(ctx);
```

## Common Patterns

### Witness Pattern

```move
struct TOKEN has drop {}
// Consume witness to prove ownership
let _ = TOKEN {};
```

### Capability Pattern

```move
struct AdminCap has key, store {
    id: UID,
    is_admin: bool,
}

public fun admin_only(cap: &AdminCap) {
    assert!(cap.is_admin, 0);
}
```

### Freeze Object

```move
transfer::public_freeze_object(metadata);
```

### Delete Object

```move
object::delete(uid);
```

## Error Handling

```move
const E_INVALID: u64 = 0;

assert!(condition, E_INVALID);

#[test]
#[expected_failure(abort_code = E_INVALID)]
fun test_error() {
    // Should fail
}
```

## Vectors

```move
let mut v = vector::empty<u64>();
vector::push_back(&mut v, 100);
let len = vector::length(&v);
let val = vector::borrow(&v, 0);
vector::pop_back(&mut v);
```

## Math Operations

```move
use kanari_system::math;

let sqrt = math::sqrt_u64(100);
let pow = math::pow_u64(2, 8);
let diff = math::diff_u64(100, 80);
let pct = math::percentage(1000, 10); // 10%
```

## Hash Functions

```move
use std::hash;

let h1 = hash::sha2_256(&data);
let h2 = hash::keccak256(&data);
let h3 = hash::blake3_256(&data);
```

## Signatures

```move
use kanari_system::ed25519;
use kanari_system::ecdsa_k1;

// Ed25519
let valid = ed25519::verify(&sig, &pubkey, &msg);

// ECDSA K1
let valid = ecdsa_k1::verify(&sig, &pubkey, &msg, 0);
```

## Type Abilities

```move
has copy    // Can be copied
has drop    // Can be discarded
has key     // Can be stored globally
has store   // Can be stored inside objects
```

## Visibility Modifiers

```move
public fun foo() {}           // Anyone can call
public entry fun bar() {}     // Can be called from transactions
fun baz() {}                  // Private (module only)
```

## Generics

```move
public fun example<T: drop>(value: T) {
    // T must have drop ability
}

struct Container<T> has store {
    value: T,
}
```

## Common Constants

```move
const MIST_PER_KARI: u64 = 1_000_000_000;
const MAX_U64: u64 = 18_446_744_073_709_551_615;
const ZERO_ADDRESS: address = @0x0;
```

## Testing

```move
#[test]
fun test_example() {
    let ctx = &mut tx_context::dummy();
    // Test code
    assert!(condition, 0);
}

#[test_only]
public fun test_helper() {
    // Only available in tests
}
```

## Gas Tips

✅ Do:

- Batch operations
- Use efficient data structures
- Minimize storage writes
- Avoid loops over large collections

❌ Don't:

- Store unnecessary data
- Use nested loops
- Create many small objects
- Perform complex calculations on-chain

## Security Checklist

- [ ] Validate all inputs
- [ ] Check for overflow/underflow
- [ ] Verify ownership/capabilities
- [ ] Prevent reentrancy
- [ ] Handle edge cases
- [ ] Add access controls
- [ ] Write comprehensive tests
- [ ] Get security audit

---

**Need more details?** Check out the full [Usage Examples](usage-examples.md) and tutorials!
