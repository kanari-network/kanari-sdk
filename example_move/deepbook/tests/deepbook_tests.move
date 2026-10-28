// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[allow(unused_let_mut)]
#[test_only]
module deepbook::deepbook_tests {
    use deepbook::deepbook::{
        calculate_base_amount, calculate_base_amount_with_decimals, calculate_quote_amount,
        calculate_quote_amount_with_decimals, check_refund_overflow, create_order_book_with_decimals,
        new_order_book, place_bid, place_ask, cancel_order,
        destroy_order_book_for_testing,
        get_best_bid, get_best_ask, get_spread,
        get_bid_count, get_ask_count, get_book_depth,
        get_fee_balances, get_locked_balances,
        validate_quote_capacity,
    };
    use deepbook::registry::compare_vectors;
    use kanari_system::balance::destroy_for_testing;
    use kanari_system::coin::{Coin, mint, split, into_balance};
    use kanari_system::kanari::KANARI;
    use kanari_system::math;
    use kanari_system::transfer::public_freeze_object;
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::usd::USD;

    const PRICE_SCALE: u64 = 1_000_000_000;

    // ---------- Test token types ----------
    public struct TEST_BASE has drop {}

    public struct TEST_QUOTE has drop {}

    const BASE_DECIMALS: u8 = 9;
    const QUOTE_DECIMALS: u8 = 6;

    // ---------- Helpers ----------
    /// Splits `amount` from `coin`, returns the split coin (coin still has remainder).
    fun split_off<T>(coin: &mut Coin<T>, amount: u64, ctx: &mut TxContext): Coin<T> {
        split(coin, amount, ctx)
    }

    /// Creates TEST_BASE and TEST_QUOTE currencies and mints initial coins.
    fun setup_and_mint(ctx: &mut TxContext): (Coin<TEST_BASE>, Coin<TEST_QUOTE>) {
        let (mut base_cap, base_meta) =
            kanari_system::coin::create_currency<TEST_BASE>(TEST_BASE {}, BASE_DECIMALS, b"BASE", b"Test Base", b"", option::none(), ctx);
        let (mut quote_cap, quote_meta) =
            kanari_system::coin::create_currency<TEST_QUOTE>(TEST_QUOTE {}, QUOTE_DECIMALS, b"QUOTE", b"Test Quote", b"", option::none(), ctx);

        let base_coin = mint<TEST_BASE>(&mut base_cap, 10_000_000_000, ctx); // 10 BASE
        let quote_coin = mint<TEST_QUOTE>(&mut quote_cap, 10_000_000, ctx); // 10 QUOTE

        public_freeze_object(base_cap);
        public_freeze_object(base_meta);
        public_freeze_object(quote_cap);
        public_freeze_object(quote_meta);

        (base_coin, quote_coin)
    }

    /// Consume remaining coins at end of test.
    fun consume_coins(base_coin: Coin<TEST_BASE>, quote_coin: Coin<TEST_QUOTE>) {
        let _b = into_balance(base_coin);
        let _q = into_balance(quote_coin);
        destroy_for_testing(_b);
        destroy_for_testing(_q);
    }

    // ---------- compare_vectors ----------
    #[test]
    fun compare_vectors_orders_lexicographically() {
        assert!(compare_vectors(&b"abc", &b"abd"), 0);
        assert!(!compare_vectors(&b"abd", &b"abc"), 1);
        assert!(compare_vectors(&b"ab", &b"abc"), 2);
        assert!(compare_vectors(&b"abc", &b"abc"), 3);
    }

    #[test]
    fun compare_vectors_is_stable_for_pair_sorting() {
        let x = b"0x2::kanari::KANARI";
        let y = b"0x2::usd::USD";
        assert!(compare_vectors(&x, &y) != compare_vectors(&y, &x), 4);
    }

    #[test]
    fun compare_vectors_empty_and_equal() {
        assert!(compare_vectors(&b"", &b""), 5);
        assert!(compare_vectors(&b"", &b"a"), 6);
        assert!(!compare_vectors(&b"a", &b""), 7);
        assert!(compare_vectors(&b"aaaa", &b"aaaa"), 8);
    }

    // ---------- pow10 ----------
    #[test]
    fun pow10_matches_decimals() {
        assert!(math::pow_u128(10, 0) == 1, 10);
        assert!(math::pow_u128(10, 1) == 10, 11);
        assert!(math::pow_u128(10, 9) == 1_000_000_000, 12);
    }

    // ---------- price math ----------
    #[test]
    fun quote_and_base_round_trip_at_par() {
        let par = 1_000_000_000;
        assert!(calculate_quote_amount(1_000, par) == 1_000, 20);
        assert!(calculate_base_amount(1_000, par) == 1_000, 21);
    }

    #[test]
    fun quote_math_scales_with_price() {
        let price = 2_500_000_000;
        assert!(calculate_quote_amount(100, price) == 250, 22);
        assert!(calculate_base_amount(250, price) == 100, 23);
    }

    #[test]
    fun quote_math_zero_quantity() {
        assert!(calculate_quote_amount(0, PRICE_SCALE) == 0, 26);
        assert!(calculate_base_amount(0, PRICE_SCALE) == 0, 27);
    }

    #[test]
    #[expected_failure(abort_code = 2, location = kanari_system::math)]
    fun quote_math_zero_price_rejects() {
        calculate_base_amount(100, 0);
    }

    #[test]
    fun decimals_aware_math_normalizes_mismatched_decimals() {
        let quote = calculate_quote_amount_with_decimals(1_000_000_000, 1_000_000_000, 9, 6);
        assert!(quote == 1_000_000, 24);
        let base = calculate_base_amount_with_decimals(1_000_000, 1_000_000_000, 9, 6);
        assert!(base == 1_000_000_000, 25);
    }

    #[test]
    fun decimals_aware_math_quote_more_decimals_than_base() {
        let quote = calculate_quote_amount_with_decimals(1_000_000, 1_000_000_000, 6, 9);
        assert!(quote == 1_000_000_000, 30);
        let base = calculate_base_amount_with_decimals(1_000_000_000, 1_000_000_000, 6, 9);
        assert!(base == 1_000_000, 31);
    }

    #[test]
    fun decimals_aware_math_high_price() {
        let quote = calculate_quote_amount_with_decimals(1_000_000_000, 100_000_000_000, 9, 6);
        assert!(quote == 100_000_000, 32);
        let base = calculate_base_amount_with_decimals(100_000_000, 100_000_000_000, 9, 6);
        assert!(base == 1_000_000_000, 33);
    }

    // ---------- validate_quote_capacity ----------
    #[test]
    fun validate_quote_capacity_accepts_sane_orders() {
        validate_quote_capacity(1_000_000_000, 1_000_000, 9, 9);
    }

    #[test]
    #[expected_failure(abort_code = 1, location = deepbook::deepbook)]
    fun validate_quote_capacity_rejects_dust_orders() {
        validate_quote_capacity(1, 1, 9, 9);
    }

    #[test]
    #[expected_failure(abort_code = 1, location = deepbook::deepbook)]
    fun validate_quote_capacity_rejects_zero_price() {
        validate_quote_capacity(0, 1_000_000, 9, 9);
    }

    #[test]
    #[expected_failure(abort_code = 1, location = deepbook::deepbook)]
    fun validate_quote_capacity_rejects_zero_quantity() {
        validate_quote_capacity(1_000_000_000, 0, 9, 9);
    }

    #[test]
    fun validate_quote_capacity_large_order() {
        validate_quote_capacity(1_000_000_000, 100_000_000_000, 9, 9);
    }

    // ---------- check_refund_overflow ----------
    #[test]
    fun check_refund_overflow_passes_for_bounded_values() {
        check_refund_overflow(1_000_000, 1_000_000_000, 9, 9);
    }

    #[test]
    #[expected_failure(abort_code = 10, location = deepbook::deepbook)]
    fun check_refund_overflow_rejects_u64_blowout() {
        check_refund_overflow(18446744073709551615, 18446744073709551615, 0, 0);
    }

    #[test]
    fun check_refund_overflow_zero_unmatched() {
        check_refund_overflow(0, 1_000_000_000, 9, 9);
    }

    #[test]
    fun check_refund_overflow_zero_price() {
        check_refund_overflow(1_000_000, 0, 9, 9);
    }

    // ---------- Order book creation ----------
    #[test]
    fun create_order_book_returns_nonzero_address() {
        let mut ctx = tx_context::dummy();
        let addr = create_order_book_with_decimals<KANARI, USD>(30, 1000, 9, 6, &mut ctx);
        assert!(addr != @0x0, 40);
    }

    #[test]
    fun create_order_book_validates_fee_bps() {
        let mut ctx = tx_context::dummy();
        let addr = create_order_book_with_decimals<KANARI, USD>(50, 1000, 9, 6, &mut ctx);
        assert!(addr != @0x0, 41);
    }

    #[test]
    fun create_order_book_validates_depth() {
        let mut ctx = tx_context::dummy();
        let addr = create_order_book_with_decimals<KANARI, USD>(30, 1, 9, 6, &mut ctx);
        assert!(addr != @0x0, 42);
    }

    #[test]
    fun create_order_book_different_decimals() {
        let mut ctx = tx_context::dummy();
        let addr = create_order_book_with_decimals<KANARI, USD>(30, 1000, 9, 6, &mut ctx);
        assert!(addr != @0x0, 43);
        let addr2 = create_order_book_with_decimals<USD, KANARI>(30, 1000, 6, 9, &mut ctx);
        assert!(addr2 != @0x0, 44);
    }

    // ---------- Order types ----------
    #[test]
    fun test_limit_bid_and_ask_full_fill() {
        let mut ctx = tx_context::dummy();
        let (mut base_coin, mut quote_coin) = setup_and_mint(&mut ctx);

        let mut book = new_order_book<TEST_BASE, TEST_QUOTE>(30, 1000, BASE_DECIMALS, QUOTE_DECIMALS, &mut ctx);

        let base_to_sell = split_off(&mut base_coin, 1_000_000_000, &mut ctx);
        place_ask<TEST_BASE, TEST_QUOTE>(
            &mut book, 1_000_000_000, 1_000_000_000, base_to_sell, 0, &mut ctx
        );

        assert!(get_best_ask(&book) == 1_000_000_000, 50);
        assert!(get_ask_count(&book) == 1, 51);

        let quote_to_pay = split_off(&mut quote_coin, 1_000_000, &mut ctx);
        place_bid<TEST_BASE, TEST_QUOTE>(
            &mut book, 1_000_000_000, 1_000_000_000, quote_to_pay, 0, &mut ctx
        );

        assert!(get_best_bid(&book) == 0, 52);
        assert!(get_best_ask(&book) == 0, 53);
        assert!(get_bid_count(&book) == 0, 54);
        assert!(get_ask_count(&book) == 0, 55);

        destroy_order_book_for_testing(book);
        consume_coins(base_coin, quote_coin);
    }

    #[test]
    fun test_ioc_partial_fill_cancels_unfilled() {
        let mut ctx = tx_context::dummy();
        let (mut base_coin, mut quote_coin) = setup_and_mint(&mut ctx);

        let mut book = new_order_book<TEST_BASE, TEST_QUOTE>(30, 1000, BASE_DECIMALS, QUOTE_DECIMALS, &mut ctx);

        let base_to_sell = split_off(&mut base_coin, 1_000_000_000, &mut ctx);
        place_ask<TEST_BASE, TEST_QUOTE>(
            &mut book, 1_000_000_000, 1_000_000_000, base_to_sell, 0, &mut ctx
        );

        let quote_to_pay = split_off(&mut quote_coin, 2_000_000, &mut ctx);
        place_bid<TEST_BASE, TEST_QUOTE>(
            &mut book, 1_000_000_000, 2_000_000_000, quote_to_pay, 1, &mut ctx
        );

        assert!(get_best_ask(&book) == 0, 60);
        assert!(get_ask_count(&book) == 0, 61);
        assert!(get_bid_count(&book) == 0, 62);

        destroy_order_book_for_testing(book);
        consume_coins(base_coin, quote_coin);
    }

    #[test]
    fun test_fok_full_fill_succeeds() {
        let mut ctx = tx_context::dummy();
        let (mut base_coin, mut quote_coin) = setup_and_mint(&mut ctx);

        let mut book = new_order_book<TEST_BASE, TEST_QUOTE>(30, 1000, BASE_DECIMALS, QUOTE_DECIMALS, &mut ctx);

        let base_to_sell = split_off(&mut base_coin, 1_000_000_000, &mut ctx);
        place_ask<TEST_BASE, TEST_QUOTE>(
            &mut book, 1_000_000_000, 1_000_000_000, base_to_sell, 0, &mut ctx
        );

        let quote_to_pay = split_off(&mut quote_coin, 1_000_000, &mut ctx);
        place_bid<TEST_BASE, TEST_QUOTE>(
            &mut book, 1_000_000_000, 1_000_000_000, quote_to_pay, 2, &mut ctx
        );

        assert!(get_best_ask(&book) == 0, 70);
        assert!(get_ask_count(&book) == 0, 71);

        destroy_order_book_for_testing(book);
        consume_coins(base_coin, quote_coin);
    }

    #[test]
    #[expected_failure(abort_code = 1, location = deepbook::deepbook)]
    fun test_fok_partial_fill_aborts() {
        let mut ctx = tx_context::dummy();
        let (mut base_coin, mut quote_coin) = setup_and_mint(&mut ctx);

        let mut book = new_order_book<TEST_BASE, TEST_QUOTE>(30, 1000, BASE_DECIMALS, QUOTE_DECIMALS, &mut ctx);

        let base_to_sell = split_off(&mut base_coin, 1_000_000_000, &mut ctx);
        place_ask<TEST_BASE, TEST_QUOTE>(
            &mut book, 1_000_000_000, 1_000_000_000, base_to_sell, 0, &mut ctx
        );

        let quote_to_pay = split_off(&mut quote_coin, 2_000_000, &mut ctx);
        place_bid<TEST_BASE, TEST_QUOTE>(
            &mut book, 1_000_000_000, 2_000_000_000, quote_to_pay, 2, &mut ctx
        );

        destroy_order_book_for_testing(book);
        consume_coins(base_coin, quote_coin);
    }

    #[test]
    fun test_post_only_resting_order_when_no_cross() {
        let mut ctx = tx_context::dummy();
        let (mut base_coin, mut quote_coin) = setup_and_mint(&mut ctx);

        let mut book = new_order_book<TEST_BASE, TEST_QUOTE>(30, 1000, BASE_DECIMALS, QUOTE_DECIMALS, &mut ctx);

        let base_to_sell = split_off(&mut base_coin, 1_000_000_000, &mut ctx);
        place_ask<TEST_BASE, TEST_QUOTE>(
            &mut book, 2_000_000_000, 1_000_000_000, base_to_sell, 0, &mut ctx
        );

        let quote_to_pay = split_off(&mut quote_coin, 1_000_000, &mut ctx);
        place_bid<TEST_BASE, TEST_QUOTE>(
            &mut book, 1_000_000_000, 1_000_000_000, quote_to_pay, 3, &mut ctx
        );

        assert!(get_best_bid(&book) == 1_000_000_000, 80);
        assert!(get_bid_count(&book) == 1, 81);

        destroy_order_book_for_testing(book);
        consume_coins(base_coin, quote_coin);
    }

    #[test]
    #[expected_failure(abort_code = 11, location = deepbook::deepbook)]
    fun test_post_only_crossing_aborts() {
        let mut ctx = tx_context::dummy();
        let (mut base_coin, mut quote_coin) = setup_and_mint(&mut ctx);

        let mut book = new_order_book<TEST_BASE, TEST_QUOTE>(30, 1000, BASE_DECIMALS, QUOTE_DECIMALS, &mut ctx);

        let base_to_sell = split_off(&mut base_coin, 1_000_000_000, &mut ctx);
        place_ask<TEST_BASE, TEST_QUOTE>(
            &mut book, 1_000_000_000, 1_000_000_000, base_to_sell, 0, &mut ctx
        );

        let quote_to_pay = split_off(&mut quote_coin, 2_000_000, &mut ctx);
        place_bid<TEST_BASE, TEST_QUOTE>(
            &mut book, 2_000_000_000, 1_000_000_000, quote_to_pay, 3, &mut ctx
        );

        destroy_order_book_for_testing(book);
        consume_coins(base_coin, quote_coin);
    }

    #[test]
    fun test_cancel_order_refunds_locked_funds() {
        let mut ctx = tx_context::dummy();
        let (_base_coin, mut quote_coin) = setup_and_mint(&mut ctx);

        let mut book = new_order_book<TEST_BASE, TEST_QUOTE>(30, 1000, BASE_DECIMALS, QUOTE_DECIMALS, &mut ctx);

        let quote_to_pay = split_off(&mut quote_coin, 1_000_000, &mut ctx);
        place_bid<TEST_BASE, TEST_QUOTE>(
            &mut book, 1_000_000_000, 1_000_000_000, quote_to_pay, 0, &mut ctx
        );

        assert!(get_bid_count(&book) == 1, 90);
        let (_locked_base, locked_quote) = get_locked_balances(&book);
        assert!(locked_quote == 1_000_000, 91);

        cancel_order<TEST_BASE, TEST_QUOTE>(&mut book, 1, &mut ctx);

        assert!(get_bid_count(&book) == 0, 92);

        destroy_order_book_for_testing(book);
        consume_coins(_base_coin, quote_coin);
    }

    #[test]
    fun test_query_functions_on_empty_book() {
        let mut ctx = tx_context::dummy();
        let (mut base_coin, mut quote_coin) = setup_and_mint(&mut ctx);

        let book = new_order_book<TEST_BASE, TEST_QUOTE>(30, 1000, BASE_DECIMALS, QUOTE_DECIMALS, &mut ctx);

        assert!(get_best_bid(&book) == 0, 100);
        assert!(get_best_ask(&book) == 0, 101);
        assert!(get_spread(&book) == 0, 102);
        assert!(get_bid_count(&book) == 0, 103);
        assert!(get_ask_count(&book) == 0, 104);
        let (depth_bid, depth_ask) = get_book_depth(&book);
        assert!(depth_bid == 0, 105);
        assert!(depth_ask == 0, 106);
        let (fee_base, fee_quote) = get_fee_balances(&book);
        assert!(fee_base == 0, 107);
        assert!(fee_quote == 0, 108);
        let (locked_base, locked_quote) = get_locked_balances(&book);
        assert!(locked_base == 0, 109);
        assert!(locked_quote == 0, 110);

        destroy_order_book_for_testing(book);
        consume_coins(base_coin, quote_coin);
    }

    // ---------- Depth eviction ----------
    #[test]
    fun test_bid_depth_eviction() {
        let mut ctx = tx_context::dummy();
        let (mut base_coin, mut quote_coin) = setup_and_mint(&mut ctx);

        let mut book = new_order_book<TEST_BASE, TEST_QUOTE>(30, 3, BASE_DECIMALS, QUOTE_DECIMALS, &mut ctx);

        // Bid 1: price 100, qty 1 (order_id=1)
        let q1 = split_off(&mut quote_coin, 1_000_000, &mut ctx);
        place_bid<TEST_BASE, TEST_QUOTE>(
            &mut book, 100_000_000, 1_000_000_000, q1, 0, &mut ctx
        );

        // Bid 2: price 90, qty 1 (order_id=2)
        let q2 = split_off(&mut quote_coin, 900_000, &mut ctx);
        place_bid<TEST_BASE, TEST_QUOTE>(
            &mut book, 90_000_000, 1_000_000_000, q2, 0, &mut ctx
        );

        // Bid 3: price 80, qty 1 (order_id=3)
        let q3 = split_off(&mut quote_coin, 800_000, &mut ctx);
        place_bid<TEST_BASE, TEST_QUOTE>(
            &mut book, 80_000_000, 1_000_000_000, q3, 0, &mut ctx
        );

        assert!(get_bid_count(&book) == 3, 120);
        assert!(get_best_bid(&book) == 100_000_000, 121);

        // Bid 4: price 95, qty 1 (order_id=4) -> evicts bid 3 (price 80, worst)
        let q4 = split_off(&mut quote_coin, 950_000, &mut ctx);
        place_bid<TEST_BASE, TEST_QUOTE>(
            &mut book, 95_000_000, 1_000_000_000, q4, 0, &mut ctx
        );

        assert!(get_bid_count(&book) == 3, 122);
        assert!(get_best_bid(&book) == 100_000_000, 123);

        destroy_order_book_for_testing(book);
        consume_coins(base_coin, quote_coin);
    }

    #[test]
    fun test_ask_depth_eviction() {
        let mut ctx = tx_context::dummy();
        let (mut base_coin, mut quote_coin) = setup_and_mint(&mut ctx);

        let mut book = new_order_book<TEST_BASE, TEST_QUOTE>(30, 3, BASE_DECIMALS, QUOTE_DECIMALS, &mut ctx);

        // Ask 1: price 100, qty 1 (order_id=1)
        let b1 = split_off(&mut base_coin, 1_000_000_000, &mut ctx);
        place_ask<TEST_BASE, TEST_QUOTE>(
            &mut book, 100_000_000, 1_000_000_000, b1, 0, &mut ctx
        );

        // Ask 2: price 110, qty 1 (order_id=2)
        let b2 = split_off(&mut base_coin, 1_000_000_000, &mut ctx);
        place_ask<TEST_BASE, TEST_QUOTE>(
            &mut book, 110_000_000, 1_000_000_000, b2, 0, &mut ctx
        );

        // Ask 3: price 120, qty 1 (order_id=3)
        let b3 = split_off(&mut base_coin, 1_000_000_000, &mut ctx);
        place_ask<TEST_BASE, TEST_QUOTE>(
            &mut book, 120_000_000, 1_000_000_000, b3, 0, &mut ctx
        );

        assert!(get_ask_count(&book) == 3, 130);
        assert!(get_best_ask(&book) == 100_000_000, 131);

        // Ask 4: price 105, qty 1 (order_id=4) -> evicts ask 3 (price 120, worst)
        let b4 = split_off(&mut base_coin, 1_000_000_000, &mut ctx);
        place_ask<TEST_BASE, TEST_QUOTE>(
            &mut book, 105_000_000, 1_000_000_000, b4, 0, &mut ctx
        );

        assert!(get_ask_count(&book) == 3, 132);
        assert!(get_best_ask(&book) == 100_000_000, 133);

        destroy_order_book_for_testing(book);
        consume_coins(base_coin, quote_coin);
    }
}
