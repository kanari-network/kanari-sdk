// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0
#[test_only]
module kanari_system::coin_tests {
    use kanari_system::coin::{Self, Coin, TreasuryCap, CoinMetadata};
    use kanari_system::transfer;
    use kanari_system::tx_context;
    use std::option;
    use std::string;

    struct TEST has drop {}

    fun setup(ctx: &mut tx_context::TxContext): (TreasuryCap<TEST>, CoinMetadata<TEST>) {
        coin::create_currency(
            TEST {},
            6,
            b"TEST",
            b"Test Coin",
            b"coin tests",
            option::none(),
            ctx
        )
    }

    fun cleanup(cap: TreasuryCap<TEST>, meta: CoinMetadata<TEST>) {
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(meta);
    }

    #[test]
    fun test_mint_burn_supply() {
        let ctx = tx_context::dummy();
        let (cap, meta) = setup(&mut ctx);
        let c = coin::mint(&mut cap, 1000, &mut ctx);
        assert!(coin::total_supply(&cap) == 1000, 0);
        assert!(coin::value(&c) == 1000, 1);

        // Burn half: returns burned value, supply drops, object deleted.
        let half = coin::split(&mut c, 400, &mut ctx);
        let burned = coin::burn(&mut cap, half);
        assert!(burned == 400, 2);
        assert!(coin::total_supply(&cap) == 600, 3);
        assert!(coin::value(&c) == 600, 4);

        transfer::public_transfer(c, @0x1);
        cleanup(cap, meta);
    }

    #[test]
    fun test_split_join_roundtrip() {
        let ctx = tx_context::dummy();
        let (cap, meta) = setup(&mut ctx);
        let a = coin::mint(&mut cap, 1000, &mut ctx);
        let b = coin::split(&mut a, 300, &mut ctx);
        assert!(coin::value(&a) == 700, 0);
        assert!(coin::value(&b) == 300, 1);
        // Join deletes `b`'s object and merges value.
        coin::join(&mut a, b);
        assert!(coin::value(&a) == 1000, 2);
        transfer::public_transfer(a, @0x1);
        cleanup(cap, meta);
    }

    #[test]
    fun test_zero_and_destroy_zero() {
        let ctx = tx_context::dummy();
        let z: Coin<TEST> = coin::zero(&mut ctx);
        assert!(coin::value(&z) == 0, 0);
        coin::destroy_zero(z);
        let (cap, meta) = setup(&mut ctx);
        cleanup(cap, meta);
    }

    #[test]
    #[expected_failure(location = kanari_system::coin, abort_code = 1)]
    fun test_destroy_zero_nonzero_fails() {
        // EInvalidArg in coin module.
        let ctx = tx_context::dummy();
        let (cap, meta) = setup(&mut ctx);
        let c = coin::mint(&mut cap, 100, &mut ctx);
        coin::destroy_zero(c);
        cleanup(cap, meta);
    }

    #[test]
    fun test_into_from_balance() {
        let ctx = tx_context::dummy();
        let (cap, meta) = setup(&mut ctx);
        let c = coin::mint(&mut cap, 250, &mut ctx);
        let bal = coin::into_balance(c);
        assert!(kanari_system::balance::value(&bal) == 250, 0);
        let c2 = coin::from_balance(bal, &mut ctx);
        assert!(coin::value(&c2) == 250, 1);
        transfer::public_transfer(c2, @0x1);
        cleanup(cap, meta);
    }

    #[test]
    fun test_divide_into_n_and_metadata() {
        let ctx = tx_context::dummy();
        let (cap, meta) = setup(&mut ctx);
        let c = coin::mint(&mut cap, 1000, &mut ctx);
        let parts = coin::divide_into_n(&mut c, 4, &mut ctx);
        assert!(std::vector::length(&parts) == 3, 0);
        assert!(coin::value(&c) == 250, 1); // remainder stays
        let i = 0;
        while (i < 3) {
            let part = std::vector::pop_back(&mut parts);
            assert!(coin::value(&part) == 250, 2);
            transfer::public_transfer(part, @0x1);
            i = i + 1;
        };
        std::vector::destroy_empty(parts);
        transfer::public_transfer(c, @0x1);

        coin::update_name(&cap, &mut meta, string::utf8(b"Renamed"));
        coin::update_symbol(&cap, &mut meta, std::ascii::string(b"RNM"));
        coin::update_description(&cap, &mut meta, string::utf8(b"desc"));
        coin::update_icon_url(&cap, &mut meta, option::none());
        cleanup(cap, meta);
    }

    #[test]
    #[expected_failure(location = kanari_system::coin, abort_code = 3)]
    fun test_mint_zero_fails() {
        // EZERO_AMOUNT in coin module.
        let ctx = tx_context::dummy();
        let (cap, meta) = setup(&mut ctx);
        let c = coin::mint(&mut cap, 0, &mut ctx);
        transfer::public_transfer(c, @0x1);
        cleanup(cap, meta);
    }
}

