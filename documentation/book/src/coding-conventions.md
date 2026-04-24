# Coding Conventions

This guide outlines best practices and conventions for writing clean, maintainable, and efficient Move code for the Kanari blockchain.

## Naming Conventions

### Modules and Functions

Use `snake_case` for modules and functions:

```move
module my_project::token_manager {
    // Good
    public fun mint_tokens() { }
    public fun get_balance() { }
    public entry fun transfer_to_recipient() { }
    
    // Bad
    public fun MintTokens() { }
    public fun getBalance() { }
}
```

### Structs and Types

Use `PascalCase` for structs and types:

```move
// Good
struct UserProfile has key, store { }
struct TokenMetadata has copy, drop { }
enum OrderStatus has copy, drop { }

// Bad
struct user_profile { }
struct token_metadata { }
```

### Constants

Use `SCREAMING_SNAKE_CASE` for constants:

```move
// Good
const MAX_SUPPLY: u64 = 1_000_000;
const DECIMALS: u8 = 9;
const ADMIN_ADDRESS: address = @0x123;

// Bad
const maxSupply: u64 = 1_000_000;
const decimals: u8 = 9;
```

### Variables

Use `snake_case` for variables:

```move
// Good
let user_balance: u64 = 100;
let mut total_amount: u64 = 0;
let recipient_address: address = @0x456;

// Bad
let userBalance: u64 = 100;
let TotalAmount: u64 = 0;
```

## Code Organization

### Module Structure

Organize modules in a consistent order:

```move
module my_project::token {
    // 1. Use statements
    use std::vector;
    use kanari_system::coin;
    use kanari_system::transfer;
    
    // 2. Constants
    const DECIMALS: u8 = 9;
    const MAX_SUPPLY: u64 = 1_000_000;
    
    // 3. Error codes
    const E_INVALID_AMOUNT: u64 = 0;
    const E_INSUFFICIENT_BALANCE: u64 = 1;
    
    // 4. Structs and enums
    struct Token has drop {}
    struct Balance has key, store { }
    
    // 5. Public API functions
    public fun mint() { }
    public fun burn() { }
    
    // 6. Internal helper functions
    fun validate_amount() { }
    
    // 7. Tests
    #[test]
    fun test_mint() { }
}
```

### Function Ordering

Within a module, order functions logically:

```move
// Constructor/initializer first
public fun initialize() { }

// Core functionality
public fun mint() { }
public fun burn() { }
public fun transfer() { }

// Query/view functions
public fun balance_of() { }
public fun total_supply() { }

// Helper/private functions
fun validate() { }
fun update_state() { }
```

## Documentation

### Module Documentation

Document each module:

```move
/// # Token Module
/// 
/// Implements a fungible token with minting and burning capabilities.
/// 
/// # Features
/// - Mint new tokens (admin only)
/// - Burn existing tokens
/// - Transfer between accounts
/// - Query balances
/// 
/// # Example
/// ```
/// let tokens = token::mint(1000);
/// token::transfer(tokens, recipient);
/// ```
module my_project::token {
    // ...
}
```

### Function Documentation

Use doc comments for public functions:

```move
/// Mints new tokens and transfers to recipient
/// 
/// # Arguments
/// * `cap` - Mint capability reference
/// * `amount` - Amount to mint (in smallest units)
/// * `recipient` - Address to receive tokens
/// * `ctx` - Transaction context
/// 
/// # Panics
/// * If amount is zero
/// * If recipient is zero address
/// * If total supply would exceed maximum
public entry fun mint_and_transfer(
    cap: &mut MintCap,
    amount: u64,
    recipient: address,
    ctx: &mut TxContext
) {
    // Implementation
}
```

### Inline Comments

Use comments sparingly for complex logic:

```move
// Calculate fee as basis points (1 bps = 0.01%)
let fee_bps = 25; // 0.25%
let fee = amount * fee_bps / 10_000;

// Update state after validation
balance -= amount;
```

## Error Handling

### Error Code Organization

Group error codes by category:

```move
// Validation errors (0-99)
const E_INVALID_AMOUNT: u64 = 0;
const E_INVALID_ADDRESS: u64 = 1;
const E_OUT_OF_RANGE: u64 = 2;

// Authorization errors (100-199)
const E_UNAUTHORIZED: u64 = 100;
const E_NOT_OWNER: u64 = 101;

// State errors (200-299)
const E_INSUFFICIENT_BALANCE: u64 = 200;
const E_SUPPLY_EXCEEDED: u64 = 201;
```

### Assertion Style

Use assertions for validation:

```move
// Good: Clear and concise
assert!(amount > 0, E_INVALID_AMOUNT);
assert!(balance >= amount, E_INSUFFICIENT_BALANCE);

// Bad: Verbose if-statements
if (amount == 0) {
    abort E_INVALID_AMOUNT
};
```

## Gas Optimization

### Efficient Data Structures

Choose appropriate data structures:

```move
// For small collections (< 100 items): vector
let items = vector[1u64, 2, 3];

// For large collections or key-value: Table
let mut table = table::new<u64, address>(ctx);

// For heterogeneous data: Bag
let mut bag = bag::new(ctx);
```

### Minimize Storage Writes

```move
// Bad: Multiple writes
balance.amount = balance.amount + 100;
balance.timestamp = clock::timestamp_ms();
balance.last_update = tx_context::sender(ctx);

// Better: Batch updates when possible
update_balance(&mut balance, 100, ctx);
```

### Avoid Unnecessary Copies

```move
// Bad: Unnecessary copy
let data = *borrow(&large_struct);
process(data);

// Good: Use reference
process(borrow(&large_struct));
```

### Loop Optimization

```move
// Cache length outside loop
let len = vector::length(&items);
let mut i = 0;
while (i < len) {
    // Process
    i = i + 1;
};
```

## Security Best Practices

### Input Validation

Always validate inputs:

```move
public fun transfer(amount: u64, recipient: address) {
    assert!(amount > 0, E_INVALID_AMOUNT);
    assert!(recipient != @0x0, E_INVALID_RECIPIENT);
    assert!(recipient != tx_context::sender(ctx), E_SELF_TRANSFER);
    
    // Execute transfer
}
```

### Access Control

Implement proper access control:

```move
struct AdminCap has key, store {
    id: UID,
}

public fun admin_only_function(cap: &AdminCap) {
    // Requires admin capability
}

public entry fun protected_operation(
    cap: &AdminCap,
    ctx: &mut TxContext
) {
    // Verify caller has capability
    assert!(object::owner(&cap.id) == tx_context::sender(ctx), E_UNAUTHORIZED);
    
    // Execute operation
}
```

### Reentrancy Protection

Guard against reentrancy:

```move
struct Account has key, store {
    id: UID,
    balance: u64,
    locked: bool,
}

public fun withdraw(account: &mut Account, amount: u64) {
    assert!(!account.locked, E_REENTRANCY);
    account.locked = true;
    
    // Perform withdrawal
    account.balance -= amount;
    
    account.locked = false;
}
```

### Integer Overflow Protection

Use safe arithmetic:

```move
use kanari_system::math;

// Safe addition
let sum = math::checked_add_u64(a, b);

// Safe multiplication
let product = math::checked_mul_u64(a, b);

// Or use wider types
let wide_sum = (a as u128) + (b as u128);
```

## Testing Conventions

### Test Organization

Organize tests logically:

```move
#[test]
fun test_initialization() {
    // Test setup
}

#[test]
fun test_minting() {
    // Test mint functionality
}

#[test]
fun test_transfers() {
    // Test transfer scenarios
}

#[test]
#[expected_failure(abort_code = E_INVALID_AMOUNT)]
fun test_invalid_mint() {
    // Test error cases
}
```

### Test Naming

Use descriptive test names:

```move
// Good
#[test]
fun test_mint_zero_amount_fails() { }

#[test]
fun test_transfer_to_self_fails() { }

// Bad
#[test]
fun test1() { }

#[test]
fn test_error() { }
```

## Performance Guidelines

### Batch Operations

Prefer batch operations:

```move
// Bad: Multiple transactions
for item in items {
    process_item(item);
}

// Good: Single batch transaction
public entry fun process_batch(items: vector<Item>) {
    let len = vector::length(&items);
    let mut i = 0;
    while (i < len) {
        process_item(vector::borrow(&items, i));
        i = i + 1;
    };
}
```

### Minimize On-Chain Computation

```move
// Bad: Complex calculation on-chain
let result = complex_calculation(input1, input2, input3);

// Better: Pre-calculate off-chain, verify on-chain
assert!(verify_proof(proof, expected_result), E_INVALID_PROOF);
```

## Code Review Checklist

Before submitting code:

- [ ] All public functions have documentation
- [ ] Error codes are defined as constants
- [ ] Input validation is comprehensive
- [ ] Access controls are implemented
- [ ] Tests cover happy path and error cases
- [ ] Gas optimization considerations applied
- [ ] No hardcoded addresses or values
- [ ] Follows naming conventions
- [ ] Module structure is organized
- [ ] Security best practices followed

## Common Anti-Patterns

### Magic Numbers

```move
// Bad
if (amount > 1000000000) { }

// Good
const MAX_AMOUNT: u64 = 1_000_000_000;
if (amount > MAX_AMOUNT) { }
```

### Deep Nesting

```move
// Bad
if (cond1) {
    if (cond2) {
        if (cond3) {
            // Deep nesting
        }
    }
}

// Good
if (!cond1) return;
if (!cond2) return;
if (!cond3) return;
// Main logic
```

### Long Functions

```move
// Bad: 100+ line function
public fn do_everything() {
    // Validate
    // Calculate
    // Update state
    // Emit events
    // Return
}

// Good: Separated concerns
public fn process() {
    validate();
    let result = calculate();
    update_state(result);
    emit_events(result);
}
```

## Next Steps

- Review [Security Audit Checklist](../security/audit-checklist.md)
- Study [Gas Optimization Guide](../advanced/gas-optimization.md)
- Explore [Design Patterns](../patterns/README.md)
