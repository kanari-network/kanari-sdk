// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[test_only]
module kanari_system::clock_tests {
    use kanari_system::clock;
    use kanari_system::tx_context;

    #[test]
    fun test_testing_helpers() {
        let ctx = tx_context::dummy();
        let c = clock::create_for_testing(&mut ctx);
        assert!(clock::timestamp_ms(&c) == 0, 0);
        clock::increment_for_testing(&mut c, 500);
        assert!(clock::timestamp_ms(&c) == 500, 1);
        clock::set_for_testing(&mut c, 1000);
        assert!(clock::timestamp_ms(&c) == 1000, 2);
        clock::destroy_for_testing(c);
    }

    #[test]
    #[expected_failure(abort_code = 1)]
    fun test_set_backwards_fails() {
        let ctx = tx_context::dummy();
        let c = clock::create_for_testing(&mut ctx);
        clock::increment_for_testing(&mut c, 500);
        clock::set_for_testing(&mut c, 100);
        clock::destroy_for_testing(c);
    }

    #[test]
    fun test_prologue_advances_system_clock() {
        let sys = tx_context::new_from_hint(@0x0, 1, 0, 0, 0);
        let ctx = tx_context::dummy();
        let c = clock::create_for_testing(&mut ctx);
        clock::consensus_commit_prologue(&mut c, 10_000, &sys);
        assert!(clock::timestamp_ms(&c) == 10_000, 0);
        // Equal timestamps are allowed (monotonic, not strictly increasing).
        clock::consensus_commit_prologue(&mut c, 10_000, &sys);
        assert!(clock::timestamp_ms(&c) == 10_000, 1);
        clock::destroy_for_testing(c);
    }

    #[test]
    #[expected_failure(abort_code = 0)]
    fun test_prologue_rejects_non_system_sender() {
        // E_NOT_SYSTEM_ADDRESS in clock module.
        let user = tx_context::new_from_hint(@0x1, 1, 0, 0, 0);
        let ctx = tx_context::dummy();
        let c = clock::create_for_testing(&mut ctx);
        clock::consensus_commit_prologue(&mut c, 10_000, &user);
        clock::destroy_for_testing(c);
    }

    #[test]
    #[expected_failure(abort_code = 1)]
    fun test_prologue_rejects_time_travel() {
        // E_TIMESTAMP_NOT_MONOTONIC in clock module.
        let sys = tx_context::new_from_hint(@0x0, 1, 0, 0, 0);
        let ctx = tx_context::dummy();
        let c = clock::create_for_testing(&mut ctx);
        clock::consensus_commit_prologue(&mut c, 10_000, &sys);
        clock::consensus_commit_prologue(&mut c, 9_999, &sys);
        clock::destroy_for_testing(c);
    }

    #[test]
    fun test_create_transfers_to_system() {
        // Genesis path: sender must be @0x0; the Clock leaves to @0x0.
        let sys = tx_context::new_from_hint(@0x0, 1, 0, 0, 0);
        clock::create(&mut sys);
    }

    #[test]
    #[expected_failure(abort_code = 0)]
    fun test_create_rejects_non_system_sender() {
        let user = tx_context::new_from_hint(@0x1, 1, 0, 0, 0);
        clock::create(&mut user);
    }
}
