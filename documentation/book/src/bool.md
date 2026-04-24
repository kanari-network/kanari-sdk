# Bool

The `bool` type represents boolean values in Move: `true` or `false`. It's fundamental for control flow and conditional logic.

## Boolean Literals

```move
let is_valid: bool = true;
let is_empty: bool = false;
```

## Comparison Operators

All comparison operators return `bool`:

```move
let a: u64 = 10;
let b: u64 = 20;

// Equality
let eq: bool = a == b; // false

// Inequality
let neq: bool = a != b; // true

// Less than
let lt: bool = a < b; // true

// Greater than
let gt: bool = a > b; // false

// Less than or equal
let lte: bool = a <= b; // true

// Greater than or equal
let gte: bool = a >= b; // false
```

## Logical Operators

```move
let x: bool = true;
let y: bool = false;

// AND - both must be true
let and_result = x && y; // false

// OR - at least one must be true
let or_result = x || y; // true

// NOT - negates the value
let not_result = !x; // false
```

## Short-Circuit Evaluation

Move uses short-circuit evaluation for logical operators:

```move
// If first condition is false, second is not evaluated
if (is_valid && check_expensive_operation()) {
    // ...
}

// If first condition is true, second is not evaluated
if (has_permission || is_admin()) {
    // ...
}
```

## Conditional Statements

### If-Else

```move
let balance: u64 = 1000;

if (balance >= 100) {
    // Sufficient balance
} else {
    // Insufficient balance
};
```

### If-Else-If Chain

```move
let score: u64 = 85;

if (score >= 90) {
    // Grade A
} else if (score >= 80) {
    // Grade B
} else if (score >= 70) {
    // Grade C
} else {
    // Grade F
};
```

### Ternary-like Pattern

Move doesn't have ternary operator, but you can use if-expressions:

```move
let status = if (is_active) { b"active" } else { b"inactive" };
```

## Assertions

Use `assert!` to enforce conditions:

```move
const E_INSUFFICIENT_BALANCE: u64 = 0;
const E_INVALID_AMOUNT: u64 = 1;

public fun transfer(amount: u64, balance: u64) {
    assert!(amount > 0, E_INVALID_AMOUNT);
    assert!(balance >= amount, E_INSUFFICIENT_BALANCE);
    
    // Transfer logic
}
```

## Common Patterns

### Validation Functions

```move
/// Check if an address is valid (not zero address)
public fun is_valid_address(addr: address): bool {
    addr != @0x0
}

/// Check if amount is valid (non-zero)
public fun is_valid_amount(amount: u64): bool {
    amount > 0
}

/// Check if timestamp is in the future
public fun is_future_timestamp(ts: u64, current: u64): bool {
    ts > current
}
```

### Flag Management

```move
struct AccountFlags has copy, drop, store {
    is_frozen: bool,
    is_verified: bool,
    has_kyc: bool,
}

public fun can_withdraw(flags: &AccountFlags): bool {
    !flags.is_frozen && flags.is_verified
}

public fun set_frozen(flags: &mut AccountFlags, frozen: bool) {
    flags.is_frozen = frozen;
}
```

### Option Checking

```move
use std::option;

let opt: Option<u64> = option::some(100);

if (option::is_some(&opt)) {
    let value = option::borrow(&opt);
    // Use value
} else {
    // Handle None case
};
```

## Boolean in Structs

```move
struct Permission has copy, drop, store {
    can_read: bool,
    can_write: bool,
    can_delete: bool,
    can_admin: bool,
}

public fun has_full_access(perm: &Permission): bool {
    perm.can_read && perm.can_write && perm.can_delete && perm.can_admin
}

public fun is_read_only(perm: &Permission): bool {
    perm.can_read && !perm.can_write && !perm.can_delete && !perm.can_admin
}
```

## Testing Boolean Logic

```move
#[test]
fun test_boolean_operations() {
    // AND truth table
    assert!(true && true, 0);
    assert!(!(true && false), 1);
    assert!(!(false && true), 2);
    assert!(!(false && false), 3);
    
    // OR truth table
    assert!(true || true, 4);
    assert!(true || false, 5);
    assert!(false || true, 6);
    assert!(!(false || false), 7);
    
    // NOT
    assert!(!true == false, 8);
    assert!(!false == true, 9);
}

#[test]
fun test_assertions() {
    assert!(10 > 5, 0);
    assert!(10 >= 10, 1);
    assert!(5 < 10, 2);
    assert!(5 <= 5, 3);
    assert!(5 == 5, 4);
    assert!(5 != 6, 5);
}

#[test]
#[expected_failure(abort_code = 0)]
fun test_assertion_failure() {
    assert!(false, 0); // This will fail
}
```

## De Morgan's Laws

Remember these useful transformations:

```move
// !(A && B) == !A || !B
assert!(!(true && false) == (!true || !false), 0);

// !(A || B) == !A && !B
assert!(!(true || false) == (!true && !false), 1);
```

## Common Mistakes

### Assignment vs Comparison

```move
// Wrong: This is assignment, not comparison
// if (x = 5) { }

// Correct: Use == for comparison
if (x == 5) { }
```

### Missing Semicolon After If-Else

```move
// Wrong: Missing semicolon
if (condition) {
    // ...
} else {
    // ...
} // <- Missing semicolon

// Correct
if (condition) {
    // ...
} else {
    // ...
}; // <- Add semicolon
```

### Chaining Comparisons

```move
// Wrong: Can't chain comparisons like Python
// if (a < b < c) { }

// Correct: Use logical AND
if (a < b && b < c) { }
```

## Best Practices

1. **Use descriptive names**: `is_valid`, `has_permission`, `can_execute`
2. **Keep conditions simple**: Break complex conditions into helper functions
3. **Use early returns**: Reduce nesting with guard clauses
4. **Document error codes**: Explain what each assertion checks
5. **Test edge cases**: Verify boolean logic with unit tests

## Performance Considerations

- Boolean operations are very fast (single CPU instruction)
- Short-circuit evaluation prevents unnecessary computation
- Prefer `&&` over nested `if` statements when appropriate
- Avoid redundant boolean checks

## Next Steps

- Learn about [Conditionals](conditionals.md) for control flow
- Study [Abort and Assert](abort-and-assert.md) for error handling
- Explore [Loops](loops.md) for iteration
