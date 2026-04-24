# Functions

Functions are the building blocks of Move programs. They encapsulate reusable logic and define the interface of modules.

## Function Declaration

```move
// Simple function
public fun add(a: u64, b: u64): u64 {
    a + b
}

// Function with explicit return
public fun multiply(x: u64, y: u64): u64 {
    let result = x * y;
    result
}

// Void function (returns unit ())
public fun log_message(msg: vector<u8>) {
    // Side effects only
}
```

## Visibility Modifiers

### Public Functions

Accessible from anywhere:

```move
public fun public_function() {
    // Can be called by anyone
}
```

### Public Entry Functions

Can be called directly from transactions:

```move
public entry fun transfer_tokens(
    coins: Coin<KANARI>,
    recipient: address
) {
    transfer::public_transfer(coins, recipient);
}
```

### Private Functions

Only accessible within the same module:

```move
fun private_helper() {
    // Internal implementation
}

// Or explicitly:
private fun another_private() {
    // Same as above
}
```

## Parameters

### Immutable Parameters

```move
public fun read_only(x: u64, y: u64): u64 {
    // Cannot modify x or y
    // x = 10; // Error
    x + y
}
```

### Mutable Parameters

```move
public fun increment(mut counter: u64): u64 {
    counter = counter + 1;
    counter
}

public fun update_balance(balance: &mut u64, amount: u64) {
    *balance = *balance + amount;
}
```

### Multiple Parameters

```move
public fun complex_function(
    addr: address,
    amount: u64,
    metadata: vector<u8>,
    flag: bool
): bool {
    // Function body
    true
}
```

## Return Values

### Single Return

```move
public fun get_value(): u64 {
    42
}
```

### Multiple Returns (Tuples)

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
```

### Early Return

```move
public fun validate_and_process(amount: u64): bool {
    if (amount == 0) {
        return false;
    };
    
    if (amount > 1000) {
        return false;
    };
    
    // Process...
    true
}
```

## Generics

Functions can work with generic types:

```move
public fun identity<T>(x: T): T {
    x
}

public fun swap<T>(a: &mut T, b: &mut T) {
    let temp = *a;
    *a = *b;
    *b = temp;
}

// Constrained generics
public fun copy_value<T: copy>(x: T): T {
    x // Can copy because of constraint
}
```

## Recursive Functions

```move
public fun factorial(n: u64): u64 {
    if (n <= 1) {
        1
    } else {
        n * factorial(n - 1)
    }
}

public fun fibonacci(n: u64): u64 {
    if (n == 0) {
        0
    } else if (n == 1) {
        1
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}
```

## Common Patterns

### Guard Clauses

```move
public fun withdraw(balance: &mut u64, amount: u64): bool {
    // Guard clauses
    if (amount == 0) return false;
    if (*balance < amount) return false;
    
    *balance = *balance - amount;
    true
}
```

### Builder Pattern

```move
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

public fun set_amount(builder: &mut TransactionBuilder, amount: u64) {
    builder.amount = amount;
}

public fun execute(builder: &TransactionBuilder) {
    assert!(builder.recipient != @0x0, 0);
    assert!(builder.amount > 0, 1);
    // Execute transaction
}
```

### Callback Pattern

```move
public fun process_items<T>(
    items: &vector<T>,
    processor: |&T| -> bool
): u64 {
    let mut count = 0;
    let len = vector::length(items);
    let mut i = 0;
    
    while (i < len) {
        let item = vector::borrow(items, i);
        if (processor(item)) {
            count = count + 1;
        };
        i = i + 1;
    };
    
    count
}
```

## Function Composition

```move
public fun square(x: u64): u64 {
    x * x
}

public fun double(x: u64): u64 {
    x * 2
}

// Compose functions
public fun square_and_double(x: u64): u64 {
    double(square(x))
}
```

## Testing Functions

```move
#[test]
fun test_addition() {
    assert!(add(2, 3) == 5, 0);
    assert!(add(0, 0) == 0, 1);
    assert!(add(100, 200) == 300, 2);
}

#[test]
fun test_min_max() {
    let (min, max) = min_max(10, 20);
    assert!(min == 10, 0);
    assert!(max == 20, 1);
    
    let (min2, max2) = min_max(30, 15);
    assert!(min2 == 15, 2);
    assert!(max2 == 30, 3);
}

#[test]
#[expected_failure]
fun test_division_by_zero() {
    divide(10, 0); // Should fail
}
```

## Best Practices

### 1. Keep Functions Small

```move
// Bad: Too many responsibilities
public fun bad_example() {
    // Validate
    // Calculate
    // Update state
    // Emit event
    // Return result
}

// Good: Separated concerns
public fun good_example() {
    validate_input();
    let result = calculate();
    update_state(result);
    emit_event(result);
    result
}
```

### 2. Use Descriptive Names

```move
// Bad
pub fn proc() { }

// Good
public fun process_transaction() { }
```

### 3. Document Public Functions

```move
/// Transfer tokens from sender to recipient
/// 
/// # Arguments
/// * `coins` - The coins to transfer
/// * `recipient` - The recipient address
/// 
/// # Panics
/// * If recipient is zero address
public entry fun transfer_tokens(
    coins: Coin<KANARI>,
    recipient: address
) {
    assert!(recipient != @0x0, E_INVALID_RECIPIENT);
    transfer::public_transfer(coins, recipient);
}
```

### 4. Minimize Side Effects

```move
// Pure function (no side effects)
public fun calculate_fee(amount: u64, rate: u64): u64 {
    amount * rate / 10000
}

// Function with side effects (clearly marked)
public entry fun update_and_emit(
    balance: &mut u64,
    amount: u64
) {
    *balance += amount;
    event::emit(BalanceUpdate { new_balance: *balance });
}
```

## Performance Considerations

### Inline Simple Functions

```move
// Compiler may inline this
public fun is_zero(x: u64): bool {
    x == 0
}
```

### Avoid Unnecessary Copies

```move
// Pass by reference for large structs
public fun process_large_data(data: &LargeStruct) {
    // Read without copying
}

// Pass by value only when necessary
public fun consume_resource(resource: Resource) {
    // Takes ownership
}
```

## Common Errors

### Missing Return Type

```move
// Bad: Unclear return type
public fun ambiguous() {
    10
}

// Good: Explicit return type
public fun clear(): u64 {
    10
}
```

### Unused Parameters

```move
// Warning: unused parameter 'y'
public fun unused_param(x: u64, y: u64): u64 {
    x
}

// Fix: Use underscore prefix
public fun no_warning(x: u64, _y: u64): u64 {
    x
}
```

## Next Steps

- Learn about [Generics](generics.md) for type flexibility
- Study [Modules](modules-and-scripts.md) for organization
- Explore [Error Handling](abort-and-assert.md)
