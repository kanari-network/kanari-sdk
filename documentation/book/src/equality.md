# Equality

Equality operations in Move allow you to compare values. Understanding equality is crucial for writing correct programs.

## Equality Operators

### Equal (==)

```move
let a: u64 = 10;
let b: u64 = 10;

assert!(a == b, 0); // true
```

### Not Equal (!=)

```move
let a: u64 = 10;
let b: u64 = 20;

assert!(a != b, 0); // true
```

## Equality by Type

### Integers

```move
assert!(10u64 == 10u64, 0);
assert!(10u64 != 20u64, 1);
assert!(0xFFu8 == 255u8, 2); // Hex equals decimal
```

### Booleans

```move
assert!(true == true, 0);
assert!(false == false, 1);
assert!(true != false, 2);
```

### Addresses

```move
assert!(@0x1 == @0x1, 0);
assert!(@0x1 != @0x2, 1);
assert!(@0x0 != @0x1, 2);
```

### Vectors

Vectors are compared element-by-element:

```move
let v1 = vector[1u64, 2, 3];
let v2 = vector[1u64, 2, 3];
let v3 = vector[1u64, 2, 4];

assert!(v1 == v2, 0); // Same elements
assert!(v1 != v3, 1); // Different last element
assert!(v1 != vector[1u64], 2); // Different length
```

### Byte Vectors

```move
assert!(b"hello" == b"hello", 0);
assert!(b"hello" != b"world", 1);
assert!(b"" == b"", 2); // Empty vectors equal
```

## Struct Equality

Structs can be compared if all fields support equality:

```move
#[derive(copy, drop)]
struct Point {
    x: u64,
    y: u64,
}

let p1 = Point { x: 10, y: 20 };
let p2 = Point { x: 10, y: 20 };
let p3 = Point { x: 10, y: 30 };

assert!(p1 == p2, 0); // All fields equal
assert!(p1 != p3, 1); // y field different
```

### Nested Structs

```move
#[derive(copy, drop)]
struct Address {
    street: vector<u8>,
    city: vector<u8>,
}

#[derive(copy, drop)]
struct Person {
    name: vector<u8>,
    age: u64,
    address: Address,
}

let addr1 = Address {
    street: b"123 Main",
    city: b"Springfield",
};

let addr2 = Address {
    street: b"123 Main",
    city: b"Springfield",
};

let person1 = Person {
    name: b"Alice",
    age: 30,
    address: addr1,
};

let person2 = Person {
    name: b"Alice",
    age: 30,
    address: addr2,
};

assert!(person1 == person2, 0); // Deep equality
```

## Option Equality

```move
use std::option;

let some1 = option::some(42u64);
let some2 = option::some(42u64);
let some3 = option::some(43u64);
let none1: Option<u64> = option::none();
let none2: Option<u64> = option::none();

assert!(some1 == some2, 0);
assert!(some1 != some3, 1);
assert!(none1 == none2, 2);
assert!(some1 != none1, 3);
```

## Common Patterns

### Checking Against Constants

```move
const ZERO: u64 = 0;
const MAX: u64 = 1000;

public fun validate_amount(amount: u64): bool {
    amount != ZERO && amount <= MAX
}
```

### State Comparison

```move
enum Status has copy, drop {
    Pending,
    Active,
    Completed,
}

public fun is_completed(status: Status): bool {
    status == Status::Completed
}

public fun is_not_pending(status: Status): bool {
    status != Status::Pending
}
```

### Vector Search

```move
public fun contains(items: &vector<u64>, target: u64): bool {
    let len = vector::length(items);
    let mut i = 0;
    
    while (i < len) {
        if (*vector::borrow(items, i) == target) {
            return true;
        };
        i = i + 1;
    };
    
    false
}
```

### Finding Index

```move
public fun index_of(items: &vector<u64>, target: u64): Option<u64> {
    let len = vector::length(items);
    let mut i = 0;
    
    while (i < len) {
        if (*vector::borrow(items, i) == target) {
            return option::some(i);
        };
        i = i + 1;
    };
    
    option::none()
}
```

## Equality in Conditions

### If Statements

```move
let status = get_status();

if (status == Status::Active) {
    // Handle active state
} else if (status == Status::Pending) {
    // Handle pending state
} else {
    // Handle other states
};
```

### While Loops

```move
let mut counter = 0;
let target = 10;

while (counter != target) {
    counter = counter + 1;
};
```

### Assertions

```move
public fun test_equality() {
    assert!(10 == 10, 0);
    assert!(true == true, 1);
    assert!(b"test" == b"test", 2);
    assert!(@0x1 == @0x1, 3);
}
```

## Advanced Equality

### Custom Equality Functions

For complex types, implement custom equality:

```move
#[derive(copy, drop)]
struct Fraction {
    numerator: u64,
    denominator: u64,
}

/// Check if two fractions are equal (cross-multiply)
public fun fractions_equal(f1: &Fraction, f2: &Fraction): bool {
    f1.numerator * f2.denominator == f2.numerator * f1.denominator
}

// Usage
let f1 = Fraction { numerator: 1, denominator: 2 };
let f2 = Fraction { numerator: 2, denominator: 4 };

assert!(fractions_equal(&f1, &f2), 0); // 1/2 == 2/4
```

### Approximate Equality

For floating-point-like comparisons:

```move
/// Check if two values are within tolerance
public fun approximately_equal(a: u64, b: u64, tolerance: u64): bool {
    let diff = if (a > b) { a - b } else { b - a };
    diff <= tolerance
}

// Usage
assert!(approximately_equal(100, 102, 5), 0); // Within tolerance
assert!(!approximately_equal(100, 110, 5), 1); // Outside tolerance
```

### Case-Insensitive String Comparison

```move
public fun strings_equal_ignore_case(s1: &vector<u8>, s2: &vector<u8>): bool {
    if (vector::length(s1) != vector::length(s2)) {
        return false;
    };
    
    let len = vector::length(s1);
    let mut i = 0;
    
    while (i < len) {
        let c1 = *vector::borrow(s1, i);
        let c2 = *vector::borrow(s2, i);
        
        // Convert to lowercase and compare
        if (to_lowercase(c1) != to_lowercase(c2)) {
            return false;
        };
        
        i = i + 1;
    };
    
    true
}

fun to_lowercase(c: u8): u8 {
    if (c >= 65 && c <= 90) { // A-Z
        c + 32 // Convert to a-z
    } else {
        c
    }
}
```

## Best Practices

### 1. Use Appropriate Comparison

```move
// Good: Clear intent
if (balance == 0) { }
if (balance > 0) { }

// Bad: Confusing
if (!(balance != 0)) { }
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

### 3. Document Equality Semantics

```move
/// Checks if two users are equal
/// Compares all fields: name, age, and address
public fun users_equal(u1: &User, u2: &User): bool {
    u1 == u2
}
```

### 4. Be Careful with Floating Point

```move
// Don't use exact equality for calculated values
// Bad: if (calculated_value == expected_value) { }

// Good: Use tolerance
if (approximately_equal(calculated, expected, TOLERANCE)) { }
```

## Common Errors

### Type Mismatch

```move
// Wrong: Can't compare different types
// assert!(10u64 == 10u8, 0); // Error!

// Correct: Same types
assert!(10u64 == 10u64, 0);
```

### Comparing Non-Comparable Types

```move
// Resources with key ability may not support ==
struct Resource has key, store {
    id: UID,
}

// Can't directly compare resources
// let r1 = Resource { id: new(ctx) };
// let r2 = Resource { id: new(ctx) };
// assert!(r1 == r2, 0); // Error!

// Compare specific fields instead
assert!(object::id_to_address(&r1.id) == object::id_to_address(&r2.id), 0);
```

### Vector Length Mismatch

```move
let v1 = vector[1u64, 2];
let v2 = vector[1u64, 2, 3];

assert!(v1 != v2, 0); // Different lengths
```

## Performance Considerations

- Integer comparison is very fast (single CPU instruction)
- Vector comparison is O(n) - proportional to length
- Struct comparison checks all fields
- Short-circuit on first difference

## Testing Equality

```move
#[test]
fun test_integer_equality() {
    assert!(10 == 10, 0);
    assert!(10 != 20, 1);
    assert!(0xFF == 255, 2);
}

#[test]
fun test_vector_equality() {
    let v1 = vector[1u64, 2, 3];
    let v2 = vector[1u64, 2, 3];
    let v3 = vector[1u64, 2, 4];
    
    assert!(v1 == v2, 0);
    assert!(v1 != v3, 1);
}

#[test]
fn test_struct_equality() {
    let p1 = Point { x: 10, y: 20 };
    let p2 = Point { x: 10, y: 20 };
    let p3 = Point { x: 10, y: 30 };
    
    assert!(p1 == p2, 0);
    assert!(p1 != p3, 1);
}
```

## Next Steps

- Learn about [Comparison Operators](integers.md#comparison-operators)
- Study [Pattern Matching](conditionals.md)
- Explore [Testing](unit-testing.md) equality
