# Friends

The `friend` declaration in Move allows modules to grant special access privileges to other modules within the same package. This enables controlled encapsulation and modular design.

## Friend Basics

### Declaring Friends

```move
module my_package::token {
    // Grant friend access to another module
    friend my_package::treasury;
    
    struct TOKEN has drop {}
    
    // Private function - only accessible to this module and friends
    fun internal_mint(amount: u64): Coin<TOKEN> {
        // Internal minting logic
    }
    
    // Public function
    public fun public_mint(amount: u64): Coin<TOKEN> {
        internal_mint(amount)
    }
}
```

### Using Friend Access

```move
module my_package::treasury {
    use my_package::token;
    
    // Can call private functions because we're a friend
    public fun mint_treasury_tokens(amount: u64): token::Coin<token::TOKEN> {
        token::internal_mint(amount) // Allowed!
    }
}
```

## Friend Use Cases

### Module Separation

Separate public API from internal implementation:

```move
module my_package::core {
    friend my_package::admin;
    friend my_package::governance;
    
    struct Config has key, store {
        id: UID,
        max_supply: u64,
        fee_rate: u64,
    }
    
    // Private: Only friends can modify
    fun update_config(config: &mut Config, new_max: u64) {
        config.max_supply = new_max;
    }
    
    // Public: Anyone can read
    public fun get_max_supply(config: &Config): u64 {
        config.max_supply
    }
}

module my_package::admin {
    use my_package::core;
    
    // Can update config (friend access)
    public entry fun update_max_supply(
        config: &mut core::Config,
        new_max: u64
    ) {
        core::update_config(config, new_max);
    }
}

module my_package::governance {
    use my_package::core;
    
    // Can also update config (friend access)
    public entry fun governance_update(
        config: &mut core::Config,
        new_max: u64
    ) {
        core::update_config(config, new_max);
    }
}
```

### Capability Management

Control who can create capabilities:

```move
module my_package::capabilities {
    friend my_package::factory;
    
    struct AdminCap has key, store {
        id: UID,
    }
    
    // Private: Only factory can create
    fun create_admin_cap(ctx: &mut TxContext): AdminCap {
        AdminCap { id: object::new(ctx) }
    }
    
    // Public: Anyone can use
    public fun requires_admin(_cap: &AdminCap) {
        // Admin-only operation
    }
}

module my_package::factory {
    use my_package::capabilities;
    
    // Can create admin caps (friend access)
    public fun initialize_system(ctx: &mut TxContext) {
        let cap = capabilities::create_admin_cap(ctx);
        transfer::public_transfer(cap, tx_context::sender(ctx));
    }
}
```

### Testing Helpers

Provide test-only functionality:

```move
module my_package::token {
    #[test_only]
    friend my_package::token_tests;
    
    struct TOKEN has drop {}
    
    // Test-only function
    #[test_only]
    fun mint_for_testing(amount: u64, ctx: &mut TxContext): Coin<TOKEN> {
        // Simplified minting for tests
    }
}

#[test_only]
module my_package::token_tests {
    use my_package::token;
    
    #[test]
    fun test_token_operations() {
        let ctx = &mut tx_context::dummy();
        let coins = token::mint_for_testing(1000, ctx);
        // Test logic
    }
}
```

## Advanced Patterns

### Layered Architecture

```move
// Core layer - business logic
module my_package::core_logic {
    friend my_package::api_layer;
    
    fun process_transaction(tx: Transaction): Result {
        // Complex business logic
    }
}

// API layer - public interface
module my_package::api_layer {
    use my_package::core_logic;
    
    public entry fun submit_transaction(tx: Transaction) {
        // Validate input
        // Call core logic (friend access)
        core_logic::process_transaction(tx);
    }
}

// External users can only use API layer
```

### Upgrade Proxy Pattern

```move
module my_package::implementation_v1 {
    friend my_package::proxy;
    
    fun execute_operation(data: vector<u8>) {
        // Implementation details
    }
}

module my_package::proxy {
    use my_package::implementation_v1;
    
    public entry fun delegate_call(data: vector<u8>) {
        // Forward to implementation (friend access)
        implementation_v1::execute_operation(data);
    }
}
```

### Multi-Module Coordination

```move
module my_package::orchestrator {
    friend my_package::step_one;
    friend my_package::step_two;
    friend my_package::step_three;
    
    struct WorkflowState has key, store {
        id: UID,
        current_step: u8,
        completed: bool,
    }
    
    // Private: Only workflow steps can update
    fun advance_step(state: &mut WorkflowState) {
        state.current_step += 1;
    }
}

module my_package::step_one {
    use my_package::orchestrator;
    
    public fun execute_step(state: &mut orchestrator::WorkflowState) {
        // Do step one work
        orchestrator::advance_step(state); // Friend access
    }
}
```

## Best Practices

### 1. Minimize Friend Relationships

```move
// Bad: Too many friends
friend module_a;
friend module_b;
friend module_c;
friend module_d;

// Good: Few, well-defined friends
friend admin_module;
friend governance_module;
```

### 2. Document Friend Purpose

```move
/// Treasury module needs friend access to mint reserve tokens
friend my_package::treasury;

/// Governance module needs friend access to update parameters
friend my_package::governance;
```

### 3. Use Friends for Encapsulation

```move
// Keep internal functions private
fun internal_helper() { }

// Grant friend access only when necessary
friend trusted_module;

// Prefer public APIs when possible
public fun public_api() { }
```

### 4. Avoid Circular Friends

```move
// Bad: Circular dependency
// Module A friends Module B
// Module B friends Module A

// Good: Unidirectional friend relationships
// Module A friends Module B (A trusts B)
```

## Security Considerations

### Trust Boundaries

```move
// Only grant friend access to trusted modules
friend my_package::verified_admin;

// Don't grant to untrusted code
// friend unknown_module; // Dangerous!
```

### Capability Leakage

```move
module secure::caps {
    friend secure::factory;
    
    struct PowerfulCap has key, store {
        id: UID,
    }
    
    // Private creation
    fun create_cap(ctx: &mut TxContext): PowerfulCap {
        PowerfulCap { id: object::new(ctx) }
    }
}

// Ensure factory properly controls distribution
module secure::factory {
    use secure::caps;
    
    public fun distribute_cap(recipient: address, ctx: &mut TxContext) {
        let cap = caps::create_cap(ctx);
        transfer::public_transfer(cap, recipient);
    }
}
```

## Testing Friends

```move
#[test]
fun test_friend_access() {
    // Test that friend module can access private functions
    let result = friend_module::call_private_function();
    assert!(result == expected, 0);
}

#[test]
#[expected_failure]
fun test_non_friend_cannot_access() {
    // This should fail to compile if uncommented
    // non_friend_module::call_private_function();
}
```

## Common Errors

### Friend Not in Same Package

```move
// Wrong: Friends must be in same package
// friend other_package::module; // Error!

// Correct: Same package
friend my_package::module;
```

### Circular Friend Dependency

```move
// Module A
friend my_package::module_b;

// Module B
friend my_package::module_a; // May cause issues
```

### Missing Friend Declaration

```move
// Module A tries to call private function in Module B
// But Module A is not declared as friend
// Error: Cannot access private function
```

## Performance Considerations

- Friend declarations have zero runtime cost
- Purely compile-time access control
- No performance difference between friend and non-friend calls
- Use for organization, not optimization

## Next Steps

- Learn about [Modules](modules-and-scripts.md) for organization
- Study [Access Control](../security/access-control.md)
- Explore [Package Structure](packages.md)
