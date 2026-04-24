# Uses and Aliases

The `use` statement in Move allows you to import modules, functions, structs, and constants from other modules, making your code cleaner and more organized.

## Basic Use Statements

### Import Module

```move
use std::vector;

public fun example() {
    let v = vector::empty<u64>();
}
```

### Import Specific Items

```move
use std::vector::{push_back, pop_back, length};

public fun example() {
    let mut v = vector::empty<u64>();
    push_back(&mut v, 10);
    let len = length(&v);
}
```

### Import with Alias

```move
use std::vector as vec;

public fun example() {
    let v = vec::empty<u64>();
}
```

## Use Statement Patterns

### Multiple Imports

```move
use std::vector;
use std::option;
use kanari_system::coin;
use kanari_system::transfer;
```

### Grouped Imports

```move
use std::{
    vector,
    option,
    string,
};
```

### Selective Import with Aliases

```move
use kanari_system::{
    coin,
    transfer,
    object::{self as obj},
    tx_context::TxContext,
};
```

## Importing Different Items

### Functions

```move
use std::vector::{push_back, pop_back};

public fun example() {
    let mut v = vector[1u64, 2, 3];
    push_back(&mut v, 4);
    let last = pop_back(&mut v);
}
```

### Structs

```move
use kanari_system::object::UID;

struct MyResource has key, store {
    id: UID,
    value: u64,
}
```

### Constants

```move
use kanari_system::coin::MIST_PER_KARI;

public fun convert_kari_to_mist(kari: u64): u64 {
    kari * MIST_PER_KARI
}
```

### Type Parameters

```move
use kanari_system::coin::Coin;

public fun example(coins: Coin<KANARI>) {
    // Use Coin type
}
```

## Aliases

### Module Aliases

```move
use std::vector as v;
use kanari_system::tx_context as ctx;

public fun example() {
    let vec = v::empty<u64>();
    // Can't get sender from ctx directly, need TxContext
}
```

### Function Aliases

```move
use std::vector::{
    push_back as add,
    pop_back as remove_last,
    length as size,
};

public fun example() {
    let mut v = vector::empty<u64>();
    add(&mut v, 10);
    let len = size(&v);
}
```

### Struct Aliases

```move
use kanari_system::object::UID as ObjectId;

struct MyObject has key, store {
    id: ObjectId,
}
```

## Common Use Patterns

### Standard Library Imports

```move
use std::vector;
use std::option;
use std::string;
use std::ascii;
use std::hash;
use std::bcs;
```

### Kanari System Imports

```move
use kanari_system::coin;
use kanari_system::transfer;
use kanari_system::object;
use kanari_system::tx_context::TxContext;
use kanari_system::event;
use kanari_system::clock;
use kanari_system::table;
use kanari_system::bag;
```

### Combined Imports

```move
use kanari_system::{
    coin,
    transfer,
    object::{self, UID},
    tx_context::TxContext,
    event,
    clock,
};
```

## Best Practices

### 1. Organize Imports

```move
// Group by source
use std::vector;
use std::option;

use kanari_system::coin;
use kanari_system::transfer;

use crate::utils;
use crate::types;
```

### 2. Use Descriptive Aliases

```move
// Bad: Unclear alias
use std::vector as v;

// Good: Clear when needed
use very_long_module_name as short;

// Best: No alias if not needed
use std::vector;
```

### 3. Avoid Wildcard Imports

```move
// Bad: Imports everything (not supported in Move anyway)
// use std::*;

// Good: Import specific items
use std::vector::{push_back, pop_back};
```

### 4. Consistent Style

```move
// Choose one style and stick with it

// Style 1: Individual imports
use std::vector;
use std::option;
use std::string;

// Style 2: Grouped imports
use std::{
    vector,
    option,
    string,
};
```

## Practical Examples

### Token Module

```move
module my_project::token {
    use std::vector;
    use std::option;
    use kanari_system::coin;
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    
    struct TOKEN has drop {}
    
    public fun initialize(ctx: &mut TxContext) {
        let (cap, meta) = coin::create_currency<TOKEN>(
            TOKEN {},
            9,
            b"MTK",
            b"My Token",
            b"Description",
            option::none(),
            ctx
        );
        
        transfer::public_freeze_object(meta);
        transfer::public_transfer(cap, tx_context::sender(ctx));
    }
}
```

### NFT Collection

```move
module my_project::nft {
    use kanari_system::collection;
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    use kanari_system::object::UID;
    
    public fun create_collection(
        name: vector<u8>,
        description: vector<u8>,
        max_supply: u64,
        ctx: &mut TxContext
    ) {
        let (col, cap) = collection::create_collection(
            name,
            description,
            max_supply,
            ctx
        );
        
        transfer::public_transfer(col, tx_context::sender(ctx));
        transfer::public_transfer(cap, tx_context::sender(ctx));
    }
}
```

### DeFi Protocol

```move
module my_project::dex {
    use kanari_system::coin::{self, Coin};
    use kanari_system::balance;
    use kanari_system::table::{self, Table};
    use kanari_system::tx_context::TxContext;
    
    struct Pool has key, store {
        id: UID,
        token_a_balance: balance::Balance<TOKEN_A>,
        token_b_balance: balance::Balance<TOKEN_B>,
        total_shares: u64,
    }
    
    public fun swap(
        pool: &mut Pool,
        input_coins: Coin<TOKEN_A>,
        ctx: &mut TxContext
    ): Coin<TOKEN_B> {
        // Implementation
    }
}
```

## Import Resolution

### Module Paths

```move
// Absolute path
use std::vector;

// Relative path (within same package)
use crate::utils::helper;

// External package
use some_package::module;
```

### Name Conflicts

```move
// Two modules with same function name
use module_a::process;
use module_b::process as process_b;

public fun example() {
    process();      // From module_a
    process_b();    // From module_b
}
```

## Testing with Imports

```move
#[test]
fun test_with_imports() {
    use std::vector;
    
    let v = vector[1u64, 2, 3];
    assert!(vector::length(&v) == 3, 0);
}

#[test_only]
use test_helpers::{setup, teardown};

#[test]
fun test_with_helpers() {
    let env = setup();
    // Test logic
    teardown(env);
}
```

## Common Errors

### Undefined Import

```move
// Wrong: Module doesn't exist
// use nonexistent::module; // Error!

// Correct: Use existing module
use std::vector;
```

### Item Not Found

```move
// Wrong: Function doesn't exist in module
// use std::vector::nonexistent_function; // Error!

// Correct: Use existing function
use std::vector::push_back;
```

### Circular Dependencies

```move
// Don't create circular imports
// Module A uses Module B
// Module B uses Module A
// This will cause compilation errors
```

## Performance Considerations

- `use` statements have zero runtime cost
- They're compile-time only
- No performance difference between aliased and non-aliased imports
- Choose clarity over brevity

## Next Steps

- Learn about [Modules](modules-and-scripts.md) for organization
- Study [Packages](packages.md) for dependency management
- Explore [Standard Library](standard-library.md) modules
