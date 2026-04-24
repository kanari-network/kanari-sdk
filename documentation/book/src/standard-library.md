# Standard Library

The Move standard library provides essential modules for common operations. This guide covers the most frequently used modules and functions.

## Overview

Move's standard library includes:

- **Vector Operations**: Collection manipulation
- **Option Type**: Handling optional values
- **String/ASCII**: Text processing
- **Hash Functions**: Cryptographic hashing
- **BCS Encoding**: Binary serialization
- **Type Information**: Runtime type inspection

## Vector Module

### Basic Operations

```move
use std::vector;

// Create empty vector
let mut v = vector::empty<u64>();

// Create with literals
let nums = vector[1u64, 2, 3, 4, 5];

// Length
let len = vector::length(&nums); // 5

// Check if empty
let is_empty = vector::is_empty(&nums); // false
```

### Adding Elements

```move
let mut v = vector::empty<u64>();

// Push back
vector::push_back(&mut v, 10);
vector::push_back(&mut v, 20);

// Append another vector
let other = vector[30u64, 40];
vector::append(&mut v, other);
// v is now [10, 20, 30, 40]
```

### Accessing Elements

```move
let nums = vector[10u64, 20, 30];

// Borrow (read-only)
let first = vector::borrow(&nums, 0); // &10

// Borrow mutable
let mut nums_mut = vector[10u64, 20];
let first_mut = vector::borrow_mut(&mut nums_mut, 0);
*first_mut = 100;

// Pop back
let last = vector::pop_back(&mut nums); // 30
```

### Searching

```move
let nums = vector[10u64, 20, 30, 20];

// Contains
let has_20 = vector::contains(&nums, &20); // true

// Index of
let index = vector::index_of(&nums, &20); // (true, 1)

// Reverse contains
let has_99 = vector::contains(&nums, &99); // false
```

### Removing Elements

```move
let mut nums = vector[10u64, 20, 30, 40];

// Remove at index
let removed = vector::remove(&mut nums, 1); // 20
// nums is now [10, 30, 40]

// Remove value
let (found, value) = vector::remove_value(&mut nums, &30);
// found = true, value = 30

// Destroy empty vector
vector::destroy_empty(nums);
```

### Iteration

```move
let nums = vector[1u64, 2, 3, 4, 5];
let mut sum = 0;
let len = vector::length(&nums);
let mut i = 0;

while (i < len) {
    sum = sum + *vector::borrow(&nums, i);
    i = i + 1;
};
```

## Option Module

The `Option` type represents optional values.

### Creating Options

```move
use std::option;

// Some value
let some_val = option::some(42u64);

// None
let no_val: Option<u64> = option::none();
```

### Checking Options

```move
let opt = option::some(100u64);

// Is some
if (option::is_some(&opt)) {
    // Has value
};

// Is none
if (option::is_none(&opt)) {
    // No value
};
```

### Extracting Values

```move
let opt = option::some(42u64);

// Borrow
let val = option::borrow(&opt); // &42

// Extract (consumes option)
let val = option::extract(&mut opt); // 42

// With default
let val = option::get_with_default(&opt, 0); // 42
let val2 = option::get_with_default(&option::none(), 0); // 0
```

### Pattern Matching

```move
public fun unwrap_or_zero(opt: Option<u64>): u64 {
    if (option::is_some(&opt)) {
        *option::borrow(&opt)
    } else {
        0
    }
}
```

## String and ASCII Modules

### ASCII Strings

```move
use std::ascii;

// Create ASCII string
let s = ascii::string(b"Hello");

// To bytes
let bytes = ascii::into_bytes(s);

// Char operations
let char = ascii::char(b'A');
let byte = ascii::byte(char); // 65
```

### UTF-8 Strings

```move
use std::string;

// Create UTF-8 string
let s = string::utf8(b"Hello, World!");

// Convert to bytes
let bytes = string::bytes(&s);

// Check if valid UTF-8
let is_valid = string::is_valid_utf8(&bytes);
```

## Hash Module

Cryptographic hash functions:

```move
use std::hash;

let data = b"hello world";

// SHA2-256
let sha256 = hash::sha2_256(data);

// SHA3-256
let sha3 = hash::sha3_256(data);

// Blake2b-256
let blake2b = hash::blake2b_256(data);

// Keccak256 (Ethereum)
let keccak = hash::keccak256(data);
```

## BCS Module

Binary Canonical Serialization:

```move
use std::bcs;

// Serialize
let value = 42u64;
let bytes = bcs::to_bytes(&value);

// Deserialize
let deserialized: u64 = bcs::from_bytes(&bytes);
assert!(deserialized == 42, 0);
```

## Type Name Module

Runtime type information:

```move
use std::type_name;

// Get type name
let name = type_name::get<u64>();

// Convert to string
let name_str = type_name::into_string(name);
```

## Kanari System Modules

In addition to the standard library, Kanari provides extended modules:

### Math Module

```move
use kanari_system::math;

// Safe arithmetic
let sum = math::checked_add_u64(a, b);
let product = math::checked_mul_u64(a, b);

// Utility functions
let sqrt = math::sqrt_u64(100); // 10
let power = math::pow_u64(2, 8); // 256
let min = math::min_u64(10, 20); // 10
let max = math::max_u64(10, 20); // 20
let diff = math::diff_u64(100, 80); // 20
```

### Object Module

```move
use kanari_system::object;

// Create new object ID
let uid = object::new(ctx);

// Delete object
object::delete(uid);

// Save object state
object::save_object(&obj);
```

### Transfer Module

```move
use kanari_system::transfer;

// Transfer object
transfer::public_transfer(obj, recipient);

// Freeze object
transfer::public_freeze_object(obj);
```

### Coin Module

```move
use kanari_system::coin;

// Create currency
let (cap, meta) = coin::create_currency::<TOKEN>(
    TOKEN {}, 9, b"TKN", b"Token", b"Desc", option::none(), ctx
);

// Mint tokens
let coins = coin::mint(&mut cap, 1000, ctx);

// Burn tokens
coin::burn(&mut cap, coins);
```

## Common Patterns

### Vector Filtering

```move
public fun filter_positive(numbers: &vector<i64>): vector<u64> {
    let mut result = vector::empty<u64>();
    let len = vector::length(numbers);
    let mut i = 0;
    
    while (i < len) {
        let num = *vector::borrow(numbers, i);
        if (num > 0) {
            vector::push_back(&mut result, (num as u64));
        };
        i = i + 1;
    };
    
    result
}
```

### Option Chaining

```move
public fun get_nested_value(outer: Option<Option<u64>>): u64 {
    if (option::is_some(&outer)) {
        let inner = option::borrow(&outer);
        if (option::is_some(inner)) {
            *option::borrow(inner)
        } else {
            0
        }
    } else {
        0
    }
}
```

### Hash and Compare

```move
public fun verify_data(data: &vector<u8>, expected_hash: &vector<u8>): bool {
    let actual_hash = hash::sha2_256(data);
    vector::compare(&actual_hash, expected_hash) == 0
}
```

## Best Practices

### 1. Use Appropriate Data Structures

```move
// For small collections: vector
let items = vector[1u64, 2, 3];

// For optional values: Option
let maybe_value: Option<u64> = option::some(42);

// For key-value pairs: Table or Bag
let mut table = table::new<u64, address>(ctx);
```

### 2. Handle Edge Cases

```move
public fun safe_divide(a: u64, b: u64): Option<u64> {
    if (b == 0) {
        option::none()
    } else {
        option::some(a / b)
    }
}
```

### 3. Validate Input

```move
public fun safe_vector_access(v: &vector<u64>, index: u64): Option<u64> {
    if (index >= vector::length(v)) {
        option::none()
    } else {
        option::some(*vector::borrow(v, index))
    }
}
```

### 4. Clean Up Resources

```move
public fun cleanup_table(table: Table<u64, address>) {
    // Ensure table is empty before destroying
    assert!(table::length(&table) == 0, 0);
    table::destroy_empty(table);
}
```

## Performance Tips

- Vectors are efficient for small collections (<100 items)
- Use Tables/Bags for large datasets
- Minimize vector reallocations by pre-sizing when possible
- Avoid unnecessary copies - use references
- Batch operations when possible

## Next Steps

- Explore [Vector Module](vector.md) in detail
- Learn about [Kanari System Modules](usage-examples.md)
- Study [Coding Conventions](coding-conventions.md)
