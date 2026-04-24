# Unit Testing

Testing is crucial for building secure and reliable Move smart contracts. This guide covers testing strategies, patterns, and best practices.

## Test Basics

### Simple Test

```move
#[test]
fun test_addition() {
    let sum = 2 + 3;
    assert!(sum == 5, 0);
}

#[test]
fun test_subtraction() {
    let diff = 10 - 4;
    assert!(diff == 6, 0);
}
```

### Test with Setup

```move
#[test]
fun test_token_creation() {
    // Setup
    let ctx = &mut tx_context::dummy();
    
    // Execute
    let token = create_token(b"Test", b"TST", 9, ctx);
    
    // Verify
    assert!(token.name == b"Test", 0);
    assert!(token.symbol == b"TST", 1);
    assert!(token.decimals == 9, 2);
}
```

## Expected Failures

### Testing Error Conditions

```move
const E_INVALID_AMOUNT: u64 = 0;
const E_INSUFFICIENT_BALANCE: u64 = 1;

#[test]
#[expected_failure(abort_code = E_INVALID_AMOUNT)]
fun test_zero_amount_fails() {
    mint_tokens(0);
}

#[test]
#[expected_failure(abort_code = E_INSUFFICIENT_BALANCE)]
fun test_insufficient_balance_fails() {
    let mut balance = 100;
    withdraw(&mut balance, 200);
}

#[test]
#[expected_failure]
fun test_any_failure() {
    abort 999;
}
```

## Test-Only Code

### Test-Only Functions

```move
#[test_only]
public fun create_test_token(ctx: &mut TxContext): Coin {
    // Helper function only available in tests
    coin::mint_for_testing(1000, ctx)
}

#[test_only]
public fun setup_test_environment(): TestEnv {
    TestEnv {
        admin: @0x1,
        user1: @0x2,
        user2: @0x3,
    }
}
```

### Test-Only Modules

```move
#[test_only]
module test_helpers {
    use kanari_system::tx_context::TxContext;
    
    public fun dummy_ctx(): TxContext {
        tx_context::dummy()
    }
    
    public fun test_address(index: u64): address {
        @0x0 + index
    }
}
```

## Common Test Patterns

### Arrange-Act-Assert

```move
#[test]
fun test_transfer() {
    // Arrange
    let ctx = &mut tx_context::dummy();
    let sender = tx_context::sender(ctx);
    let recipient = @0x2;
    let amount = 100;
    
    let coins = create_test_coins(amount, ctx);
    let initial_balance = get_balance(recipient);
    
    // Act
    transfer_coins(coins, recipient);
    
    // Assert
    let new_balance = get_balance(recipient);
    assert!(new_balance == initial_balance + amount, 0);
}
```

### Test Fixtures

```move
struct TestFixture has drop {
    admin: address,
    user1: address,
    user2: address,
    token_cap: MintCap,
}

#[test_only]
fun create_fixture(ctx: &mut TxContext): TestFixture {
    let (cap, _) = coin::create_currency::<TOKEN>(
        TOKEN {}, 9, b"TKN", b"Token", b"Test", option::none(), ctx
    );
    
    TestFixture {
        admin: tx_context::sender(ctx),
        user1: @0x1,
        user2: @0x2,
        token_cap: cap,
    }
}

#[test]
fun test_mint_with_fixture() {
    let ctx = &mut tx_context::dummy();
    let fixture = create_fixture(ctx);
    
    let coins = coin::mint(&mut fixture.token_cap, 1000, ctx);
    assert!(coin::value(&coins) == 1000, 0);
}
```

## Testing Different Scenarios

### Boundary Testing

```move
#[test]
fun test_boundary_values() {
    // Test minimum
    assert!(is_valid_amount(1), 0);
    
    // Test maximum
    assert!(is_valid_amount(MAX_AMOUNT), 1);
    
    // Test just over maximum
    #[expected_failure]
    fun test_over_max() {
        is_valid_amount(MAX_AMOUNT + 1);
    }
    test_over_max();
    
    // Test zero
    #[expected_failure]
    fun test_zero() {
        is_valid_amount(0);
    }
    test_zero();
}
```

### State Transition Testing

```move
#[test]
fun test_order_state_transitions() {
    let ctx = &mut tx_context::dummy();
    let mut order = create_order(ctx);
    
    // Created -> Paid
    pay_order(&mut order);
    assert!(matches!(order.state, OrderState::Paid), 0);
    
    // Paid -> Shipped
    ship_order(&mut order);
    assert!(matches!(order.state, OrderState::Shipped), 1);
    
    // Shipped -> Delivered
    deliver_order(&mut order);
    assert!(matches!(order.state, OrderState::Delivered), 2);
}

#[test]
#[expected_failure]
fun test_invalid_transition() {
    let ctx = &mut tx_context::dummy();
    let mut order = create_order(ctx);
    
    // Can't go directly from Created to Shipped
    ship_order(&mut order);
}
```

### Access Control Testing

```move
#[test]
fun test_admin_access() {
    let ctx = &mut tx_context::dummy();
    let admin_cap = create_admin_cap(ctx);
    
    // Admin can execute
    admin_function(&admin_cap);
}

#[test]
#[expected_failure(abort_code = E_UNAUTHORIZED)]
fun test_non_admin_access() {
    // Non-admin cannot execute
    admin_function_without_cap();
}
```

## Testing Collections

### Vector Testing

```move
#[test]
fun test_vector_operations() {
    let mut v = vector::empty<u64>();
    
    // Test empty
    assert!(vector::is_empty(&v), 0);
    assert!(vector::length(&v) == 0, 1);
    
    // Test push
    vector::push_back(&mut v, 10);
    vector::push_back(&mut v, 20);
    assert!(vector::length(&v) == 2, 2);
    
    // Test access
    assert!(*vector::borrow(&v, 0) == 10, 3);
    assert!(*vector::borrow(&v, 1) == 20, 4);
    
    // Test pop
    let last = vector::pop_back(&mut v);
    assert!(last == 20, 5);
    assert!(vector::length(&v) == 1, 6);
}
```

### Table Testing

```move
#[test]
fun test_table_operations() {
    let ctx = &mut tx_context::dummy();
    let mut table = table::new<u64, address>(ctx);
    
    // Test empty
    assert!(table::length(&table) == 0, 0);
    
    // Test insert
    table::add(&mut table, 1, @0x1);
    table::add(&mut table, 2, @0x2);
    assert!(table::length(&table) == 2, 1);
    
    // Test contains
    assert!(table::contains<u64, address>(&table, &1), 2);
    assert!(!table::contains<u64, address>(&table, &999), 3);
    
    // Test borrow
    let addr = table::borrow<u64, address>(&table, &1);
    assert!(*addr == @0x1, 4);
    
    // Test remove
    table::remove<u64, address>(&mut table, &1);
    assert!(table::length(&table) == 1, 5);
}
```

## Integration Testing

### Multi-Module Tests

```move
#[test]
fun test_full_workflow() {
    let ctx = &mut tx_context::dummy();
    
    // Create token
    let (cap, meta) = token::create_token(ctx);
    
    // Mint tokens
    let coins = token::mint(&mut cap, 1000, ctx);
    
    // Transfer
    let recipient = @0x2;
    token::transfer(coins, recipient);
    
    // Verify balance
    let balance = token::balance_of(recipient);
    assert!(balance == 1000, 0);
}
```

### Cross-Module Dependencies

```move
#[test]
fun test_dex_swap() {
    let ctx = &mut tx_context::dummy();
    
    // Setup tokens
    let (cap_a, _) = token_a::create(ctx);
    let (cap_b, _) = token_b::create(ctx);
    
    // Create pool
    let pool = dex::create_pool(ctx);
    
    // Add liquidity
    let coins_a = token_a::mint(&mut cap_a, 1000, ctx);
    let coins_b = token_b::mint(&mut cap_b, 1000, ctx);
    dex::add_liquidity(&pool, coins_a, coins_b);
    
    // Perform swap
    let input = token_a::mint(&mut cap_a, 100, ctx);
    let output = dex::swap(&pool, input);
    
    // Verify
    assert!(coin::value(&output) > 0, 0);
}
```

## Property-Based Testing

### Invariant Testing

```move
#[test]
fun test_supply_invariant() {
    let ctx = &mut tx_context::dummy();
    let (mut cap, meta) = create_token(ctx);
    
    let initial_supply = coin::supply(&meta);
    
    // Mint some tokens
    let coins = coin::mint(&mut cap, 1000, ctx);
    let new_supply = coin::supply(&meta);
    
    // Supply should increase by minted amount
    assert!(new_supply == initial_supply + 1000, 0);
    
    // Burn some tokens
    coin::burn(&mut cap, coins);
    let final_supply = coin::supply(&meta);
    
    // Supply should return to initial
    assert!(final_supply == initial_supply, 1);
}
```

### Conservation Laws

```move
#[test]
fun test_value_conservation() {
    let ctx = &mut tx_context::dummy();
    
    let user1_balance = 1000;
    let user2_balance = 500;
    let total_before = user1_balance + user2_balance;
    
    // Transfer
    let transfer_amount = 200;
    let user1_after = user1_balance - transfer_amount;
    let user2_after = user2_balance + transfer_amount;
    let total_after = user1_after + user2_after;
    
    // Total should be conserved
    assert!(total_before == total_after, 0);
}
```

## Test Utilities

### Mock Objects

```move
#[test_only]
struct MockClock {
    timestamp: u64,
}

#[test_only]
impl MockClock {
    public fun new(ts: u64): Self {
        MockClock { timestamp: ts }
    }
    
    public fun timestamp_ms(&self): u64 {
        self.timestamp
    }
    
    public fun advance(&mut self, ms: u64) {
        self.timestamp += ms;
    }
}

#[test]
fun test_time_based_logic() {
    let mut clock = MockClock::new(1000);
    
    // Test at time 1000
    assert!(is_active(&clock), 0);
    
    // Advance time
    clock.advance(86400000); // 1 day
    
    // Test at time 86401000
    assert!(!is_active(&clock), 1);
}
```

### Test Data Generators

```move
#[test_only]
fun random_address(seed: u64): address {
    @0x0 + seed
}

#[test_only]
fun random_amount(seed: u64): u64 {
    (seed % 1000) + 1
}

#[test]
fun test_with_generated_data() {
    let ctx = &mut tx_context::dummy();
    
    for (i in 1..10) {
        let addr = random_address(i);
        let amount = random_amount(i);
        
        // Test with generated data
        let coins = create_coins(amount, ctx);
        transfer_coins(coins, addr);
        
        assert!(get_balance(addr) == amount, i);
    }
}
```

## Best Practices

### 1. Test Both Success and Failure

```move
#[test]
fun test_success_case() {
    // Test happy path
}

#[test]
#[expected_failure]
fun test_failure_case() {
    // Test error path
}
```

### 2. Use Descriptive Test Names

```move
// Good
#[test]
fun test_mint_zero_amount_fails() { }

#[test]
fn test_transfer_to_self_fails() { }

// Bad
#[test]
fn test1() { }

#[test]
fn test_error() { }
```

### 3. Keep Tests Independent

```move
// Good: Each test sets up its own state
#[test]
fn test_scenario_a() {
    let ctx = &mut tx_context::dummy();
    // Setup and test
}

#[test]
fn test_scenario_b() {
    let ctx = &mut tx_context::dummy();
    // Independent setup and test
}
```

### 4. Test Edge Cases

```move
#[test]
fn test_edge_cases() {
    // Minimum value
    test_operation(MIN_VALUE);
    
    // Maximum value
    test_operation(MAX_VALUE);
    
    // Zero
    #[expected_failure]
    fn test_zero() { test_operation(0); }
    test_zero();
    
    // Overflow
    #[expected_failure]
    fn test_overflow() { test_operation(MAX_VALUE + 1); }
    test_overflow();
}
```

### 5. Document Test Purpose

```move
/// Tests that minting zero tokens fails with E_INVALID_AMOUNT
#[test]
#[expected_failure(abort_code = E_INVALID_AMOUNT)]
fun test_mint_zero_amount_fails() {
    mint_tokens(0);
}

/// Tests that transferring to self fails with E_SELF_TRANSFER
#[test]
#[expected_failure(abort_code = E_SELF_TRANSFER)]
fun test_transfer_to_self_fails() {
    let ctx = &mut tx_context::dummy();
    let sender = tx_context::sender(ctx);
    let coins = create_coins(100, ctx);
    transfer_coins(coins, sender);
}
```

## Running Tests

### Run All Tests

```bash
move test
```

### Run Specific Test

```bash
move test --filter test_mint
```

### Run with Verbose Output

```bash
move test --verbose
```

## Test Coverage

Aim for comprehensive coverage:

- [ ] All public functions tested
- [ ] Happy path scenarios
- [ ] Error/failure scenarios
- [ ] Edge cases (min, max, zero)
- [ ] State transitions
- [ ] Access control
- [ ] Invariants maintained
- [ ] Integration workflows

## Next Steps

- Learn about [Integration Testing](../testing/integration.md)
- Study [Fuzz Testing](../testing/fuzz.md)
- Explore [Formal Verification](../testing/formal.md)
