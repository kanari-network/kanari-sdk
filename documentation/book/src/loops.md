# Loops

Loops allow you to execute code repeatedly. Move provides `while` and `loop` constructs for iteration.

## While Loop

The most common loop construct:

```move
let mut i: u64 = 0;

while (i < 10) {
    // Execute while condition is true
    i = i + 1;
};
```

### Sum Example

```move
public fun sum_up_to(n: u64): u64 {
    let mut sum: u64 = 0;
    let mut i: u64 = 1;
    
    while (i <= n) {
        sum = sum + i;
        i = i + 1;
    };
    
    sum
}

// Usage
let total = sum_up_to(100); // 5050
```

### Vector Iteration

```move
public fun sum_vector(numbers: &vector<u64>): u64 {
    let mut sum: u64 = 0;
    let len = vector::length(numbers);
    let mut i: u64 = 0;
    
    while (i < len) {
        sum = sum + *vector::borrow(numbers, i);
        i = i + 1;
    };
    
    sum
}
```

## Loop with Break

Use `break` to exit early:

```move
public fun find_index(items: &vector<u64>, target: u64): u64 {
    let len = vector::length(items);
    let mut i: u64 = 0;
    let mut found: u64 = 0;
    let mut is_found: bool = false;
    
    while (i < len && !is_found) {
        if (*vector::borrow(items, i) == target) {
            found = i;
            is_found = true;
        };
        i = i + 1;
    };
    
    if (is_found) { found } else { 0xFFFFFFFFFFFFFFFF }
}
```

## Infinite Loop with Break

```move
public fun wait_for_condition(): u64 {
    let mut attempts: u64 = 0;
    
    loop {
        if (check_condition()) {
            break attempts;
        };
        
        attempts = attempts + 1;
        
        // Prevent infinite loops
        assert!(attempts < 1000, 0);
    }
}
```

## Continue Statement

Skip to next iteration:

```move
public fun sum_even_numbers(numbers: &vector<u64>): u64 {
    let mut sum: u64 = 0;
    let len = vector::length(numbers);
    let mut i: u64 = 0;
    
    while (i < len) {
        let num = *vector::borrow(numbers, i);
        
        // Skip odd numbers
        if (num % 2 != 0) {
            i = i + 1;
            continue;
        };
        
        sum = sum + num;
        i = i + 1;
    };
    
    sum
}
```

## Common Patterns

### Counter Pattern

```move
public fun count_items(items: &vector<u8>, target: u8): u64 {
    let mut count: u64 = 0;
    let len = vector::length(items);
    let mut i: u64 = 0;
    
    while (i < len) {
        if (*vector::borrow(items, i) == target) {
            count = count + 1;
        };
        i = i + 1;
    };
    
    count
}
```

### Accumulator Pattern

```move
public fun product(numbers: &vector<u64>): u64 {
    let mut result: u64 = 1;
    let len = vector::length(numbers);
    let mut i: u64 = 0;
    
    while (i < len) {
        result = result * *vector::borrow(numbers, i);
        i = i + 1;
    };
    
    result
}
```

### Find Maximum

```move
public fun find_max(numbers: &vector<u64>): u64 {
    let len = vector::length(numbers);
    assert!(len > 0, 0);
    
    let mut max_val = *vector::borrow(numbers, 0);
    let mut i: u64 = 1;
    
    while (i < len) {
        let current = *vector::borrow(numbers, i);
        if (current > max_val) {
            max_val = current;
        };
        i = i + 1;
    };
    
    max_val
}
```

### Filter Pattern

```move
public fun filter_positive(numbers: &vector<i64>): vector<u64> {
    let mut result = vector::empty<u64>();
    let len = vector::length(numbers);
    let mut i: u64 = 0;
    
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

### Map Pattern

```move
public fun double_all(numbers: &vector<u64>): vector<u64> {
    let mut result = vector::empty<u64>();
    let len = vector::length(numbers);
    let mut i: u64 = 0;
    
    while (i < len) {
        let num = *vector::borrow(numbers, i);
        vector::push_back(&mut result, num * 2);
        i = i + 1;
    };
    
    result
}
```

## Nested Loops

```move
public fun multiplication_table(size: u64): vector<vector<u64>> {
    let mut table = vector::empty<vector<u64>>();
    let mut i: u64 = 1;
    
    while (i <= size) {
        let mut row = vector::empty<u64>();
        let mut j: u64 = 1;
        
        while (j <= size) {
            vector::push_back(&mut row, i * j);
            j = j + 1;
        };
        
        vector::push_back(&mut table, row);
        i = i + 1;
    };
    
    table
}
```

## Loop with Index and Value

```move
public fun enumerate(items: &vector<u8>): vector<(u64, u8)> {
    let mut result = vector::empty<(u64, u8)>();
    let len = vector::length(items);
    let mut i: u64 = 0;
    
    while (i < len) {
        let value = *vector::borrow(items, i);
        vector::push_back(&mut result, (i, value));
        i = i + 1;
    };
    
    result
}
```

## Common Use Cases

### Retry Logic

```move
public fun retry_operation(max_attempts: u64): bool {
    let mut attempts: u64 = 0;
    
    while (attempts < max_attempts) {
        if (try_operation()) {
            return true;
        };
        
        attempts = attempts + 1;
    };
    
    false
}
```

### Batch Processing

```move
public fun process_in_batches(
    items: &vector<Transaction>,
    batch_size: u64
) {
    let total = vector::length(items);
    let mut processed: u64 = 0;
    
    while (processed < total) {
        let batch_end = min(processed + batch_size, total);
        process_batch(items, processed, batch_end);
        processed = batch_end;
    };
}
```

### Validation Loop

```move
public fun validate_all(items: &vector<Address>): bool {
    let len = vector::length(items);
    let mut i: u64 = 0;
    
    while (i < len) {
        let addr = *vector::borrow(items, i);
        if (!is_valid_address(addr)) {
            return false;
        };
        i = i + 1;
    };
    
    true
}
```

## Testing Loops

```move
#[test]
fun test_while_loop() {
    let mut sum: u64 = 0;
    let mut i: u64 = 1;
    
    while (i <= 10) {
        sum = sum + i;
        i = i + 1;
    };
    
    assert!(sum == 55, 0); // 1+2+3+...+10
}

#[test]
fun test_vector_iteration() {
    let numbers = vector[10u64, 20, 30, 40];
    let sum = sum_vector(&numbers);
    assert!(sum == 100, 0);
}

#[test]
fun test_find_max() {
    let numbers = vector[5u64, 2, 8, 1, 9, 3];
    let max = find_max(&numbers);
    assert!(max == 9, 0);
}

#[test]
fun test_filter() {
    let numbers = vector[1i64, -2, 3, -4, 5];
    let positive = filter_positive(&numbers);
    assert!(vector::length(&positive) == 3, 0);
    assert!(*vector::borrow(&positive, 0) == 1, 1);
    assert!(*vector::borrow(&positive, 1) == 3, 2);
    assert!(*vector::borrow(&positive, 2) == 5, 3);
}
```

## Best Practices

### 1. Always Increment Counter

```move
// Bad: Infinite loop
// while (i < 10) {
//     // Forgot to increment i
// };

// Good: Always increment
while (i < 10) {
    // Process
    i = i + 1;
};
```

### 2. Use Descriptive Variable Names

```move
// Bad
let mut i = 0;
while (i < len) { }

// Good
let mut index = 0;
while (index < length) { }
```

### 3. Add Safety Limits

```move
public fun safe_loop() {
    let mut iterations: u64 = 0;
    let max_iterations: u64 = 10000;
    
    loop {
        assert!(iterations < max_iterations, 0);
        
        if (condition_met()) {
            break;
        };
        
        iterations = iterations + 1;
    }
}
```

### 4. Minimize Work Inside Loops

```move
// Bad: Recalculating constant value
while (i < len) {
    let threshold = calculate_threshold(); // Same every iteration
    // ...
}

// Good: Calculate once outside
let threshold = calculate_threshold();
while (i < len) {
    // Use threshold
}
```

## Performance Considerations

- Loops can be expensive in terms of gas
- Minimize iterations when possible
- Avoid nested loops for large datasets
- Consider batch operations
- Use efficient data structures

### Gas Optimization

```move
// Expensive: O(n²)
public fun bad_nested_loop(items: &vector<u64>) {
    let len = vector::length(items);
    let mut i = 0;
    while (i < len) {
        let mut j = 0;
        while (j < len) {
            // O(n²) operations
            j = j + 1;
        };
        i = i + 1;
    };
}

// Better: O(n) with hash map or sorting
public fun better_approach(items: &vector<u64>) {
    // Use more efficient algorithm
}
```

## Common Errors

### Off-by-One Errors

```move
// Wrong: Should be i < len, not i <= len
// while (i <= len) { } // Accesses out of bounds

// Correct
while (i < len) {
    // Process
    i = i + 1;
};
```

### Forgetting to Update Counter

```move
let mut i: u64 = 0;
while (i < 10) {
    // Process
    // Forgot: i = i + 1; // Infinite loop!
};
```

### Modifying Collection During Iteration

```move
// Dangerous: Don't modify vector while iterating
// while (i < vector::length(&mut vec)) {
//     vector::push_back(&mut vec, new_item); // Changes length!
// }
```

## Advanced Patterns

### Binary Search

```move
public fun binary_search(sorted: &vector<u64>, target: u64): u64 {
    let mut left: u64 = 0;
    let mut right = vector::length(sorted);
    
    while (left < right) {
        let mid = left + (right - left) / 2;
        let mid_val = *vector::borrow(sorted, mid);
        
        if (mid_val == target) {
            return mid;
        } else if (mid_val < target) {
            left = mid + 1;
        } else {
            right = mid;
        };
    };
    
    0xFFFFFFFFFFFFFFFF // Not found
}
```

### Exponential Backoff

```move
public fun retry_with_backoff(max_retries: u64): bool {
    let mut attempt: u64 = 0;
    let mut delay: u64 = 1000; // Start with 1 second
    
    while (attempt < max_retries) {
        if (try_operation()) {
            return true;
        };
        
        // Wait (simulated)
        // sleep(delay);
        
        // Exponential backoff
        delay = delay * 2;
        attempt = attempt + 1;
    };
    
    false
}
```

## Next Steps

- Learn about [Vector Operations](vector.md)
- Study [Algorithm Patterns](usage-examples.md)
- Explore [Gas Optimization](coding-conventions.md)
