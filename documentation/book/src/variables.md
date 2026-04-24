# Variables

Variables in Move are used to store values temporarily during function execution. They follow strict scoping and ownership rules.

## Variable Declaration

Use the `let` keyword to declare variables:

```move
// Immutable variable
let x: u64 = 10;

// Mutable variable
let mut y: u64 = 20;

// Type inference (Move can infer types)
let z = 30; // Inferred as u64 based on context
```

## Variable Assignment

```move
let mut x: u64 = 10;

// Reassignment
x = 20;

// Multiple assignments
let mut a: u64 = 0;
let mut b: u64 = 0;
a = 10;
b = 20;
```

## Shadowing

You can shadow variables by redeclaring them with the same name:

```move
let x: u64 = 10;
let x: u64 = 20; // Shadows previous x

let y: u64 = 30;
let y: bool = true; // Can even change type when shadowing
```

## Scope

Variables are only accessible within their scope (between `{` and `}`):

```move
{
    let x: u64 = 10;
    // x is accessible here
};
// x is NOT accessible here - it's been dropped

{
    let y: u64 = 20;
    // y is accessible here
    {
        // y is also accessible in nested scopes
        let z: u64 = 30;
        // z is accessible here
    };
    // z is NOT accessible here
    // y is still accessible
};
```

## Destructuring

Destructure tuples and structs:

```move
// Tuple destructuring
let pair: (u64, u64) = (10, 20);
let (x, y) = pair;
assert!(x == 10, 0);
assert!(y == 20, 1);

// Struct destructuring
struct Point has copy, drop {
    x: u64,
    y: u64,
}

let p = Point { x: 100, y: 200 };
let Point { x, y } = p;
assert!(x == 100, 0);
assert!(y == 200, 1);

// Partial destructuring
let Point { x, .. } = p; // Only extract x, ignore y
```

## Mutable References

```move
let mut x: u64 = 10;
let x_ref = &mut x;
*x_ref = 20; // Modify through reference
assert!(x == 20, 0);
```

## Common Patterns

### Swapping Values

```move
public fun swap(a: &mut u64, b: &mut u64) {
    let temp = *a;
    *a = *b;
    *b = temp;
}
```

### Accumulator Pattern

```move
public fun sum(numbers: &vector<u64>): u64 {
    let mut total: u64 = 0;
    let len = vector::length(numbers);
    let mut i = 0;
    
    while (i < len) {
        total = total + *vector::borrow(numbers, i);
        i = i + 1;
    };
    
    total
}
```

### Counter Pattern

```move
public fun count_items(items: &vector<u8>): u64 {
    let mut count: u64 = 0;
    let len = vector::length(items);
    let mut i = 0;
    
    while (i < len) {
        if (*vector::borrow(items, i) > 0) {
            count = count + 1;
        };
        i = i + 1;
    };
    
    count
}
```

## Ownership and Move Semantics

Move types have ownership semantics:

```move
struct Resource has key, store {
    id: UID,
    value: u64,
}

public fun ownership_example(ctx: &mut TxContext) {
    let r1 = Resource { id: object::new(ctx), value: 100 };
    
    // Move r1 to r2 (r1 is no longer valid)
    let r2 = r1;
    
    // Error: r1 has been moved
    // let x = r1.value;
    
    // r2 is now the owner
    assert!(r2.value == 100, 0);
}
```

## Copy vs Move

Types with `copy` ability can be copied:

```move
#[derive(copy, drop)]
struct Copyable {
    value: u64,
}

let c1 = Copyable { value: 10 };
let c2 = c1; // Copy (c1 still valid)
let c3 = c1; // Can copy again

assert!(c1.value == 10, 0);
assert!(c2.value == 10, 1);
assert!(c3.value == 10, 2);
```

## Function Parameters as Variables

```move
public fun example_param(x: u64, y: u64): u64 {
    // Parameters are immutable by default
    // x = 20; // Error: cannot assign to immutable parameter
    
    let mut result = x + y;
    result
}

public fun mutable_param(mut x: u64): u64 {
    // Mutable parameter
    x = x + 10;
    x
}
```

## Return Values

```move
public fun multiple_returns(): (u64, u64, bool) {
    let x: u64 = 10;
    let y: u64 = 20;
    let success: bool = true;
    
    (x, y, success)
}

public fun use_multiple_returns() {
    let (a, b, ok) = multiple_returns();
    assert!(a == 10, 0);
    assert!(b == 20, 1);
    assert!(ok == true, 2);
}
```

## Best Practices

### 1. Use Descriptive Names

```move
// Bad
let x = 100;
let y = 200;

// Good
let token_amount: u64 = 100;
let recipient_balance: u64 = 200;
```

### 2. Initialize Before Use

```move
// Bad: Using uninitialized variable
// let x: u64;
// let y = x + 10; // Error

// Good: Initialize first
let x: u64 = 0;
let y = x + 10;
```

### 3. Minimize Mutable State

```move
// Prefer immutable when possible
let config_value: u64 = 100;

// Only use mut when necessary
let mut counter: u64 = 0;
counter = counter + 1;
```

### 4. Clean Up Resources

```move
public fun cleanup_example(table: Table<u64, address>) {
    // Use table...
    
    // Destroy when done
    table::destroy_empty(table);
}
```

## Common Errors

### Unused Variable Warning

```move
public fun unused_warning() {
    let x: u64 = 10; // Warning: unused variable
    // Use underscore prefix to suppress warning
    let _x: u64 = 10; // OK: explicitly unused
}
```

### Cannot Assign to Immutable

```move
let x: u64 = 10;
// x = 20; // Error: x is immutable

let mut y: u64 = 10;
y = 20; // OK: y is mutable
```

### Use After Move

```move
struct NoCopy {
    value: u64,
}

let r1 = NoCopy { value: 10 };
let r2 = r1; // r1 moved to r2
// let x = r1.value; // Error: use after move
```

## Testing Variables

```move
#[test]
fun test_variable_basics() {
    // Test declaration
    let x: u64 = 10;
    assert!(x == 10, 0);
    
    // Test mutation
    let mut y: u64 = 20;
    y = 30;
    assert!(y == 30, 1);
    
    // Test shadowing
    let z: u64 = 40;
    let z: bool = true;
    assert!(z == true, 2);
    
    // Test destructuring
    let (a, b) = (100, 200);
    assert!(a == 100, 3);
    assert!(b == 200, 4);
}
```

## Next Steps

- Learn about [References](references.md) for borrowing
- Study [Structs](structs-and-resources.md) for complex data
- Explore [Generics](generics.md) for type flexibility
