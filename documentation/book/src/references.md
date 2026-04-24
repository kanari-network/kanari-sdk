# References

References in Move allow you to borrow values without taking ownership. They are essential for efficient and safe code.

## Immutable References

Immutable references (`&`) allow read-only access:

```move
let x: u64 = 10;
let x_ref = &x; // Immutable reference

// Can read through reference
let y = *x_ref; // 10

// Cannot modify through immutable reference
// *x_ref = 20; // Error!
```

### Function Parameters

```move
// Takes immutable reference
public fun read_value(data: &u64): u64 {
    *data // Dereference to get value
}

// Usage
let x: u64 = 42;
let val = read_value(&x);
assert!(val == 42, 0);
```

## Mutable References

Mutable references (`&mut`) allow reading and writing:

```move
let mut x: u64 = 10;
let x_mut = &mut x; // Mutable reference

// Can read
let y = *x_mut; // 10

// Can write
*x_mut = 20;
assert!(x == 20, 0);
```

### Function Parameters

```move
// Takes mutable reference
public fun increment(counter: &mut u64) {
    *counter = *counter + 1;
}

// Usage
let mut count: u64 = 0;
increment(&mut count);
increment(&mut count);
assert!(count == 2, 0);
```

## Borrowing Rules

### One Mutable or Many Immutable

```move
let mut x: u64 = 10;

// Can have multiple immutable references
let ref1 = &x;
let ref2 = &x;
let ref3 = &x;

// OR one mutable reference
let mut_ref = &mut x;

// But NOT both at the same time
// let ref1 = &x;
// let mut_ref = &mut x; // Error!
```

### No Dangling References

```move
let mut x: u64 = 10;
let x_ref = &x;

// x is still valid, can use both
assert!(*x_ref == 10, 0);
assert!(x == 10, 1);
```

## Borrowing Structs

### Borrow Fields

```move
struct Point has copy, drop {
    x: u64,
    y: u64,
}

let p = Point { x: 10, y: 20 };

// Borrow individual fields
let x_ref = &p.x;
let y_ref = &p.y;

assert!(*x_ref == 10, 0);
assert!(*y_ref == 20, 1);
```

### Mutable Field Access

```move
let mut p = Point { x: 10, y: 20 };

// Mutable borrow of field
let x_mut = &mut p.x;
*x_mut = 100;

assert!(p.x == 100, 0);
assert!(p.y == 20, 1); // Other field unchanged
```

## Borrowing Vectors

### Immutable Vector Access

```move
let nums = vector[10u64, 20, 30];

// Borrow element (returns reference)
let first = vector::borrow(&nums, 0);
assert!(*first == 10, 0);

// Can't modify through immutable borrow
// *first = 100; // Error!
```

### Mutable Vector Access

```move
let mut nums = vector[10u64, 20, 30];

// Mutable borrow of element
let first_mut = vector::borrow_mut(&mut nums, 0);
*first_mut = 100;

assert!(*vector::borrow(&nums, 0) == 100, 0);
```

## Common Patterns

### Read-Only Functions

```move
public fun get_balance(account: &Account): u64 {
    account.balance
}

public fun is_active(user: &User): bool {
    user.status == UserStatus::Active
}

public fun sum(numbers: &vector<u64>): u64 {
    let mut total = 0;
    let len = vector::length(numbers);
    let mut i = 0;
    
    while (i < len) {
        total = total + *vector::borrow(numbers, i);
        i = i + 1;
    };
    
    total
}
```

### Update Functions

```move
public fun update_balance(account: &mut Account, amount: u64) {
    account.balance = account.balance + amount;
}

public fun set_name(user: &mut User, name: vector<u8>) {
    user.name = name;
}

public fun increment_all(counters: &mut vector<u64>) {
    let len = vector::length(counters);
    let mut i = 0;
    
    while (i < len) {
        let counter = vector::borrow_mut(counters, i);
        *counter = *counter + 1;
        i = i + 1;
    };
}
```

### Getter/Setter Pattern

```move
struct Config has key, store {
    id: UID,
    max_supply: u64,
    min_price: u64,
    is_active: bool,
}

// Getters (immutable references)
public fun get_max_supply(config: &Config): u64 {
    config.max_supply
}

public fun get_min_price(config: &Config): u64 {
    config.min_price
}

public fun is_config_active(config: &Config): bool {
    config.is_active
}

// Setters (mutable references)
public fun set_max_supply(config: &mut Config, value: u64) {
    config.max_supply = value;
}

public fun set_min_price(config: &mut Config, value: u64) {
    config.min_price = value;
}

public fun toggle_active(config: &mut Config) {
    config.is_active = !config.is_active;
}
```

## Advanced Borrowing

### Nested Structs

```move
struct Address has copy, drop {
    street: vector<u8>,
    city: vector<u8>,
}

struct Person has copy, drop {
    name: vector<u8>,
    address: Address,
}

let person = Person {
    name: b"Alice",
    address: Address {
        street: b"123 Main St",
        city: b"Springfield",
    },
};

// Borrow nested field
let city = &person.address.city;
assert!(*city == b"Springfield", 0);
```

### Conditional Borrowing

```move
public fun get_or_default(opt: &Option<u64>, default: u64): u64 {
    if (option::is_some(opt)) {
        *option::borrow(opt)
    } else {
        default
    }
}
```

### Iterator Pattern

```move
public fun for_each(items: &vector<u64>, f: |&u64|) {
    let len = vector::length(items);
    let mut i = 0;
    
    while (i < len) {
        let item = vector::borrow(items, i);
        f(item);
        i = i + 1;
    };
}

// Usage
let nums = vector[1u64, 2, 3, 4, 5];
for_each(&nums, |x| {
    // Process each item
    log(*x);
});
```

## Best Practices

### 1. Prefer References Over Copies

```move
// Bad: Unnecessary copy
public fn process_bad(data: LargeStruct) {
    // Uses data
}

// Good: Use reference
public fn process_good(data: &LargeStruct) {
    // Uses data
}
```

### 2. Use Immutable When Possible

```move
// Prefer immutable
public fn read_data(data: &Data) { }

// Only use mutable when necessary
public fn update_data(data: &mut Data) { }
```

### 3. Minimize Borrow Duration

```move
// Bad: Long-lived borrow
let ref1 = &data.field1;
let ref2 = &data.field2;
// ... many lines ...
use(ref1);
use(ref2);

// Good: Short-lived borrows
use(&data.field1);
use(&data.field2);
```

### 4. Document Borrow Requirements

```move
/// Updates user balance
/// 
/// # Arguments
/// * `account` - Mutable reference to account
/// * `amount` - Amount to add (must be > 0)
public fun deposit(account: &mut Account, amount: u64) {
    assert!(amount > 0, 0);
    account.balance += amount;
}
```

## Common Errors

### Borrow Checker Errors

```move
let mut x: u64 = 10;
let ref1 = &x;
// let ref_mut = &mut x; // Error: cannot borrow as mutable while immutable exists

let mut y: u64 = 20;
let mut_ref = &mut y;
// let ref1 = &y; // Error: cannot borrow as immutable while mutable exists
```

### Use After Move

```move
struct NoCopy { value: u64 }

let data = NoCopy { value: 10 };
let moved = data; // Move
// let ref = &data.value; // Error: data has been moved
```

### Dangling Reference

```move
// Can't return reference to local variable
// public fn bad_return(): &u64 {
//     let x = 10;
//     &x // Error: x will be dropped
// }
```

## Performance Considerations

- References avoid copying large data structures
- Immutable references enable compiler optimizations
- Mutable references prevent certain optimizations
- Minimize reference chaining for better performance

## Next Steps

- Learn about [Ownership](structs-and-resources.md#ownership)
- Study [Borrowing Rules](../advanced/borrowing.md)
- Explore [Memory Safety](../security/memory-safety.md)
