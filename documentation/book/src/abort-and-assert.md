# Abort and Assert

Error handling in Move uses `abort` to halt execution and `assert!` for validation. This guide covers error handling patterns and best practices.

## Abort Statement

The `abort` statement stops execution and returns an error code:

```move
public fun withdraw(balance: &mut u64, amount: u64) {
    if (amount == 0) {
        abort 0  // Error code 0: Invalid amount
    };
    
    if (*balance < amount) {
        abort 1  // Error code 1: Insufficient balance
    };
    
    *balance -= amount;
}
```

### Error Codes

Error codes are `u64` values. Use constants for clarity:

```move
const E_INVALID_AMOUNT: u64 = 0;
const E_INSUFFICIENT_BALANCE: u64 = 1;
const E_UNAUTHORIZED: u64 = 2;

public fun safe_withdraw(balance: &mut u64, amount: u64) {
    assert!(amount > 0, E_INVALID_AMOUNT);
    assert!(*balance >= amount, E_INSUFFICIENT_BALANCE);
    
    *balance -= amount;
}
```

## Assert Macro

The `assert!` macro combines condition checking with abort:

```move
// These are equivalent:
if (!condition) abort ERROR_CODE;
assert!(condition, ERROR_CODE);
```

### Basic Usage

```move
public fun divide(a: u64, b: u64): u64 {
    assert!(b > 0, E_DIVISION_BY_ZERO);
    a / b
}
```

### Multiple Assertions

```move
public fun transfer_tokens(
    sender: address,
    recipient: address,
    amount: u64
) {
    assert!(sender != @0x0, E_INVALID_SENDER);
    assert!(recipient != @0x0, E_INVALID_RECIPIENT);
    assert!(sender != recipient, E_CANNOT_SEND_TO_SELF);
    assert!(amount > 0, E_INVALID_AMOUNT);
    
    // Transfer logic
}
```

## Error Code Organization

### Module-Level Error Codes

```move
module errors::token {
    // Validation errors (0-99)
    const E_INVALID_AMOUNT: u64 = 0;
    const E_INVALID_ADDRESS: u64 = 1;
    const E_OVERFLOW: u64 = 2;
    const E_UNDERFLOW: u64 = 3;
    
    // Authorization errors (100-199)
    const E_UNAUTHORIZED: u64 = 100;
    const E_NOT_OWNER: u64 = 101;
    const E_CAPABILITY_REQUIRED: u64 = 102;
    
    // State errors (200-299)
    const E_INSUFFICIENT_BALANCE: u64 = 200;
    const E_SUPPLY_EXCEEDED: u64 = 201;
    const E_TOKEN_FROZEN: u64 = 202;
}
```

### Categorized Errors

```move
module errors::categories {
    // Input validation
    const E_INVALID_INPUT: u64 = 0;
    const E_OUT_OF_RANGE: u64 = 1;
    const E_INVALID_FORMAT: u64 = 2;
    
    // Business logic
    const E_INVALID_STATE: u64 = 100;
    const E_CONSTRAINT_VIOLATED: u64 = 101;
    const E_PRECONDITION_FAILED: u64 = 102;
    
    // Access control
    const E_ACCESS_DENIED: u64 = 200;
    const E_ROLE_REQUIRED: u64 = 201;
    const E_PERMISSION_DENIED: u64 = 202;
}
```

## Common Patterns

### Guard Clauses

Use assertions as guard clauses:

```move
public fun update_profile(
    profile: &mut Profile,
    name: vector<u8>,
    age: u64
) {
    // Guard clauses
    assert!(vector::length(&name) > 0, E_EMPTY_NAME);
    assert!(vector::length(&name) <= 100, E_NAME_TOO_LONG);
    assert!(age >= 18, E_UNDERAGE);
    assert!(age <= 150, E_INVALID_AGE);
    
    // Main logic
    profile.name = name;
    profile.age = age;
}
```

### Validation Functions

Extract validation into separate functions:

```move
public fun validate_transfer(
    sender: address,
    recipient: address,
    amount: u64,
    balance: u64
) {
    assert!(sender != @0x0, E_INVALID_SENDER);
    assert!(recipient != @0x0, E_INVALID_RECIPIENT);
    assert!(sender != recipient, E_SAME_ADDRESS);
    assert!(amount > 0, E_ZERO_AMOUNT);
    assert!(balance >= amount, E_INSUFFICIENT_BALANCE);
}

public fun execute_transfer(
    sender: address,
    recipient: address,
    amount: u64,
    balance: &mut u64
) {
    validate_transfer(sender, recipient, amount, *balance);
    
    *balance -= amount;
    // Complete transfer
}
```

### State Validation

Validate state transitions:

```move
enum OrderState has copy, drop, store {
    Pending,
    Confirmed,
    Shipped,
    Delivered,
    Cancelled,
}

public fun confirm_order(order: &mut Order) {
    assert!(matches!(order.state, OrderState::Pending), E_INVALID_STATE);
    order.state = OrderState::Confirmed;
}

public fun ship_order(order: &mut Order) {
    assert!(matches!(order.state, OrderState::Confirmed), E_INVALID_TRANSITION);
    order.state = OrderState::Shipped;
}
```

### Range Checking

```move
public fun set_percentage(value: u64) {
    assert!(value <= 10000, E_PERCENTAGE_OUT_OF_RANGE); // Max 100.00%
    // Process percentage
}

public fun set_timestamp(ts: u64) {
    let current = clock::timestamp_ms();
    assert!(ts >= current, E_PAST_TIMESTAMP);
    assert!(ts <= current + 365 * 86400000, E_TIMESTAMP_TOO_FAR); // Max 1 year
}
```

## Error Handling Strategies

### Early Validation

Validate inputs early:

```move
public fun mint_tokens(
    cap: &MintCap,
    amount: u64,
    recipient: address
): Coin {
    // Validate first
    assert!(amount > 0, E_ZERO_MINT);
    assert!(amount <= MAX_MINT_AMOUNT, E_EXCEEDS_LIMIT);
    assert!(recipient != @0x0, E_INVALID_RECIPIENT);
    
    // Then execute
    coin::mint(cap, amount)
}
```

### Fail Fast

Don't continue after errors:

```move
// Bad: Continue after error
public fn bad_example() {
    if (condition1) abort 0;
    do_something(); // Might use invalid state
    if (condition2) abort 1;
}

// Good: Fail fast
public fn good_example() {
    assert!(condition1, 0);
    assert!(condition2, 1);
    do_something(); // Safe to proceed
}
```

### Defensive Programming

Assume inputs are invalid:

```move
public fun safe_divide(a: u64, b: u64): u64 {
    assert!(b != 0, E_DIVISION_BY_ZERO);
    a / b
}

public fun safe_subtract(a: u64, b: u64): u64 {
    assert!(a >= b, E_UNDERFLOW);
    a - b
}
```

## Testing Error Conditions

### Expected Failure Tests

```move
#[test]
#[expected_failure(abort_code = E_INVALID_AMOUNT)]
fun test_zero_amount() {
    let mut balance = 1000;
    withdraw(&mut balance, 0);
}

#[test]
#[expected_failure(abort_code = E_INSUFFICIENT_BALANCE)]
fun test_insufficient_balance() {
    let mut balance = 100;
    withdraw(&mut balance, 200);
}

#[test]
#[expected_failure]
fun test_any_failure() {
    abort 999;
}
```

### Testing Multiple Error Cases

```move
#[test]
fun test_all_error_cases() {
    let mut balance: u64 = 1000;
    
    // Test zero amount
    #[expected_failure(abort_code = E_INVALID_AMOUNT)]
    fun test_zero() { withdraw(&mut balance, 0); }
    test_zero();
    
    // Test insufficient balance
    #[expected_failure(abort_code = E_INSUFFICIENT_BALANCE)]
    fun test_insufficient() { withdraw(&mut balance, 2000); }
    test_insufficient();
    
    // Test valid withdrawal
    withdraw(&mut balance, 500);
    assert!(balance == 500, 0);
}
```

## Best Practices

### 1. Use Descriptive Error Constants

```move
// Bad
assert!(x > 0, 0);
assert!(y < 100, 1);

// Good
assert!(x > 0, E_INVALID_VALUE);
assert!(y < 100, E_VALUE_OUT_OF_RANGE);
```

### 2. Group Related Errors

```move
// Token errors
const E_TOKEN_NOT_FOUND: u64 = 0;
const E_TOKEN_ALREADY_EXISTS: u64 = 1;
const E_TOKEN_FROZEN: u64 = 2;

// Balance errors
const E_INSUFFICIENT_BALANCE: u64 = 100;
const E_BALANCE_OVERFLOW: u64 = 101;
```

### 3. Provide Clear Error Messages (in comments)

```move
/// Withdraws tokens from balance
/// 
/// Aborts with:
/// - E_INVALID_AMOUNT if amount is zero
/// - E_INSUFFICIENT_BALANCE if balance < amount
public fun withdraw(balance: &mut u64, amount: u64) {
    assert!(amount > 0, E_INVALID_AMOUNT);
    assert!(*balance >= amount, E_INSUFFICIENT_BALANCE);
    *balance -= amount;
}
```

### 4. Validate All Inputs

```move
public fun complex_operation(
    addr: address,
    amount: u64,
    timestamp: u64,
    data: vector<u8>
) {
    // Validate all inputs
    assert!(addr != @0x0, E_INVALID_ADDRESS);
    assert!(amount > 0, E_INVALID_AMOUNT);
    assert!(timestamp > 0, E_INVALID_TIMESTAMP);
    assert!(vector::length(&data) > 0, E_EMPTY_DATA);
    assert!(vector::length(&data) <= MAX_DATA_SIZE, E_DATA_TOO_LARGE);
    
    // Execute operation
}
```

### 5. Use Specific Error Codes

```move
// Bad: Generic error
assert!(condition, E_ERROR);

// Good: Specific error
assert!(balance >= amount, E_INSUFFICIENT_BALANCE);
assert!(is_owner, E_NOT_OWNER);
assert!(!is_frozen, E_ACCOUNT_FROZEN);
```

## Common Errors

### Inconsistent Error Codes

```move
// Bad: Reusing error codes for different purposes
const E_ERROR: u64 = 0; // Used for multiple things

// Good: Unique codes
const E_INVALID_AMOUNT: u64 = 0;
const E_INVALID_ADDRESS: u64 = 1;
```

### Missing Assertions

```move
// Bad: No validation
public fn unsafe_divide(a: u64, b: u64): u64 {
    a / b // Panics if b == 0
}

// Good: Validated
public fn safe_divide(a: u64, b: u64): u64 {
    assert!(b > 0, E_DIVISION_BY_ZERO);
    a / b
}
```

### Silent Failures

```move
// Bad: Ignoring errors
public fn silent_fail() {
    if (condition) {
        // Do nothing on error
    };
}

// Good: Explicit failure
public fn explicit_fail() {
    assert!(condition, E_CONDITION_NOT_MET);
}
```

## Performance Considerations

- Assertions add minimal overhead
- Failed assertions consume gas up to the failure point
- Validate cheap conditions first
- Avoid expensive calculations before validation

```move
// Good: Cheap check first
assert!(amount > 0, E_INVALID_AMOUNT);
assert!(expensive_verification(), E_UNAUTHORIZED);

// Bad: Expensive check first
assert!(expensive_verification(), E_UNAUTHORIZED);
assert!(amount > 0, E_INVALID_AMOUNT);
```

## Next Steps

- Learn about [Testing](unit-testing.md) error conditions
- Study [Security Patterns](coding-conventions.md)
- Explore [Error Recovery](usage-examples.md)
