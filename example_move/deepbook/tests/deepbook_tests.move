// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[test_only]
module deepbook::deepbook_tests {
    use deepbook::deepbook::{
        calculate_base_amount, calculate_base_amount_with_decimals, calculate_quote_amount,
        calculate_quote_amount_with_decimals, check_refund_overflow, compare_vectors,
        validate_quote_capacity,
    };
    use kanari_system::math;

    // ---------- compare_vectors (registry pair-hash ordering) ----------
    #[test]
    fun compare_vectors_orders_lexicographically() {
        assert!(compare_vectors(&b"abc", &b"abd"), 0);
        assert!(!compare_vectors(&b"abd", &b"abc"), 1);
        // equal prefixes: shorter is "less"
        assert!(compare_vectors(&b"ab", &b"abc"), 2);
        assert!(compare_vectors(&b"abc", &b"abc"), 3);
    }

    #[test]
    fun compare_vectors_is_stable_for_pair_sorting() {
        // Sorting must be total: exactly one direction holds for unequal inputs.
        let x = b"0x2::kanari::KANARI";
        let y = b"0x2::usd::USD";
        assert!(compare_vectors(&x, &y) != compare_vectors(&y, &x), 4);
    }

    // ---------- pow10 (framework math) ----------
    #[test]
    fun pow10_matches_decimals() {
        assert!(math::pow_u128(10, 0) == 1, 10);
        assert!(math::pow_u128(10, 1) == 10, 11);
        assert!(math::pow_u128(10, 9) == 1_000_000_000, 12);
    }

    // ---------- price math ----------
    #[test]
    fun quote_and_base_round_trip_at_par() {
        // price = 1.0 in PRICE_SCALE units
        let par = 1_000_000_000;
        assert!(calculate_quote_amount(1_000, par) == 1_000, 20);
        assert!(calculate_base_amount(1_000, par) == 1_000, 21);
    }

    #[test]
    fun quote_math_scales_with_price() {
        // price 2.5x
        let price = 2_500_000_000;
        assert!(calculate_quote_amount(100, price) == 250, 22);
        assert!(calculate_base_amount(250, price) == 100, 23);
    }

    #[test]
    fun decimals_aware_math_normalizes_mismatched_decimals() {
        // base 9 decimals, quote 6 decimals, price 1.0:
        // 1_000_000_000 base units (1.0 base) -> 1_000_000 quote units (1.0 quote)
        let quote = calculate_quote_amount_with_decimals(1_000_000_000, 1_000_000_000, 9, 6);
        assert!(quote == 1_000_000, 24);
        let base = calculate_base_amount_with_decimals(1_000_000, 1_000_000_000, 9, 6);
        assert!(base == 1_000_000_000, 25);
    }

    #[test]
    fun validate_quote_capacity_accepts_sane_orders() {
        validate_quote_capacity(1_000_000_000, 1_000_000, 9, 9);
    }

    #[test]
    #[expected_failure(abort_code = 1, location = deepbook::deepbook)] // E_INSUFFICIENT_LIQUIDITY: rounds to zero
    fun validate_quote_capacity_rejects_dust_orders() {
        // quantity 1 at price 1 with 9/9 decimals rounds required quote to 0.
        validate_quote_capacity(1, 1, 9, 9);
    }

    #[test]
    fun check_refund_overflow_passes_for_bounded_values() {
        check_refund_overflow(1_000_000, 1_000_000_000, 9, 9);
    }

    #[test]
    #[expected_failure(abort_code = 10, location = deepbook::deepbook)] // E_OVERFLOW
    fun check_refund_overflow_rejects_u64_blowout() {
        // unmatched * price far above u64 max after scaling.
        check_refund_overflow(18446744073709551615, 18446744073709551615, 0, 0);
    }
}
