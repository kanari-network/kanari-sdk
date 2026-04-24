# Constants

Constants are immutable values defined at module level. They provide named, reusable values throughout your code.

## Constant Declaration

```move
module my_module::constants {
    // Basic constant
    const MAX_SUPPLY: u64 = 1000000;
    
    // Address constant
    const ADMIN_ADDRESS: address = @0x123;
    
    // Byte vector constant
    const TOKEN_NAME: vector<u8> = b"My Token";
    
    // Boolean constant
    const IS_ACTIVE: bool = true;
}
```

## Supported Types

Constants can be of these types:

```move
// Integers
const U8_VAL: u8 = 255;
const U64_VAL: u64 = 1_000_000;
const U128_VAL: u128 = 999_999_999_999;

// Boolean
const TRUE_VAL: bool = true;
const FALSE_VAL: bool = false;

// Address
const ZERO_ADDR: address = @0x0;
const SYSTEM_ADDR: address = @0x1;

// Byte vectors
const EMPTY_BYTES: vector<u8> = b"";
const MESSAGE: vector<u8> = b"Hello, World!";

// Hex bytes
const HASH: vector<u8> = x"0000000000000000";
```

## Naming Conventions

Use `SCREAMING_SNAKE_CASE` for constants:

```move
// Good
const MAX_VALUE: u64 = 100;
const MIN_PRICE: u64 = 1;
const TOKEN_DECIMALS: u8 = 9;

// Bad
const maxValue: u64 = 100;
const min_price: u64 = 1;
```

## Common Patterns

### Error Codes

```move
module errors::codes {
    // Validation errors
    const E_INVALID_AMOUNT: u64 = 0;
    const E_INSUFFICIENT_BALANCE: u64 = 1;
    const E_UNAUTHORIZED: u64 = 2;
    const E_ALREADY_EXISTS: u64 = 3;
    const E_NOT_FOUND: u64 = 4;
    
    // State errors
    const E_INVALID_STATE: u64 = 100;
    const E_OVERFLOW: u64 = 101;
    const E_UNDERFLOW: u64 = 102;
}

// Usage
public fun withdraw(balance: &mut u64, amount: u64) {
    assert!(amount > 0, errors::codes::E_INVALID_AMOUNT);
    assert!(*balance >= amount, errors::codes::E_INSUFFICIENT_BALANCE);
    
    *balance -= amount;
}
```

### Configuration Constants

```move
module config::token {
    // Token parameters
    const DECIMALS: u8 = 9;
    const MIST_PER_TOKEN: u64 = 1_000_000_000;
    const MAX_SUPPLY: u64 = 10_000_000_000;
    const MIN_MINT_AMOUNT: u64 = 1_000_000;
    
    // Fee configuration
    const TRANSFER_FEE_BPS: u64 = 25; // 0.25%
    const BPS_DENOMINATOR: u64 = 10_000;
}

// Usage
public fun calculate_fee(amount: u64): u64 {
    amount * config::token::TRANSFER_FEE_BPS / config::token::BPS_DENOMINATOR
}
```

### Time Constants

```move
module config::time {
    const SECOND_MS: u64 = 1_000;
    const MINUTE_MS: u64 = 60_000;
    const HOUR_MS: u64 = 3_600_000;
    const DAY_MS: u64 = 86_400_000;
    const WEEK_MS: u64 = 604_800_000;
    const YEAR_MS: u64 = 31_536_000_000;
}

// Usage
use config::time;

public fun is_expired(created_at: u64, current_time: u64): bool {
    current_time - created_at > time::DAY_MS * 30 // 30 days
}
```

### Mathematical Constants

```move
module math::constants {
    const PI_X1000: u64 = 3141; // π * 1000
    const E_X1000: u64 = 2718;  // e * 1000
    
    // Percentage helpers
    const PERCENT_100: u64 = 10_000; // 100% in basis points
    const PERCENT_50: u64 = 5_000;   // 50%
    const PERCENT_10: u64 = 1_000;   // 10%
    const PERCENT_1: u64 = 100;      // 1%
}
```

## Using Constants Across Modules

```move
module user::app {
    use config::token;
    use errors::codes;
    
    public fun mint(amount: u64) {
        assert!(amount >= token::MIN_MINT_AMOUNT, codes::E_INVALID_AMOUNT);
        assert!(amount <= token::MAX_SUPPLY, codes::E_OVERFLOW);
        
        // Mint logic
    }
}
```

## Constant Expressions

Constants must be compile-time computable:

```move
// Valid: Simple literals
const VALID: u64 = 100;

// Valid: Arithmetic with literals
const COMPUTED: u64 = 100 * 10;

// Invalid: Runtime values
// const INVALID: u64 = some_function(); // Error!
```

## Best Practices

### 1. Group Related Constants

```move
module config::dex {
    // Pool constants
    const MIN_LIQUIDITY: u64 = 1_000;
    const MAX_SWAP_SLIPPAGE_BPS: u64 = 500; // 5%
    
    // Fee constants
    const SWAP_FEE_BPS: u64 = 30; // 0.3%
    const PROTOCOL_FEE_BPS: u64 = 5; // 0.05%
    
    // Price constants
    const PRICE_PRECISION: u64 = 1_000_000;
}
```

### 2. Use Descriptive Names

```move
// Bad
const X: u64 = 1000000000;

// Good
const NANOSECONDS_PER_SECOND: u64 = 1_000_000_000;
```

### 3. Document Complex Constants

```move
/// Maximum number of items per batch for gas optimization.
/// Calculated based on average transaction size and block limit.
const MAX_BATCH_SIZE: u64 = 100;

/// Basis points denominator (100% = 10,000 bps)
const BPS_DENOMINATOR: u64 = 10_000;
```

### 4. Centralize Configuration

```move
// Don't scatter constants
module scattered {
    const FEE: u64 = 25; // Where did this come from?
}

// Do centralize
module config {
    /// Protocol fee in basis points (0.25%)
    const PROTOCOL_FEE_BPS: u64 = 25;
}
```

## Common Patterns

### Enum-like Constants

```move
module status::codes {
    const STATUS_PENDING: u8 = 0;
    const STATUS_ACTIVE: u8 = 1;
    const STATUS_COMPLETED: u8 = 2;
    const STATUS_CANCELLED: u8 = 3;
    const STATUS_FAILED: u8 = 4;
}

struct Order has key, store {
    id: UID,
    status: u8,
}

public fun complete_order(order: &mut Order) {
    order.status = status::codes::STATUS_COMPLETED;
}
```

### Version Constants

```move
module version {
    const MAJOR: u8 = 1;
    const MINOR: u8 = 2;
    const PATCH: u8 = 3;
    
    /// Version as string: "1.2.3"
    const VERSION_STRING: vector<u8> = b"1.2.3";
}
```

### Permission Flags

```move
module permissions {
    const READ: u8 = 1;       // 0b0001
    const WRITE: u8 = 2;      // 0b0010
    const DELETE: u8 = 4;     // 0b0100
    const ADMIN: u8 = 8;      // 0b1000
    
    const FULL_ACCESS: u8 = 15; // 0b1111
}

public fun has_permission(flags: u8, permission: u8): bool {
    flags & permission == permission
}
```

## Testing Constants

```move
#[test]
fun test_constants() {
    assert!(config::token::DECIMALS == 9, 0);
    assert!(config::token::MIST_PER_TOKEN == 1_000_000_000, 1);
    assert!(config::time::HOUR_MS == 3_600_000, 2);
}

#[test]
fun test_error_codes_unique() {
    // Ensure error codes don't overlap
    assert!(errors::codes::E_INVALID_AMOUNT != errors::codes::E_UNAUTHORIZED, 0);
    assert!(errors::codes::E_INSUFFICIENT_BALANCE != errors::codes::E_NOT_FOUND, 1);
}
```

## Performance Considerations

- Constants are inlined at compile time (no runtime overhead)
- Prefer constants over repeated literal values
- Large byte vectors may increase bytecode size
- Group related constants to improve cache locality

## Common Mistakes

### Mutable-Looking Constants

```move
// Constants are always immutable
const VALUE: u64 = 100;
// VALUE = 200; // Error: cannot assign to constant
```

### Circular Dependencies

```move
// Don't create circular references
// const A: u64 = B + 1;
// const B: u64 = A + 1; // Error!
```

### Type Mismatches

```move
const VAL: u64 = 100;
// let x: u8 = VAL; // Error: type mismatch

let x: u64 = VAL; // OK
let y: u8 = (VAL as u8); // OK with cast
```

## Next Steps

- Learn about [Modules](modules-and-scripts.md) for organization
- Study [Error Handling](abort-and-assert.md)
- Explore [Configuration Patterns](usage-examples.md)
