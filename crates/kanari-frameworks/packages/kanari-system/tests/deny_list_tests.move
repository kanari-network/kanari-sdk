// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0
#[test_only]
module kanari_system::deny_list_tests {
    use kanari_system::deny_list;
    use kanari_system::tx_context;

    // Non-reserved sample addresses (`@0x0`–`@0xF` can never be denied).
    const A1: address = @0x100;
    const A2: address = @0x200;
    const A3: address = @0x300;

    // =================================================================
    // Tests: Basic Operations - Add and Remove
    // =================================================================

    // Test: A fresh deny list denies nothing
    #[test]
    fun test_new_denylist_is_empty() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);

        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 0);
        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, A2), 1);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // Test: Adding an address to an empty deny list
    #[test]
    fun test_add_address_to_empty_list() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);

        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 0);
        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, A2), 1);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // Test: Adding multiple addresses
    #[test]
    fun test_add_multiple_addresses() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A2, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A3, ctx);

        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 0);
        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A2), 1);
        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A3), 2);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // Test: Removing an existing address
    #[test]
    fun test_remove_existing_address() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A2, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A3, ctx);

        kanari_system::coin::deny_list_remove<u64>(&mut denylist, &cap, A2, ctx);

        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 0);
        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, A2), 1);
        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A3), 2);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // Test: Removing the first address
    #[test]
    fun test_remove_first_address() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A2, ctx);

        kanari_system::coin::deny_list_remove<u64>(&mut denylist, &cap, A1, ctx);

        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 0);
        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A2), 1);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // Test: Removing the last address
    #[test]
    fun test_remove_last_address() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A2, ctx);

        kanari_system::coin::deny_list_remove<u64>(&mut denylist, &cap, A2, ctx);

        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 0);
        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, A2), 1);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // =================================================================
    // Tests: Duplicate Prevention
    // =================================================================

    // Test: Adding a duplicate address keeps a single entry
    #[test]
    fun test_add_duplicate_address_prevented() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);

        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 0);
        kanari_system::coin::deny_list_remove<u64>(&mut denylist, &cap, A1, ctx);
        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 1);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // Test: Multiple duplicate additions with other addresses
    #[test]
    fun test_mixed_duplicates_and_unique() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A2, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A3, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A2, ctx);

        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 0);
        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A2), 1);
        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A3), 2);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // =================================================================
    // Tests: Edge Cases
    // =================================================================

    // Test: Removing from an empty list aborts `ENotDenied`
    #[test]
    #[expected_failure(location = kanari_system::deny_list, abort_code = 1)]
    fun test_remove_from_empty_list() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_remove<u64>(&mut denylist, &cap, A1, ctx);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // Test: Removing a non-existent address aborts `ENotDenied`
    #[test]
    #[expected_failure(location = kanari_system::deny_list, abort_code = 1)]
    fun test_remove_non_existent_address() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);
        kanari_system::coin::deny_list_remove<u64>(&mut denylist, &cap, A2, ctx);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // Test: Add and remove same address multiple times
    #[test]
    fun test_add_remove_cycle() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);
        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 0);

        kanari_system::coin::deny_list_remove<u64>(&mut denylist, &cap, A1, ctx);
        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 1);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);
        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 2);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // Test: Remove all addresses one by one
    #[test]
    fun test_remove_all_addresses_sequentially() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A2, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A3, ctx);

        kanari_system::coin::deny_list_remove<u64>(&mut denylist, &cap, A3, ctx);
        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, A3), 0);

        kanari_system::coin::deny_list_remove<u64>(&mut denylist, &cap, A2, ctx);
        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, A2), 1);

        kanari_system::coin::deny_list_remove<u64>(&mut denylist, &cap, A1, ctx);
        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 2);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // =================================================================
    // Tests: Contains Function
    // =================================================================

    // Test: contains returns true for existing address
    #[test]
    fun test_contains_existing_address() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);

        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 0);
        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, A2), 1);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // Test: contains returns false after removal
    #[test]
    fun test_contains_after_removal() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, A1, ctx);
        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 0);

        kanari_system::coin::deny_list_remove<u64>(&mut denylist, &cap, A1, ctx);
        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, A1), 1);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // =================================================================
    // Tests: Large Address Values
    // =================================================================

    // Test: Using full-length addresses
    #[test]
    fun test_with_full_length_addresses() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        let addr1 = @0x0000000000000000000000000000000000000000000000000000000000000010;
        let addr2 = @0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, addr1, ctx);
        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, addr2, ctx);

        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, addr1), 0);
        assert!(kanari_system::coin::deny_list_contains<u64>(&denylist, addr2), 1);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // =================================================================
    // Tests: Reserved addresses and genesis constructor
    // =================================================================

    // Test: Denying a reserved system address aborts `EInvalidAddress`
    #[test]
    #[expected_failure(location = kanari_system::deny_list, abort_code = 2)]
    fun test_reserved_address_add_aborts() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);
        let cap = deny_list::new_denycap_for_testing<u64>(ctx);

        kanari_system::coin::deny_list_add<u64>(&mut denylist, &cap, @0x2, ctx);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // Test: Reserved addresses are never reported as denied
    #[test]
    fun test_reserved_address_contains_false() {
        let ctx = &mut tx_context::dummy();
        let denylist = deny_list::new_for_testing(ctx);

        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, @0x0), 0);
        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, @0x2), 1);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // Test: The genesis constructor shares a usable singleton
    #[test]
    fun test_create_for_test_shares_singleton() {
        let ctx = &mut tx_context::dummy();
        deny_list::create_for_test(ctx);
    }

    // =================================================================
    // Tests: Regulated transfers (production contains + enforced send)
    // =================================================================
    struct REG has drop {}

    #[test]
    fun test_type_key_format_pinned() {
        // Pins the exact deny-key bytes the runtime's transfer-time
        // enforcement must reproduce for the same coin type.
        let key = kanari_system::coin::type_key_for_testing<REG>();
        assert!(
            key == b"0000000000000000000000000000000000000000000000000000000000000002::deny_list_tests::REG",
            0
        );
    }

    fun setup_regulated(
        ctx: &mut tx_context::TxContext,
    ): (
        kanari_system::coin::TreasuryCap<REG>,
        kanari_system::deny_list::DenyCap<REG>,
        kanari_system::coin::CoinMetadata<REG>
    ) {
        kanari_system::coin::create_regulated_currency(
            REG {},
            6,
            b"REG",
            b"Regulated",
            b"regulated tests",
            std::option::none(),
            ctx
        )
    }

    #[test]
    fun test_regulated_send_allowed_then_denied() {
        let ctx = &mut tx_context::dummy();
        let (cap, denycap, meta) = setup_regulated(ctx);
        let denylist = deny_list::new_for_testing(ctx);
        kanari_system::coin::deny_list_add<REG>(&mut denylist, &denycap, @0xBAD, ctx);
        assert!(kanari_system::coin::deny_list_contains<REG>(&denylist, @0xBAD), 0);
        assert!(!kanari_system::coin::deny_list_contains<REG>(&denylist, @0xCAFE), 1);

        let coin_val = kanari_system::coin::mint(&mut cap, 1000, ctx);
        let c = &mut coin_val;
        // Allowed recipient goes through.
        kanari_system::pay::split_and_transfer_checked(c, 100, @0xCAFE, &denylist, ctx);
        assert!(kanari_system::coin::value(c) == 900, 2);

        // Remove from list => allowed again.
        kanari_system::coin::deny_list_remove<REG>(&mut denylist, &denycap, @0xBAD, ctx);
        kanari_system::pay::split_and_transfer_checked(c, 100, @0xBAD, &denylist, ctx);
        assert!(kanari_system::coin::value(c) == 800, 3);

        kanari_system::transfer::public_transfer(
            kanari_system::coin::split(c, 800, ctx),
            @0x100
        );
        kanari_system::coin::destroy_zero(coin_val);
        kanari_system::transfer::public_freeze_object(cap);
        kanari_system::transfer::public_freeze_object(denycap);
        kanari_system::transfer::public_freeze_object(meta);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    #[test]
    #[expected_failure(location = kanari_system::pay, abort_code = 1)]
    fun test_regulated_send_to_denied_aborts() {
        // EDENIED in pay module.
        let ctx = &mut tx_context::dummy();
        let (cap, denycap, meta) = setup_regulated(ctx);
        let denylist = deny_list::new_for_testing(ctx);
        kanari_system::coin::deny_list_add<REG>(&mut denylist, &denycap, @0xBAD, ctx);

        let coin_val = kanari_system::coin::mint(&mut cap, 1000, ctx);
        let c = &mut coin_val;
        kanari_system::pay::split_and_transfer_checked(c, 100, @0xBAD, &denylist, ctx);
        // Unreachable at runtime (aborts above), but satisfies the borrow checker.
        kanari_system::transfer::public_transfer(coin_val, @0x100);
        kanari_system::transfer::public_freeze_object(cap);
        kanari_system::transfer::public_freeze_object(denycap);
        kanari_system::transfer::public_freeze_object(meta);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    #[test]
    #[expected_failure(location = kanari_system::pay, abort_code = 4)]
    fun test_regulated_checked_zero_amount_aborts() {
        // EZERO_AMOUNT in pay module: the deny check passes (recipient allowed)
        // and the amount guard rejects a zero transfer.
        let ctx = &mut tx_context::dummy();
        let (cap, denycap, meta) = setup_regulated(ctx);
        let denylist = deny_list::new_for_testing(ctx);

        let coin_val = kanari_system::coin::mint(&mut cap, 1000, ctx);
        let c = &mut coin_val;
        kanari_system::pay::split_and_transfer_checked(c, 0, @0xCAFE, &denylist, ctx);
        kanari_system::transfer::public_transfer(coin_val, @0x100);
        kanari_system::transfer::public_freeze_object(cap);
        kanari_system::transfer::public_freeze_object(denycap);
        kanari_system::transfer::public_freeze_object(meta);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    #[test]
    #[expected_failure(location = kanari_system::pay, abort_code = 1)]
    fun test_regulated_checked_denied_takes_precedence_over_zero_amount() {
        // EDENIED is checked before the amount guard, so a denied recipient
        // aborts with EDENIED even for a zero-amount transfer.
        let ctx = &mut tx_context::dummy();
        let (cap, denycap, meta) = setup_regulated(ctx);
        let denylist = deny_list::new_for_testing(ctx);
        kanari_system::coin::deny_list_add<REG>(&mut denylist, &denycap, @0xBAD, ctx);

        let coin_val = kanari_system::coin::mint(&mut cap, 1000, ctx);
        let c = &mut coin_val;
        kanari_system::pay::split_and_transfer_checked(c, 0, @0xBAD, &denylist, ctx);
        kanari_system::transfer::public_transfer(coin_val, @0x100);
        kanari_system::transfer::public_freeze_object(cap);
        kanari_system::transfer::public_freeze_object(denycap);
        kanari_system::transfer::public_freeze_object(meta);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    #[test]
    fun test_regulated_join_vec_and_transfer_allowed() {
        let ctx = &mut tx_context::dummy();
        let (cap, denycap, meta) = setup_regulated(ctx);
        let denylist = deny_list::new_for_testing(ctx);
        kanari_system::coin::deny_list_add<REG>(&mut denylist, &denycap, @0xBAD, ctx);

        let coin_a = kanari_system::coin::mint(&mut cap, 10, ctx);
        let coin_b = kanari_system::coin::mint(&mut cap, 20, ctx);
        let coins = std::vector::empty<kanari_system::coin::Coin<REG>>();
        std::vector::push_back(&mut coins, coin_a);
        std::vector::push_back(&mut coins, coin_b);

        // Allowed receiver goes through (joined coin transferred to @0xCAFE).
        kanari_system::pay::join_vec_and_transfer_checked(coins, @0xCAFE, &denylist, ctx);

        kanari_system::transfer::public_freeze_object(cap);
        kanari_system::transfer::public_freeze_object(denycap);
        kanari_system::transfer::public_freeze_object(meta);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    #[test]
    #[expected_failure(location = kanari_system::pay, abort_code = 1)]
    fun test_regulated_join_vec_and_transfer_denied_aborts() {
        // EDENIED in pay module: the checked join rejects a denied receiver.
        let ctx = &mut tx_context::dummy();
        let (cap, denycap, meta) = setup_regulated(ctx);
        let denylist = deny_list::new_for_testing(ctx);
        kanari_system::coin::deny_list_add<REG>(&mut denylist, &denycap, @0xBAD, ctx);

        let coin_a = kanari_system::coin::mint(&mut cap, 10, ctx);
        let coin_b = kanari_system::coin::mint(&mut cap, 20, ctx);
        let coins = std::vector::empty<kanari_system::coin::Coin<REG>>();
        std::vector::push_back(&mut coins, coin_a);
        std::vector::push_back(&mut coins, coin_b);
        kanari_system::pay::join_vec_and_transfer_checked(coins, @0xBAD, &denylist, ctx);
        // Unreachable at runtime (aborts above).
        kanari_system::transfer::public_freeze_object(cap);
        kanari_system::transfer::public_freeze_object(denycap);
        kanari_system::transfer::public_freeze_object(meta);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    // =================================================================
    // Tests: V2 global pause (fail-closed kill-switch per coin type)
    // =================================================================
    #[test]
    #[expected_failure(location = kanari_system::pay, abort_code = 1)]
    fun test_pause_blocks_checked_transfer() {
        // EDENIED: @0xCAFE is not denied, but REG is paused.
        let ctx = &mut tx_context::dummy();
        let (cap, denycap, meta) = setup_regulated(ctx);
        let denylist = deny_list::new_for_testing(ctx);
        kanari_system::coin::deny_list_pause<REG>(&mut denylist, &denycap, ctx);
        assert!(kanari_system::coin::deny_list_paused<REG>(&denylist), 0);

        let coin_val = kanari_system::coin::mint(&mut cap, 1000, ctx);
        let c = &mut coin_val;
        kanari_system::pay::split_and_transfer_checked(c, 100, @0xCAFE, &denylist, ctx);
        kanari_system::transfer::public_transfer(coin_val, @0x100);
        kanari_system::transfer::public_freeze_object(cap);
        kanari_system::transfer::public_freeze_object(denycap);
        kanari_system::transfer::public_freeze_object(meta);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    #[test]
    fun test_unpause_restores_checked_transfer() {
        let ctx = &mut tx_context::dummy();
        let (cap, denycap, meta) = setup_regulated(ctx);
        let denylist = deny_list::new_for_testing(ctx);
        kanari_system::coin::deny_list_pause<REG>(&mut denylist, &denycap, ctx);
        assert!(kanari_system::coin::deny_list_paused<REG>(&denylist), 0);
        kanari_system::coin::deny_list_unpause<REG>(&mut denylist, &denycap, ctx);
        assert!(!kanari_system::coin::deny_list_paused<REG>(&denylist), 1);

        let coin_val = kanari_system::coin::mint(&mut cap, 1000, ctx);
        let c = &mut coin_val;
        kanari_system::pay::split_and_transfer_checked(c, 100, @0xCAFE, &denylist, ctx);
        assert!(kanari_system::coin::value(c) == 900, 2);
        kanari_system::transfer::public_transfer(
            kanari_system::coin::split(c, 900, ctx),
            @0x100
        );
        kanari_system::coin::destroy_zero(coin_val);
        kanari_system::transfer::public_freeze_object(cap);
        kanari_system::transfer::public_freeze_object(denycap);
        kanari_system::transfer::public_freeze_object(meta);
        kanari_system::transfer::public_freeze_object(denylist);
    }

    #[test]
    fun test_pause_is_per_type_and_unpause_missing_is_noop() {
        let ctx = &mut tx_context::dummy();
        let (cap, denycap, meta) = setup_regulated(ctx);
        let denylist = deny_list::new_for_testing(ctx);
        // Unpausing a type that was never paused is a no-op, not an abort.
        kanari_system::coin::deny_list_unpause<REG>(&mut denylist, &denycap, ctx);
        assert!(!kanari_system::coin::deny_list_paused<REG>(&denylist), 0);

        kanari_system::coin::deny_list_pause<REG>(&mut denylist, &denycap, ctx);
        assert!(kanari_system::coin::deny_list_paused<REG>(&denylist), 1);
        // Other coin types are unaffected by REG's pause.
        assert!(!kanari_system::coin::deny_list_paused<u64>(&denylist), 2);
        assert!(!kanari_system::coin::deny_list_contains<u64>(&denylist, @0xCAFE), 3);
        kanari_system::transfer::public_freeze_object(cap);
        kanari_system::transfer::public_freeze_object(denycap);
        kanari_system::transfer::public_freeze_object(meta);
        kanari_system::transfer::public_freeze_object(denylist);
    }
}
