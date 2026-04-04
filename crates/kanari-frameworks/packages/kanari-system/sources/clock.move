// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// APIs for accessing time from move calls, via the `Clock`: a unique
/// shared object that is created during genesis.
module kanari_system::clock {
    use kanari_system::object::{Self, UID};
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::transfer;

    /// Sender is not @0x0 the system address.
    const E_NOT_SYSTEM_ADDRESS: u64 = 0;

    /// Singleton shared object that exposes time to Move calls.
    struct Clock has key, store {
        id: UID,
        timestamp_ms: u64,
    }

    /// The `clock`'s current timestamp as a running total of
    /// milliseconds since an arbitrary point in the past.
    public fun timestamp_ms(clock: &Clock): u64 {
        clock.timestamp_ms
    }

    /// Create and share the singleton Clock -- this function is
    /// called exactly once, during genesis.
    public fun create(ctx: &mut TxContext) {
        assert!(tx_context::sender(ctx) == @0x0, E_NOT_SYSTEM_ADDRESS);

        let clock = Clock {
            id: object::new(ctx), 
            timestamp_ms: 0,
        };

        object::save_object(&clock); 
        
       // 🚨 Transfer ownership to System Address (@0x0)
       // To properly clear the clock value from the function according to Move rules.
        transfer::public_transfer(clock, @0x0);
    }

    /// System call: Validator (the Rust node) will call this function every time the block is closed.
    public fun consensus_commit_prologue(clock: &mut Clock, timestamp_ms: u64, ctx: &TxContext) {
        // Requires that the call be made only through the System Validator.
        assert!(tx_context::sender(ctx) == @0x0, E_NOT_SYSTEM_ADDRESS);
        // Ensure that the new timestamp is greater than or equal to the current one
        // to maintain monotonicity of time on the blockchain
        assert!(timestamp_ms >= clock.timestamp_ms, 1);
        clock.timestamp_ms = timestamp_ms;
    }

    // =================================================================
    // Functions for Testing
    // =================================================================

    #[test_only]
    public fun create_for_testing(ctx: &mut TxContext): Clock {
        Clock {
            id: object::new(ctx),
            timestamp_ms: 0,
        }
    }

    #[test_only]
    public fun increment_for_testing(clock: &mut Clock, tick: u64) {
        clock.timestamp_ms = clock.timestamp_ms + tick;
    }

    #[test_only]
    public fun set_for_testing(clock: &mut Clock, timestamp_ms: u64) {
        assert!(timestamp_ms >= clock.timestamp_ms, 1);
        clock.timestamp_ms = timestamp_ms;
    }

    #[test_only]
    public fun destroy_for_testing(clock: Clock) {
        let Clock { id, timestamp_ms: _ } = clock;
        object::delete(id);
    }
}