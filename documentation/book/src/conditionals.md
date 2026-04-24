# Conditionals

Conditionals allow your program to make decisions and execute different code paths based on conditions.

## If-Else Expressions

### Basic If

```move
let x: u64 = 10;

if (x > 5) {
    // Executed when condition is true
};
```

### If-Else

```move
let balance: u64 = 1000;
let withdrawal: u64 = 500;

if (balance >= withdrawal) {
    // Sufficient balance
    balance = balance - withdrawal;
} else {
    // Insufficient balance
    abort 0
};
```

### If-Else-If Chain

```move
let score: u64 = 85;
let mut grade: vector<u8> = b"F";

if (score >= 90) {
    grade = b"A";
} else if (score >= 80) {
    grade = b"B";
} else if (score >= 70) {
    grade = b"C";
} else if (score >= 60) {
    grade = b"D";
} else {
    grade = b"F";
};
```

## If as Expression

In Move, `if` is an expression that returns a value:

```move
let x: u64 = 10;
let y: u64 = 20;

// If expression returns a value
let max_val = if (x > y) { x } else { y };
assert!(max_val == 20, 0);

// Can be used in function calls
process_value(if (condition) { 100 } else { 200 });
```

### All Branches Must Return Same Type

```move
// Valid: Both branches return u64
let result: u64 = if (x > 0) { 100 } else { 200 };

// Invalid: Different types
// let bad = if (x > 0) { 100 } else { true }; // Error!
```

## Pattern Matching with Match

Move supports pattern matching (in newer versions):

```move
enum Status has copy, drop {
    Pending,
    Active,
    Completed,
    Failed,
}

let status = Status::Active;

let message = match (status) {
    Status::Pending => b"Waiting",
    Status::Active => b"In progress",
    Status::Completed => b"Done",
    Status::Failed => b"Error",
};
```

## Common Patterns

### Guard Clauses

Use early returns to reduce nesting:

```move
public fun withdraw(balance: &mut u64, amount: u64): bool {
    // Guard clauses
    if (amount == 0) return false;
    if (*balance < amount) return false;
    if (amount > 10000) return false;
    
    // Main logic (less nested)
    *balance -= amount;
    true
}
```

### Validation Chain

```move
public fun validate_transfer(
    sender: address,
    recipient: address,
    amount: u64
): bool {
    if (sender == @0x0) return false;
    if (recipient == @0x0) return false;
    if (sender == recipient) return false;
    if (amount == 0) return false;
    
    true
}
```

### Conditional Assignment

```move
let is_weekend = day_of_week == 6 || day_of_week == 7;
let multiplier = if (is_weekend) { 2 } else { 1 };
```

### Min/Max Functions

```move
public fun min(a: u64, b: u64): u64 {
    if (a < b) { a } else { b }
}

public fun max(a: u64, b: u64): u64 {
    if (a > b) { a } else { b }
}

// Usage
let smallest = min(10, 20); // 10
let largest = max(10, 20);  // 20
```

## Boolean Logic in Conditionals

### AND Operator

```move
if (age >= 18 && has_id) {
    // Both conditions must be true
    can_enter = true;
};
```

### OR Operator

```move
if (is_admin || has_permission) {
    // At least one condition must be true
    can_access = true;
};
```

### NOT Operator

```move
if (!is_blocked) {
    // Condition is true when is_blocked is false
    can_proceed = true;
};
```

### Complex Conditions

```move
if (balance >= amount && 
    amount > 0 && 
    !is_frozen && 
    (is_verified || amount < 1000)) {
    // Complex validation
    execute_transfer();
};
```

## Short-Circuit Evaluation

Move uses short-circuit evaluation:

```move
// If first condition is false, second is NOT evaluated
if (is_valid && expensive_check()) {
    // ...
};

// If first condition is true, second is NOT evaluated
if (has_access || is_admin()) {
    // ...
};
```

## Nested Conditionals

Avoid deep nesting when possible:

```move
// Bad: Deep nesting
if (condition1) {
    if (condition2) {
        if (condition3) {
            // Deeply nested
        };
    };
};

// Good: Flat structure with guards
if (!condition1) return;
if (!condition2) return;
if (!condition3) return;
// Main logic at top level
```

## Common Use Cases

### Range Checking

```move
public fun is_in_range(value: u64, min: u64, max: u64): bool {
    if (value < min) return false;
    if (value > max) return false;
    true
}

// Usage
if (is_in_range(score, 0, 100)) {
    // Valid score
};
```

### Type Checking (with Enums)

```move
enum PaymentMethod has copy, drop {
    Cash,
    CreditCard,
    DebitCard,
    Crypto,
}

public fun get_fee(method: PaymentMethod): u64 {
    if (matches!(method, PaymentMethod::Cash)) {
        0
    } else if (matches!(method, PaymentMethod::CreditCard)) {
        30 // 0.3%
    } else if (matches!(method, PaymentMethod::DebitCard)) {
        10 // 0.1%
    } else {
        5 // 0.05% for crypto
    }
}
```

### State Machine Transitions

```move
enum OrderState has copy, drop, store {
    Created,
    Paid,
    Shipped,
    Delivered,
    Cancelled,
}

public fun transition_order(
    current: OrderState,
    event: vector<u8>
): OrderState {
    if (matches!(current, OrderState::Created)) {
        if (event == b"pay") {
            return OrderState::Paid;
        } else if (event == b"cancel") {
            return OrderState::Cancelled;
        };
    } else if (matches!(current, OrderState::Paid)) {
        if (event == b"ship") {
            return OrderState::Shipped;
        };
    } else if (matches!(current, OrderState::Shipped)) {
        if (event == b"deliver") {
            return OrderState::Delivered;
        };
    };
    
    current // No valid transition
}
```

### Optional Values

```move
use std::option;

public fun get_balance_or_default(opt_balance: Option<u64>): u64 {
    if (option::is_some(&opt_balance)) {
        *option::borrow(&opt_balance)
    } else {
        0 // Default value
    }
}
```

## Testing Conditionals

```move
#[test]
fun test_if_expression() {
    let x: u64 = 10;
    let y: u64 = 20;
    
    let max_val = if (x > y) { x } else { y };
    assert!(max_val == 20, 0);
    
    let min_val = if (x < y) { x } else { y };
    assert!(min_val == 10, 1);
}

#[test]
fun test_guard_clauses() {
    let mut balance: u64 = 1000;
    
    // Valid withdrawal
    let success = withdraw(&mut balance, 500);
    assert!(success == true, 0);
    assert!(balance == 500, 1);
    
    // Invalid: insufficient balance
    let fail = withdraw(&mut balance, 600);
    assert!(fail == false, 2);
    assert!(balance == 500, 3); // Unchanged
    
    // Invalid: zero amount
    let fail2 = withdraw(&mut balance, 0);
    assert!(fail2 == false, 4);
}

#[test]
fun test_range_checking() {
    assert!(is_in_range(50, 0, 100) == true, 0);
    assert!(is_in_range(0, 0, 100) == true, 1);
    assert!(is_in_range(100, 0, 100) == true, 2);
    assert!(is_in_range(101, 0, 100) == false, 3);
    assert!(is_in_range(999, 0, 100) == false, 4);
}
```

## Best Practices

### 1. Keep Conditions Simple

```move
// Bad: Complex inline condition
if (x > 0 && y > 0 && z > 0 && x + y > z && y + z > x) { }

// Good: Extract to function
if (is_valid_triangle(x, y, z)) { }
```

### 2. Use Early Returns

```move
// Bad: Nested ifs
public fn bad_example(x: u64) {
    if (x > 0) {
        if (x < 100) {
            // Process
        };
    };
}

// Good: Guard clauses
public fn good_example(x: u64) {
    if (x == 0) return;
    if (x >= 100) return;
    // Process
}
```

### 3. Avoid Magic Numbers

```move
// Bad
if (status == 1) { }
if (status == 2) { }

// Good
const STATUS_ACTIVE: u8 = 1;
const STATUS_INACTIVE: u8 = 2;

if (status == STATUS_ACTIVE) { }
if (status == STATUS_INACTIVE) { }
```

### 4. Document Complex Logic

```move
/// Check if user can withdraw based on multiple criteria:
/// - Account not frozen
/// - Daily limit not exceeded
/// - KYC verified for large amounts
public fun can_withdraw(user: &User, amount: u64): bool {
    if (user.is_frozen) return false;
    if (user.daily_withdrawn + amount > user.daily_limit) return false;
    if (amount > KYC_THRESHOLD && !user.kyc_verified) return false;
    
    true
}
```

## Performance Considerations

- Conditionals are very fast (single CPU branch)
- Order conditions by likelihood (check most common first)
- Short-circuit evaluation prevents unnecessary computation
- Avoid complex calculations in conditions

## Common Errors

### Missing Semicolon

```move
// Wrong
if (condition) {
    // code
} // Missing semicolon

// Correct
if (condition) {
    // code
};
```

### Type Mismatch in Branches

```move
// Wrong: Different return types
// let x = if (cond) { 10 } else { true }; // Error!

// Correct: Same types
let x: u64 = if (cond) { 10 } else { 20 };
```

### Forgetting Else Branch

```move
// If using as expression, all paths must return value
// let x = if (cond) { 10 }; // Error: missing else

let x = if (cond) { 10 } else { 0 }; // OK
```

## Next Steps

- Learn about [Loops](loops.md) for iteration
- Study [Pattern Matching](usage-examples.md) for advanced branching
- Explore [Error Handling](abort-and-assert.md)
