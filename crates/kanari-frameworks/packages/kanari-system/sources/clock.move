// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// APIs for accessing time from move calls, via the `Clock`: a unique
/// system object that is created during genesis.
///
/// Custody model:
/// - The Clock is owned by the system address `@0x0` (see `create`).
/// - Reads are public: any transaction holding the object, or any contract
///   via `timestamp_ms_by_address` (which loads it with `borrow_global`),
///   can call `timestamp_ms`.
/// - Writes go only through `consensus_commit_prologue`, which requires
///   `sender == @0x0` as defense in depth: the runtime routes validator
///   system transactions from `@0x0`, and grants `borrow_global_mut`
///   authorization only to those transactions.
module kanari_system::clock {
    use kanari_system::object::{Self, UID};
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::transfer;

    /// Sender is not @0x0 the system address.
    const E_NOT_SYSTEM_ADDRESS: u64 = 0;

    /// Timestamp is not monotonic (not greater than or equal to current time)
    const E_TIMESTAMP_NOT_MONOTONIC: u64 = 1;

    /// Singleton shared object that exposes time to Move calls.
    struct Clock has key, store {
        id: UID,
        timestamp_ms: u64
    }

    /// The `clock`'s current timestamp as a running total of
    /// milliseconds since an arbitrary point in the past.
    public fun timestamp_ms(clock: &Clock): u64 {
        clock.timestamp_ms
    }

    /// Read the Clock at a known address without holding the object.
    /// Aborts with the object layer's `E_OBJECT_NOT_FOUND` when no Clock
    /// exists at `clock_addr` in the current execution context.
    public fun timestamp_ms_by_address(clock_addr: address): u64 {
        timestamp_ms(object::borrow_global<Clock>(clock_addr))
    }

    /// Create and share the singleton Clock -- this function is
    /// called exactly once, during genesis.
    /// The Clock is transferred to the system address `@0x0`, which is the
    /// only sender the runtime will ever authorize to mutate it (see
    /// `consensus_commit_prologue`). Reads stay public via `borrow_global`.
    public fun create(ctx: &mut TxContext) {
        assert!(tx_context::sender(ctx) == @0x0, E_NOT_SYSTEM_ADDRESS);

        let clock = Clock { id: object::new(ctx), timestamp_ms: 0 };

        object::save_object(&clock);

        // Custody passes to the system address; Move linearity guarantees
        // this function cannot retain a copy.
        transfer::public_transfer(clock, @0x0);
    }

    /// System call: the validator calls this at every block boundary with the
    /// block timestamp. Only a transaction sent from `@0x0` may call it, and
    /// time must never move backwards.
    public fun consensus_commit_prologue(
        clock: &mut Clock, timestamp_ms: u64, ctx: &TxContext
    ) {
        // Requires that the call be made only through the System Validator.
        assert!(tx_context::sender(ctx) == @0x0, E_NOT_SYSTEM_ADDRESS);
        // Ensure that the new timestamp is greater than or equal to the current one
        // to maintain monotonicity of time on the blockchain
        assert!(timestamp_ms >= clock.timestamp_ms, E_TIMESTAMP_NOT_MONOTONIC);
        clock.timestamp_ms = timestamp_ms;
    }

    // =================================================================
    // Functions for Testing
    // =================================================================
    #[test_only]
    public fun create_for_testing(ctx: &mut TxContext): Clock {
        Clock { id: object::new(ctx), timestamp_ms: 0 }
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

