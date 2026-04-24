# Structs and Resources

Structs are custom data types in Move that group related values together. Resources are special structs with the `key` ability that can be stored in global storage.

## Basic Structs

```move
// Simple struct
struct Point has copy, drop {
    x: u64,
    y: u64,
}

// Creating instances
let origin = Point { x: 0, y: 0 };
let point = Point { x: 10, y: 20 };

// Accessing fields
let x_coord = point.x;
let y_coord = point.y;
```

## Type Abilities

Structs can have up to four abilities:

### Copy

Can be duplicated:

```move
#[derive(copy, drop)]
struct Data {
    value: u64,
}

let d1 = Data { value: 10 };
let d2 = d1; // Copy
let d3 = d1; // Can copy again
```

### Drop

Can be discarded:

```move
#[derive(drop)]
struct Temporary {
    message: vector<u8>,
}

let temp = Temporary { message: b"hello" };
// temp is automatically dropped at end of scope
```

### Key

Can be stored in global storage (makes it a Resource):

```move
struct Account has key, store {
    id: UID,
    balance: u64,
}
```

### Store

Can be stored inside other resources:

```move
struct Token has store {
    amount: u64,
}

struct Wallet has key, store {
    id: UID,
    tokens: Token,
}
```

## Resources (Key Ability)

Resources are special structs that cannot be copied or dropped implicitly:

```move
use kanari_system::object::{UID, new};
use kanari_system::tx_context::TxContext;

struct NFT has key, store {
    id: UID,
    owner: address,
    metadata_uri: vector<u8>,
}

public fun create_nft(uri: vector<u8>, ctx: &mut TxContext): NFT {
    NFT {
        id: new(ctx),
        owner: tx_context::sender(ctx),
        metadata_uri: uri,
    }
}

// Must explicitly handle resource - cannot just drop it
public fun transfer_nft(nft: NFT, recipient: address) {
    transfer::public_transfer(nft, recipient);
}
```

## Mutable Structs

```move
struct Counter has copy, drop, store {
    value: u64,
}

public fun increment(counter: &mut Counter) {
    counter.value = counter.value + 1;
}

public fun reset(counter: &mut Counter) {
    counter.value = 0;
}
```

## Nested Structs

```move
struct Address has copy, drop, store {
    street: vector<u8>,
    city: vector<u8>,
    zip_code: u64,
}

struct Person has copy, drop, store {
    name: vector<u8>,
    age: u64,
    address: Address,
}

public fun get_city(person: &Person): &vector<u8> {
    &person.address.city
}
```

## Generic Structs

```move
struct Container<T> has store {
    value: T,
}

public fun create_container<T: store>(val: T): Container<T> {
    Container { value: val }
}

public fun get_value<T>(container: &Container<T>): &T {
    &container.value
}

// Usage
let num_container = create_container<u64>(42);
let bool_container = create_container<bool>(true);
```

## Phantom Types

Phantom parameters don't appear in struct fields:

```move
struct Coin<phantom T> has key, store {
    id: UID,
    value: u64,
}

// T is phantom - doesn't affect storage layout
struct KANARI has drop {}
struct USD has drop {}

let kanari_coin = Coin<KANARI> { id: new(ctx), value: 100 };
let usd_coin = Coin<USD> { id: new(ctx), value: 50 };
```

## Destructuring

```move
struct RGB has copy, drop {
    red: u8,
    green: u8,
    blue: u8,
}

let color = RGB { red: 255, green: 128, blue: 64 };

// Full destructuring
let RGB { red, green, blue } = color;

// Partial destructuring
let RGB { red, .. } = color; // Ignore other fields

// Destructuring with rename
let RGB { red: r, green: g, blue: b } = color;
```

## Common Patterns

### Builder Pattern

```move
struct Transaction has key, store {
    id: UID,
    sender: address,
    recipient: address,
    amount: u64,
    executed: bool,
}

struct TransactionBuilder has drop {
    sender: address,
    recipient: address,
    amount: u64,
}

public fun new_builder(sender: address): TransactionBuilder {
    TransactionBuilder {
        sender,
        recipient: @0x0,
        amount: 0,
    }
}

public fun set_recipient(builder: &mut TransactionBuilder, recipient: address) {
    builder.recipient = recipient;
}

public fun build(builder: TransactionBuilder, ctx: &mut TxContext): Transaction {
    assert!(builder.recipient != @0x0, 0);
    assert!(builder.amount > 0, 1);
    
    Transaction {
        id: new(ctx),
        sender: builder.sender,
        recipient: builder.recipient,
        amount: builder.amount,
        executed: false,
    }
}
```

### State Machine

```move
enum OrderState has copy, drop, store {
    Pending,
    Confirmed,
    Shipped,
    Delivered,
    Cancelled,
}

struct Order has key, store {
    id: UID,
    state: OrderState,
    buyer: address,
    seller: address,
}

public fun confirm_order(order: &mut Order) {
    assert!(matches!(order.state, OrderState::Pending), 0);
    order.state = OrderState::Confirmed;
}

public fun ship_order(order: &mut Order) {
    assert!(matches!(order.state, OrderState::Confirmed), 1);
    order.state = OrderState::Shipped;
}
```

### Option-like Pattern

```move
struct Optional<T> has drop, store {
    value: Option<T>,
}

public fun some<T: store>(val: T): Optional<T> {
    Optional { value: option::some(val) }
}

public fun none<T: store>(): Optional<T> {
    Optional { value: option::none() }
}

public fun unwrap<T: copy>(opt: &Optional<T>): T {
    option::borrow(&opt.value)
}
```

## Updating Structs

```move
struct Config has copy, drop, store {
    max_supply: u64,
    min_price: u64,
    is_active: bool,
}

public fun update_config(config: &mut Config, new_max: u64) {
    config.max_supply = new_max;
}

public fun toggle_active(config: &mut Config) {
    config.is_active = !config.is_active;
}

// Update multiple fields
public fun reconfigure(
    config: &mut Config,
    max: u64,
    min: u64
) {
    config.max_supply = max;
    config.min_price = min;
}
```

## Testing Structs

```move
#[test]
fun test_basic_struct() {
    let point = Point { x: 10, y: 20 };
    assert!(point.x == 10, 0);
    assert!(point.y == 20, 1);
}

#[test]
fun test_mutable_struct() {
    let mut counter = Counter { value: 0 };
    increment(&mut counter);
    assert!(counter.value == 1, 0);
    increment(&mut counter);
    assert!(counter.value == 2, 1);
}

#[test]
fun test_destructuring() {
    let color = RGB { red: 255, green: 128, blue: 64 };
    let RGB { red, green, blue } = color;
    assert!(red == 255, 0);
    assert!(green == 128, 1);
    assert!(blue == 64, 2);
}

#[test]
fun test_generic_struct() {
    let container = create_container<u64>(42);
    assert!(*get_value(&container) == 42, 0);
}
```

## Best Practices

### 1. Choose Appropriate Abilities

```move
// Data-only struct
#[derive(copy, drop, store)]
struct Metadata { /* ... */ }

// Resource that lives in storage
struct Asset has key, store { /* ... */ }

// Temporary computation result
#[derive(drop)]
struct CalculationResult { /* ... */ }
```

### 2. Use Descriptive Field Names

```move
// Bad
struct User {
    n: vector<u8>,
    a: u64,
}

// Good
struct User {
    name: vector<u8>,
    age: u64,
}
```

### 3. Encapsulate Internal State

```move
struct Balance has key, store {
    id: UID,
    value: u64,
}

// Don't expose internal field directly
// public fun get_value(balance: &Balance): u64 { balance.value }

// Instead, provide meaningful operations
public fun deposit(balance: &mut Balance, amount: u64) {
    balance.value += amount;
}

public fun withdraw(balance: &mut Balance, amount: u64) {
    assert!(balance.value >= amount, 0);
    balance.value -= amount;
}
```

### 4. Validate Invariants

```move
public fun create_token(amount: u64): Token {
    assert!(amount > 0, E_ZERO_AMOUNT);
    Token { amount }
}

public fun merge_tokens(token1: &mut Token, token2: Token) {
    token1.amount += token2.amount;
    // token2 is consumed
}
```

## Common Errors

### Missing Ability

```move
struct NoDrop {
    value: u64,
}

public fun test() {
    let x = NoDrop { value: 10 };
    // Error: cannot drop x without 'drop' ability
}
```

### Copying Resources

```move
struct Resource has key, store {
    id: UID,
}

let r1 = Resource { id: new(ctx) };
// let r2 = r1; // Error: cannot copy resource
let r2 = r1; // Move (r1 is no longer valid)
```

### Forgetting to Handle Resources

```move
public fun leak_resource(ctx: &mut TxContext) {
    let r = Resource { id: new(ctx) };
    // Error: resource 'r' must be used
}
```

## Next Steps

- Learn about [Global Storage](global-storage-structure.md)
- Study [Type Abilities](abilities.md) in detail
- Explore [Generics](generics.md) for flexibility
