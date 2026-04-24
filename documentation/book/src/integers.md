# Integers

Integers are fundamental numeric types in Move. Move supports both unsigned and signed integers of various bit widths.

## Integer Types

Move provides the following integer types:

### Unsigned Integers

- `u8`: 8-bit unsigned integer (0 to 255)
- `u16`: 16-bit unsigned integer (0 to 65,535)
- `u32`: 32-bit unsigned integer (0 to 4,294,967,295)
- `u64`: 64-bit unsigned integer (0 to 18,446,744,073,709,551,615)
- `u128`: 128-bit unsigned integer
- `u256`: 256-bit unsigned integer

### Signed Integers

- `i8`: 8-bit signed integer (-128 to 127)
- `i16`: 16-bit signed integer
- `i32`: 32-bit signed integer
- `i64`: 64-bit signed integer
- `i128`: 128-bit signed integer
- `i256`: 256-bit signed integer

## Literals

Integer literals can be written in several formats:

```move
// Decimal
let x: u64 = 100;
let y: u64 = 1_000_000; // Underscores for readability

// Hexadecimal
let hex: u64 = 0xFF; // 255
let addr: address = 0x1234abcd;

// Binary
let binary: u8 = 0b1010; // 10

// Octal
let octal: u8 = 0o17; // 15
```

## Arithmetic Operations

```move
let a: u64 = 10;
let b: u64 = 3;

// Addition
let sum = a + b; // 13

// Subtraction
let diff = a - b; // 7

// Multiplication
let product = a * b; // 30

// Division
let quotient = a / b; // 3

// Modulo
let remainder = a % b; // 1
```

## Overflow Behavior

Move integers **panic on overflow** in debug mode. In production, you should use checked operations:

```move
use kanari_system::math;

// Safe addition with overflow check
let result = math::checked_add_u64(a, b);

// Safe multiplication
let product = math::checked_mul_u64(a, b);

// Or use wider types
let wide_a: u128 = (a as u128);
let wide_b: u128 = (b as u128);
let safe_product = wide_a * wide_b;
```

## Type Conversions

```move
// Casting to wider types (safe)
let small: u8 = 100;
let large: u64 = (small as u64);

// Casting to narrower types (may truncate)
let big: u64 = 300;
let small_truncated: u8 = (big as u8); // 44 (300 % 256)

// Use kanari math module for safe conversions
let safe_convert = math::to_u8_saturated(big); // Returns 255 if too large
```

## Common Patterns

### Token Amounts

```move
// KARI uses 9 decimals
const MIST_PER_KARI: u64 = 1_000_000_000;

let kari_amount: u64 = 100;
let mist_amount: u64 = kari_amount * MIST_PER_KARI; // 100,000,000,000
```

### Timestamps

```move
use kanari_system::clock;

// Timestamps are typically u64 milliseconds
let now_ms: u64 = clock::timestamp_ms();
let one_hour_later = now_ms + 3_600_000; // Add 1 hour
```

### Loop Counters

```move
let mut i: u64 = 0;
while (i < 100) {
    // Do something
    i = i + 1;
};
```

## Best Practices

1. **Use appropriate sizes**: Don't use `u256` when `u64` suffices
2. **Check for overflow**: Use checked operations for financial calculations
3. **Use underscores**: Improve readability with `1_000_000` instead of `1000000`
4. **Prefer unsigned**: Use unsigned types unless negative values are needed
5. **Document units**: Comment whether values are in base units or display units

## Common Errors

### Overflow Panic

```move
#[test]
#[expected_failure]
fun test_overflow() {
    let max_u8: u8 = 255;
    let _overflow = max_u8 + 1; // Panics in debug mode
}
```

### Truncation Warning

```move
let big: u64 = 300;
let small: u8 = (big as u8); // Loses data - 300 becomes 44
```

## Mathematical Helper Functions

The `kanari_system::math` module provides safe operations:

```move
use kanari_system::math;

// Square root
let sqrt = math::sqrt_u64(100); // 10

// Power
let power = math::pow_u64(2, 8); // 256

// Min/Max
let min_val = math::min_u64(10, 20); // 10
let max_val = math::max_u64(10, 20); // 20

// Percentage calculation
let ten_percent = math::percentage(1000, 10); // 100
```

## Next Steps

- Learn about [Bool](bool.md) for conditional logic
- Explore [Math Operations](usage-examples.md#mathematical-operations)
- Study [Constants](constants.md) for defining fixed values
