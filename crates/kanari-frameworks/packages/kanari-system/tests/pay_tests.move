// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[test_only]
module kanari_system::pay_tests {
    use kanari_system::coin::{Self, TreasuryCap, CoinMetadata};
    use kanari_system::pay;
    use kanari_system::transfer;
    use kanari_system::tx_context;

    struct TEST has drop {}

    fun setup(ctx: &mut tx_context::TxContext): (TreasuryCap<TEST>, CoinMetadata<TEST>) {
        coin::create_currency(
            TEST {},
            6,
            b"TEST",
            b"Test Coin",
            b"pay tests",
            std::option::none(),
            ctx,
        )
    }

    fun cleanup(cap: TreasuryCap<TEST>, meta: CoinMetadata<TEST>) {
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(meta);
    }

    #[test]
    fun test_split_and_transfer() {
        let ctx = tx_context::dummy();
        let (cap, meta) = setup(&mut ctx);
        let coin_val = coin::mint(&mut cap, 1000, &mut ctx);
        let c = &mut coin_val;
        pay::split_and_transfer(c, 300, @0xA, &mut ctx);
        assert!(coin::value(c) == 700, 0);
        transfer::public_transfer(coin_val, @0x1);
        cleanup(cap, meta);
    }

    #[test]
    fun test_split_vec() {
        let ctx = tx_context::dummy();
        let (cap, meta) = setup(&mut ctx);
        let coin_val = coin::mint(&mut cap, 1000, &mut ctx);
        let c = &mut coin_val;
        let amounts = std::vector::empty<u64>();
        std::vector::push_back(&mut amounts, 100);
        std::vector::push_back(&mut amounts, 200);
        pay::split_vec(c, amounts, &mut ctx);
        // 100 + 200 left the coin, remainder stays.
        assert!(coin::value(c) == 700, 0);
        transfer::public_transfer(coin_val, @0x1);
        cleanup(cap, meta);
    }

    #[test]
    fun test_join_vec_and_transfer() {
        let ctx = tx_context::dummy();
        let (cap, meta) = setup(&mut ctx);
        let v = std::vector::empty<coin::Coin<TEST>>();
        std::vector::push_back(&mut v, coin::mint(&mut cap, 100, &mut ctx));
        std::vector::push_back(&mut v, coin::mint(&mut cap, 200, &mut ctx));
        std::vector::push_back(&mut v, coin::mint(&mut cap, 300, &mut ctx));
        // 600 lands at @0xB as one object.
        pay::join_vec_and_transfer(v, @0xB);
        cleanup(cap, meta);
    }

    #[test]
    fun test_divide_and_keep() {
        let ctx = tx_context::dummy();
        let (cap, meta) = setup(&mut ctx);
        let coin_val = coin::mint(&mut cap, 1000, &mut ctx);
        let c = &mut coin_val;
        pay::divide_and_keep(c, 4, &mut ctx);
        // 3 x 250 left; 250 remainder stays (transferred to sender).
        assert!(coin::value(c) == 250, 0);
        transfer::public_transfer(coin_val, @0x1);
        cleanup(cap, meta);
    }

    #[test]
    #[expected_failure(abort_code = 0)]
    fun test_join_vec_and_transfer_empty_fails() {
        // ENoCoins in pay module.
        let v = std::vector::empty<coin::Coin<TEST>>();
        pay::join_vec_and_transfer(v, @0xB);
    }

    #[test]
    #[expected_failure(abort_code = 7)]
    fun test_divide_into_n_over_cap_fails() {
        // ETOO_MANY_PARTS in coin module.
        let ctx = tx_context::dummy();
        let (cap, meta) = setup(&mut ctx);
        let coin_val = coin::mint(&mut cap, 2000, &mut ctx);
        let c = &mut coin_val;
        let parts = coin::divide_into_n(c, 1025, &mut ctx);
        let i = 0;
        while (i < std::vector::length(&parts)) {
            transfer::public_transfer(std::vector::pop_back(&mut parts), @0x1);
            i = i + 1;
        };
        std::vector::destroy_empty(parts);
        transfer::public_transfer(coin_val, @0x1);
        cleanup(cap, meta);
    }
}
