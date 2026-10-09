// Sample module exercising the core constructs the formatter supports:
// uses, constants, structs, functions, visibility and entry modifiers.
module test::basics {
    use std::option::{Self, Option};
    use std::vector;

    /// Error codes
    const E_NOT_FOUND: u64 = 0;
    const E_INVALID: u64 = 1;

    struct Counter has key, store {
        id: UID,
        value: u64,
    }

    public fun new(ctx: &mut TxContext): Counter {
        Counter {
            id: object::new(ctx),
            value: 0,
        }
    }

    public entry fun bump(counter: &mut Counter, amount: u64) {
        counter.value = counter.value + amount;
    }
}
