# Generics

Generics allow you to write code that works with multiple types. They provide flexibility while maintaining type safety.

## Generic Functions

```move
// Identity function - works with any type
public fun identity<T>(x: T): T {
    x
}

// Usage with different types
let num = identity<u64>(42);
let bool_val = identity<bool>(true);
let addr = identity<address>(@0x1);
```

## Generic Structs

```move
struct Container<T> has store {
    value: T,
}

public fun create_container<T: store>(val: T): Container<T> {
    Container { value: val }
}

// Usage
let num_container = create_container<u64>(100);
let str_container = create_container<vector<u8>>(b"hello");
```

## Type Constraints

Constrain generic types with abilities:

### Single Constraint

```move
// T must have 'copy' ability
public fun duplicate<T: copy>(x: T): (T, T) {
    (x, x)
}

// T must have 'drop' ability
public fn consume<T: drop>(_x: T) {
    // Can drop x
}
```

### Multiple Constraints

```move
// T must have both 'copy' and 'drop'
public fn process<T: copy + drop>(x: T) {
    let y = x; // Copy
    // x is automatically dropped
}

// All four abilities
public fn full_access<T: copy + drop + store + key>(x: T) {
    // Can do anything with T
}
```

## Multiple Type Parameters

```move
struct Pair<T, U> has store {
    first: T,
    second: U,
}

public fun make_pair<T: store, U: store>(
    first: T,
    second: U
): Pair<T, U> {
    Pair { first, second }
}

// Usage
let pair = make_pair<u64, bool>(42, true);
let pair2 = make_pair<address, u64>(@0x1, 100);
```

## Generic Structs with Constraints

```move
// Only types with 'store' can be in Container
struct StorableContainer<T: store> has key, store {
    id: UID,
    value: T,
}

// Only copyable types in CopiableBox
struct CopyableBox<T: copy + drop> has copy, drop {
    value: T,
}
```

## Phantom Type Parameters

Phantom parameters don't appear in struct fields but provide type safety:

```move
struct Coin<phantom T> has key, store {
    id: UID,
    value: u64,
}

// T is phantom - doesn't affect storage
struct KANARI has drop {}
struct USD has drop {}

let kanari = Coin<KANARI> { id: new(ctx), value: 100 };
let usd = Coin<USD> { id: new(ctx), value: 50 };

// Type system prevents mixing
// merge_coins(kanari, usd); // Compile error!
```

## Common Patterns

### Option Type

```move
enum Optional<T> has copy, drop, store {
    Some(T),
    None,
}

public fun some<T: store>(val: T): Optional<T> {
    Optional::Some(val)
}

public fun none<T: store>(): Optional<T> {
    Optional::None
}

public fun unwrap<T: copy>(opt: &Optional<T>): T {
    match (opt) {
        Optional::Some(val) => *val,
        Optional::None => abort 0,
    }
}

public fun is_some<T>(opt: &Optional<T>): bool {
    matches!(opt, Optional::Some(_))
}
```

### Result Type

```move
enum Result<T, E> has copy, drop, store {
    Ok(T),
    Err(E),
}

public fun ok<T: store, E: store>(val: T): Result<T, E> {
    Result::Ok(val)
}

public fun err<T: store, E: store>(error: E): Result<T, E> {
    Result::Err(error)
}

public fun unwrap_or<T: copy, E>(result: &Result<T, E>, default: T): T {
    match (result) {
        Result::Ok(val) => *val,
        Result::Err(_) => default,
    }
}
```

### Collection Types

```move
// Generic linked list
struct Node<T> has store {
    value: T,
    next: Option<UID>,
}

struct LinkedList<T> has key, store {
    id: UID,
    head: Option<UID>,
    length: u64,
}

// Generic map
struct Map<K: copy + drop + store, V: store> has key, store {
    id: UID,
    keys: vector<K>,
    values: vector<V>,
}
```

## Generic Functions in Practice

### Swap Function

```move
public fun swap<T>(a: &mut T, b: &mut T) {
    let temp = *a;
    *a = *b;
    *b = temp;
}

// Works with any type
let mut x = 10;
let mut y = 20;
swap<u64>(&mut x, &mut y);
```

### Map/Filter Pattern

```move
public fun map<T: copy, U: store>(
    items: &vector<T>,
    f: |&T| -> U
): vector<U> {
    let mut result = vector::empty<U>();
    let len = vector::length(items);
    let mut i = 0;
    
    while (i < len) {
        let item = vector::borrow(items, i);
        vector::push_back(&mut result, f(item));
        i = i + 1;
    };
    
    result
}

public fun filter<T: copy>(
    items: &vector<T>,
    predicate: |&T| -> bool
): vector<T> {
    let mut result = vector::empty<T>();
    let len = vector::length(items);
    let mut i = 0;
    
    while (i < len) {
        let item = vector::borrow(items, i);
        if (predicate(item)) {
            vector::push_back(&mut result, *item);
        };
        i = i + 1;
    };
    
    result
}
```

### Factory Pattern

```move
public fun create_multiple<T: store + drop>(
    factory: || -> T,
    count: u64
): vector<T> {
    let mut items = vector::empty<T>();
    let mut i = 0;
    
    while (i < count) {
        vector::push_back(&mut items, factory());
        i = i + 1;
    };
    
    items
}

// Usage
let counters = create_multiple(|| Counter { value: 0 }, 10);
```

## Advanced Generics

### Nested Generics

```move
struct Nested<T, U> has store {
    outer: Container<T>,
    inner: Container<U>,
}

public fun create_nested<T: store, U: store>(
    outer_val: T,
    inner_val: U
): Nested<T, U> {
    Nested {
        outer: create_container(outer_val),
        inner: create_container(inner_val),
    }
}
```

### Generic Constraints on Struct Fields

```move
struct Wrapper<T: store> has key, store {
    id: UID,
    data: T,
}

// Can only wrap storable types
let wrapper = Wrapper<u64> { id: new(ctx), data: 42 };
```

### Trait-like Patterns

While Move doesn't have traits, you can simulate them:

```move
// Define interface via functions
public trait Serializable {
    public fun serialize<T>(obj: &T): vector<u8>;
    public fun deserialize<T>(data: vector<u8>): T;
}

// Implement for specific types
public fun serialize_u64(val: &u64): vector<u8> {
    bcs::to_bytes(val)
}

public fun serialize_address(addr: &address): vector<u8> {
    bcs::to_bytes(addr)
}
```

## Testing Generics

```move
#[test]
fun test_identity() {
    assert!(identity<u64>(42) == 42, 0);
    assert!(identity<bool>(true) == true, 1);
    assert!(identity<address>(@0x1) == @0x1, 2);
}

#[test]
fun test_generic_struct() {
    let container = create_container<u64>(100);
    assert!(container.value == 100, 0);
    
    let container2 = create_container<bool>(false);
    assert!(container2.value == false, 1);
}

#[test]
fun test_swap() {
    let mut a = 10;
    let mut b = 20;
    swap<u64>(&mut a, &mut b);
    assert!(a == 20, 0);
    assert!(b == 10, 1);
}

#[test]
fun test_phantom_types() {
    let coin1 = Coin<KANARI> { id: new(ctx), value: 100 };
    let coin2 = Coin<USD> { id: new(ctx), value: 50 };
    
    // Different types despite same structure
    assert!(coin1.value == 100, 0);
    assert!(coin2.value == 50, 1);
}
```

## Best Practices

### 1. Use Descriptive Type Parameter Names

```move
// Bad
public fn process<T, U>(x: T, y: U) { }

// Good
public fn process<Key: copy + drop, Value: store>(
    key: Key,
    value: Value
) { }
```

### 2. Minimize Constraints

```move
// Only constrain what you need
// Bad: Over-constrained
public fn unnecessary<T: copy + drop + store>(x: T) { }

// Good: Minimal constraints
public fn minimal<T: store>(x: T) { }
```

### 3. Document Generic Types

```move
/// A generic container that holds a single value of type T.
/// 
/// # Type Parameters
/// * `T` - The type of value to store (must have 'store' ability)
struct Container<T: store> {
    value: T,
}
```

### 4. Use Phantom Types for Type Safety

```move
// Prevents mixing different token types at compile time
struct Token<phantom Currency> has key, store {
    id: UID,
    amount: u64,
}
```

## Common Errors

### Missing Type Annotation

```move
// Compiler can't infer type
// let container = create_container(42); // Error

// Provide explicit type
let container = create_container<u64>(42); // OK
```

### Insufficient Constraints

```move
public fn bad_example<T>(x: T) {
    // let y = x; // Error: T might not have 'copy'
}

public fn good_example<T: copy>(x: T) {
    let y = x; // OK: T has 'copy'
}
```

### Phantom Type Misuse

```move
// Wrong: Using non-phantom when field doesn't use it
struct Bad<T> has store {
    value: u64, // T not used
}

// Correct: Use phantom
struct Good<phantom T> has store {
    value: u64,
}
```

## Performance Considerations

- Generics are monomorphized at compile time (no runtime overhead)
- Each concrete type combination generates separate code
- Too many generic combinations can increase binary size
- Prefer concrete types when performance is critical

## Next Steps

- Learn about [Type Abilities](abilities.md)
- Study [Standard Library](standard-library.md) generics
- Explore [Collections](usage-examples.md#data-structures)
