# Tuples and Unit

Tuples group multiple values together, while unit (`()`) represents the absence of a value.

## Tuples

### Tuple Basics

```move
// Create a tuple
let pair: (u64, u64) = (10, 20);
let triple: (u64, bool, address) = (100, true, @0x1);

// Access tuple elements by destructuring
let (x, y) = pair;
assert!(x == 10, 0);
assert!(y == 20, 1);

// Partial destructuring
let (a, ..) = triple; // Only first element
```

### Tuple Types

```move
// Two-element tuple (pair)
let pair: (u64, u64) = (1, 2);

// Three-element tuple
let triple: (u64, u64, u64) = (1, 2, 3);

// Mixed types
let mixed: (u64, bool, vector<u8>) = (42, true, b"hello");

// Nested tuples
let nested: ((u64, u64), (u64, u64)) = ((1, 2), (3, 4));
```

### Multiple Return Values

Functions can return tuples:

```move
public fun min_max(a: u64, b: u64): (u64, u64) {
    if (a < b) {
        (a, b)
    } else {
        (b, a)
    }
}

// Usage
let (min_val, max_val) = min_max(10, 20);
assert!(min_val == 10, 0);
assert!(max_val == 20, 1);
```

### Common Tuple Patterns

#### Swap Function

```move
public fun swap<T>(a: T, b: T): (T, T) {
    (b, a)
}

// Usage
let (x, y) = swap(10, 20);
assert!(x == 20, 0);
assert!(y == 10, 1);
```

#### Division with Remainder

```move
public fun divmod(dividend: u64, divisor: u64): (u64, u64) {
    assert!(divisor > 0, E_DIVISION_BY_ZERO);
    (dividend / divisor, dividend % divisor)
}

// Usage
let (quotient, remainder) = divmod(17, 5);
assert!(quotient == 3, 0);
assert!(remainder == 2, 1);
```

#### Coordinate Operations

```move
type Point = (u64, u64);

public fun distance_squared(p1: Point, p2: Point): u64 {
    let (x1, y1) = p1;
    let (x2, y2) = p2;
    
    let dx = if (x1 > x2) { x1 - x2 } else { x2 - x1 };
    let dy = if (y1 > y2) { y1 - y2 } else { y2 - y1 };
    
    dx * dx + dy * dy
}
```

## Unit Type

The unit type `()` has exactly one value: `()`.

### Unit Basics

```move
// Unit value
let unit: () = ();

// Functions that don't return meaningful values return unit
public fn do_something() {
    // Implicitly returns ()
}

// Explicit unit return
public fn returns_unit(): () {
    ()
}
```

### Void Functions

Functions with side effects return unit:

```move
public entry fun emit_event(amount: u64) {
    event::emit(TransferEvent { amount });
    // Returns ()
}

public entry fun update_state(balance: &mut u64, amount: u64) {
    *balance += amount;
    // Returns ()
}
```

### Ignoring Return Values

```move
// Function returns a value, but we ignore it
let _ = some_function();

// Or just call it
some_function();
```

## Practical Examples

### Result Type with Tuples

```move
/// Returns (success, value) tuple
public fun safe_divide(a: u64, b: u64): (bool, u64) {
    if (b == 0) {
        (false, 0)
    } else {
        (true, a / b)
    }
}

// Usage
let (success, result) = safe_divide(10, 3);
if (success) {
    // Use result
} else {
    // Handle error
};
```

### Enumerated Results

```move
/// Returns (found, index) tuple
public fun find_item(items: &vector<u64>, target: u64): (bool, u64) {
    let len = vector::length(items);
    let mut i = 0;
    
    while (i < len) {
        if (*vector::borrow(items, i) == target) {
            return (true, i);
        };
        i = i + 1;
    };
    
    (false, 0)
}

// Usage
let (found, index) = find_item(&items, 42);
if (found) {
    // Item found at index
};
```

### Batch Operations

```move
/// Process multiple items, return (success_count, fail_count)
public fun process_batch(
    items: &vector<Transaction>
): (u64, u64) {
    let mut success = 0;
    let mut failed = 0;
    let len = vector::length(items);
    let mut i = 0;
    
    while (i < len) {
        if (process_item(vector::borrow(items, i))) {
            success = success + 1;
        } else {
            failed = failed + 1;
        };
        i = i + 1;
    };
    
    (success, failed)
}
```

## Tuple Destructuring Patterns

### Ignore Elements

```move
let triple = (1u64, 2, 3);

// Ignore middle element
let (first, _, third) = triple;

// Ignore last elements
let (only_first, ..) = triple;
```

### Nested Destructuring

```move
let nested = ((1u64, 2), (3, 4));

// Destructure nested tuples
let ((a, b), (c, d)) = nested;
assert!(a == 1, 0);
assert!(b == 2, 1);
assert!(c == 3, 2);
assert!(d == 4, 3);
```

### In Function Parameters

```move
public fun add_points(p1: (u64, u64), p2: (u64, u64)): (u64, u64) {
    let (x1, y1) = p1;
    let (x2, y2) = p2;
    
    (x1 + x2, y1 + y2)
}
```

## Common Use Cases

### Error Handling

```move
/// Returns (error_code, result)
/// error_code = 0 means success
public fun validated_operation(input: u64): (u64, u64) {
    if (input == 0) {
        (E_INVALID_INPUT, 0)
    } else if (input > MAX_VALUE) {
        (E_OUT_OF_RANGE, 0)
    } else {
        (0, input * 2) // Success
    }
}

// Usage
let (err, result) = validated_operation(10);
if (err == 0) {
    // Success - use result
} else {
    // Handle error based on err code
};
```

### State Queries

```move
/// Returns (is_valid, balance, is_frozen)
public fun account_status(addr: address): (bool, u64, bool) {
    if (!account_exists(addr)) {
        return (false, 0, false);
    };
    
    let balance = get_balance(addr);
    let frozen = is_account_frozen(addr);
    
    (true, balance, frozen)
}
```

### Pagination

```move
/// Returns (items, has_more, next_cursor)
public fun get_paginated_items(
    cursor: u64,
    limit: u64
): (vector<Item>, bool, u64) {
    let items = fetch_items(cursor, limit);
    let has_more = vector::length(&items) == limit;
    let next_cursor = cursor + limit;
    
    (items, has_more, next_cursor)
}
```

## Best Practices

### 1. Use Tuples for Related Values

```move
// Good: Related values
public fn get_coordinates(): (u64, u64) {
    (x, y)
}

// Better: Use struct for complex data
struct Point has copy, drop {
    x: u64,
    y: u64,
}
```

### 2. Limit Tuple Size

```move
// Good: Small tuple
let pair: (u64, u64) = (1, 2);

// Okay: Triple
let triple: (u64, u64, u64) = (1, 2, 3);

// Bad: Too many elements - use struct instead
// let many: (u64, u64, u64, u64, u64, u64) = ...;
```

### 3. Document Tuple Contents

```move
/// Returns (success, value)
/// - success: true if operation succeeded
/// - value: result value (meaningless if success is false)
public fun try_operation(): (bool, u64) {
    // Implementation
}
```

### 4. Use Named Variables When Destructuring

```move
// Good: Clear names
let (min_price, max_price) = get_price_range();

// Bad: Unclear
let (a, b) = get_price_range();
```

## Performance Considerations

- Tuples are stack-allocated (very efficient)
- No heap allocation overhead
- Compiler optimizes tuple operations
- Prefer tuples over small structs for simple grouping

## Common Errors

### Type Mismatch

```move
// Wrong: Different types than expected
// let (x, y): (u64, u64) = (10, true); // Error!

// Correct
let (x, y): (u64, bool) = (10, true);
```

### Wrong Number of Elements

```move
// Wrong: Mismatched count
// let (x, y) = (1u64, 2, 3); // Error!

// Correct
let (x, y, z) = (1u64, 2, 3);
```

### Forgetting Unit

```move
// Void function returns unit
public fn do_something() {
    // Returns ()
}

// Can't assign to non-unit
// let x: u64 = do_something(); // Error!

// Correct
let _: () = do_something();
// Or just
do_something();
```

## Next Steps

- Learn about [Structs](structs-and-resources.md) for named fields
- Study [Functions](functions.md) returning multiple values
- Explore [Pattern Matching](conditionals.md) for advanced destructuring
