// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[test_only]
module kanari_system::math_tests {
    use kanari_system::math;

    const E_DIVIDE_BY_ZERO: u64 = 2;
    const E_INVALID_ARG: u64 = 3;
    const E_OUT_OF_RANGE: u64 = 4;
    const E_OVERFLOW: u64 = 1;

    const MAX_U64: u64 = 18446744073709551615;
    const MAX_U128: u128 = 340282366920938463463374607431768211455;
    const MAX_U8: u8 = 255;
    const MAX_U16: u16 = 65535;
    const MAX_U32: u32 = 4294967295;
    const MAX_U256: u256 = 115792089237316195423570985008687907853269984665640564039457584007913129639935;

    // =================================================================
    // Min / Max / Average / Clamp
    // =================================================================
    #[test]
    fun test_min_max_u64() {
        assert!(math::min_u64(10, 20) == 10, 0);
        assert!(math::min_u64(20, 10) == 10, 1);
        assert!(math::min_u64(15, 15) == 15, 2);
        assert!(math::max_u64(10, 20) == 20, 3);
        assert!(math::max_u64(20, 10) == 20, 4);
        assert!(math::max_u64(15, 15) == 15, 5);
    }

    #[test]
    fun test_min_max_u128() {
        assert!(math::min_u128(100, 200) == 100, 0);
        assert!(math::min_u128(200, 100) == 100, 1);
        assert!(math::max_u128(100, 200) == 200, 2);
        assert!(math::max_u128(200, 100) == 200, 3);
    }

    #[test]
    fun test_average_no_overflow() {
        // (MAX + MAX) / 2 would overflow naively; native must not.
        assert!(math::average_u64(MAX_U64, MAX_U64) == MAX_U64, 0);
        assert!(math::average_u64(MAX_U64, 0) == MAX_U64 / 2, 1);
        assert!(math::average_u64(10, 20) == 15, 2);
        assert!(math::average_u64(10, 11) == 10, 3); // floor
        assert!(math::average_u128(MAX_U128, MAX_U128) == MAX_U128, 4);
        assert!(math::average_u128(MAX_U128, 0) == MAX_U128 / 2, 5);
        assert!(math::average_u128(10, 20) == 15, 6);
    }

    #[test]
    fun test_clamp_and_bound() {
        assert!(math::clamp_u64(5, 0, 10) == 5, 0);
        assert!(math::clamp_u64(99, 0, 10) == 10, 1);
        assert!(math::clamp_u64(0, 1, 10) == 1, 2);
        assert!(math::clamp_u128(5, 0, 10) == 5, 3);
        assert!(math::clamp_u128(99, 0, 10) == 10, 4);
        assert!(math::bound_u64(99, 0, 10) == 10, 5);
        assert!(math::bound_u128(0, 1, 10) == 1, 6);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_INVALID_ARG)]
    fun test_clamp_u64_inverted_bounds() {
        math::clamp_u64(5, 10, 0);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_INVALID_ARG)]
    fun test_clamp_u128_inverted_bounds() {
        math::clamp_u128(5, 10, 0);
    }

    // =================================================================
    // log2 / pow2 / is_pow2
    // =================================================================
    #[test]
    fun test_log2() {
        assert!(math::log2_u64(1) == 0, 0);
        assert!(math::log2_u64(2) == 1, 1);
        assert!(math::log2_u64(3) == 1, 2);
        assert!(math::log2_u64(MAX_U64) == 63, 3);
        assert!(math::log2_u128(1) == 0, 4);
        assert!(math::log2_u128(MAX_U128) == 127, 5);
        assert!(math::is_pow2_u64(8), 6);
        assert!(!math::is_pow2_u64(10), 7);
        assert!(math::is_pow2_u128(1024), 8);
        assert!(!math::is_pow2_u64(0), 9);
        assert!(math::pow2_u128(10) == 1024, 10);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_INVALID_ARG)]
    fun test_log2_u64_zero() {
        math::log2_u64(0);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_INVALID_ARG)]
    fun test_log2_u128_zero() {
        math::log2_u128(0);
    }

    // =================================================================
    // sqrt / pow / try_pow
    // =================================================================
    #[test]
    fun test_native_sqrt() {
        assert!(math::sqrt_u64(0) == 0, 0);
        assert!(math::sqrt_u64(100) == 10, 1);
        assert!(math::sqrt_u64(144) == 12, 2);
        assert!(math::sqrt_u64(2) == 1, 3);
        assert!(math::sqrt_u128(0) == 0, 4);
        assert!(math::sqrt_u128(1000000000000000000) == 1000000000, 5);
        assert!(math::sqrt_u128(MAX_U128) == 18446744073709551615, 6);
    }

    #[test]
    fun test_native_pow() {
        assert!(math::pow_u64(2, 3) == 8, 0);
        assert!(math::pow_u64(10, 4) == 10000, 1);
        assert!(math::pow_u64(5, 0) == 1, 2);
        assert!(math::pow_u128(2, 10) == 1024, 3);
        assert!(math::pow_u128(10, 18) == 1000000000000000000, 4);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_OVERFLOW)]
    fun test_pow_u64_overflow() {
        math::pow_u64(2, 64);
    }

    #[test]
    fun test_try_pow_does_not_abort() {
        let (ok, v) = math::try_pow_u64(2, 3);
        assert!(ok && v == 8, 0);
        let (ok2, v2) = math::try_pow_u64(2, 64);
        assert!(!ok2 && v2 == 0, 1);
        let (ok3, v3) = math::try_pow_u128(10, 18);
        assert!(ok3 && v3 == 1000000000000000000, 2);
        let (ok4, v4) = math::try_pow_u128(MAX_U128, 2);
        assert!(!ok4 && v4 == 0, 3);
    }

    #[test]
    fun test_diff_and_tolerance() {
        assert!(math::diff_u64(10, 20) == 10, 0);
        assert!(math::diff_u64(20, 10) == 10, 1);
        assert!(math::diff_u128(100, 250) == 150, 2);
        assert!(math::diff_u128(250, 100) == 150, 3);
        assert!(math::within_tolerance_u128(100, 105, 5), 4);
        assert!(!math::within_tolerance_u128(100, 106, 5), 5);
        assert!(math::within_tolerance_u64(10, 12, 2), 6);
    }

    // =================================================================
    // Division with rounding
    // =================================================================
    #[test]
    fun test_ceil_div() {
        assert!(math::ceil_div_u64(0, 5) == 0, 0);
        assert!(math::ceil_div_u64(10, 2) == 5, 1);
        assert!(math::ceil_div_u64(10, 3) == 4, 2);
        assert!(math::ceil_div_u64(1, 100) == 1, 3);
        assert!(math::ceil_div_u128(0, 5) == 0, 4);
        assert!(math::ceil_div_u128(10, 3) == 4, 5);
        // legacy aliases still work
        assert!(math::divide_and_round_up(10, 3) == 4, 6);
        assert!(math::divide_and_round_up_u128(10, 3) == 4, 7);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_DIVIDE_BY_ZERO)]
    fun test_ceil_div_zero_denominator() {
        math::ceil_div_u64(10, 0);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_DIVIDE_BY_ZERO)]
    fun test_ceil_div_u128_zero_denominator() {
        math::ceil_div_u128(10, 0);
    }

    // =================================================================
    // mul_div / mul_add / try_mul_div
    // =================================================================
    #[test]
    fun test_native_mul_div() {
        assert!(math::mul_div_u64(10, 20, 4) == 50, 0);
        assert!(math::mul_div_u128(100, 200, 4) == 5000, 1);
        // No intermediate overflow: (MAX * 2) / 2 == MAX
        assert!(math::mul_div_u64(MAX_U64, 2, 2) == MAX_U64, 2);
        assert!(math::mul_div_u128(MAX_U128, 2, 2) == MAX_U128, 3);
        assert!(math::mul_div_u128(MAX_U128, MAX_U128, MAX_U128) == MAX_U128, 4);
    }

    #[test]
    fun test_mul_div_round() {
        // 10 * 10 / 40 = 2.5
        assert!(math::mul_div_round_u128(10, 10, 40, false) == 2, 0);
        assert!(math::mul_div_round_u128(10, 10, 40, true) == 3, 1);
        assert!(math::mul_div_round_up_u64(10, 10, 40) == 3, 2);
        assert!(math::mul_div_round_up_u128(10, 10, 40) == 3, 3);
        assert!(math::mul_div_ceil_u64(10, 20, 5) == 40, 4);
        assert!(math::mul_div_ceil_u128(10, 20, 5) == 40, 5);
    }

    #[test]
    fun test_try_mul_div() {
        let (ok, v) = math::try_mul_div_u128(10, 20, 5);
        assert!(ok && v == 40, 0);
        let (ok2, _) = math::try_mul_div_u128(10, 20, 0);
        assert!(!ok2, 1);
        let (ok3, _) = math::try_mul_div_u128(MAX_U128, MAX_U128, 1);
        assert!(!ok3, 2);
        let (ok4, v4) = math::try_mul_div_u64(10, 20, 4);
        assert!(ok4 && v4 == 50, 3);
        let (ok5, _) = math::try_mul_div_u64(10, 20, 0);
        assert!(!ok5, 4);
    }

    #[test]
    fun test_mul_add_u128() {
        assert!(math::mul_add_u128(10, 20, 5) == 205, 0);
        assert!(math::mul_add_u128(MAX_U128, 1, 0) == MAX_U128, 1);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_OVERFLOW)]
    fun test_mul_add_u128_overflow() {
        math::mul_add_u128(MAX_U128, 2, 0);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_DIVIDE_BY_ZERO)]
    fun test_mul_div_divide_by_zero() {
        math::mul_div_u128(10, 20, 0);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_OVERFLOW)]
    fun test_mul_div_overflow() {
        math::mul_div_u128(MAX_U128, MAX_U128, 1);
    }

    // =================================================================
    // bps / percent / WAD / RAY
    // =================================================================
    #[test]
    fun test_bps_helpers() {
        // 30 bps = 0.3%
        assert!(math::bps_mul_floor_u128(10000, 30) == 30, 0);
        assert!(math::bps_mul_ceil_u128(10000, 30) == 30, 1);
        // ceil favors receiver: 10 * 1bps / 10_000 = 0.001 -> 1
        assert!(math::bps_mul_floor_u128(10, 1) == 0, 2);
        assert!(math::bps_mul_ceil_u128(10, 1) == 1, 3);
        assert!(math::bps_mul_floor_u64(10000, 30) == 30, 4);
        assert!(math::bps_mul_ceil_u64(10, 1) == 1, 5);
        assert!(math::add_bps_u128(10000, 30) == 10030, 6);
        // sub uses ceil fee: 10000 - ceil(10000*30/10000)=9970
        assert!(math::sub_bps_u128(10000, 30) == 9970, 7);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_OUT_OF_RANGE)]
    fun test_bps_out_of_range() {
        math::bps_mul_floor_u128(100, 10001);
    }

    #[test]
    fun test_percent_helpers() {
        assert!(math::percent_mul_floor_u128(1000, 15) == 150, 0);
        // ceil: 10 * 1% / 100 = 0.1 -> 1
        assert!(math::percent_mul_floor_u128(10, 1) == 0, 1);
        assert!(math::percent_mul_ceil_u128(10, 1) == 1, 2);
    }

    #[test]
    fun test_wad_ray() {
        let wad = 1000000000000000000;
        // 2.0 (wad) * 3.0 (wad) = 6.0 (wad)
        assert!(math::wad_mul(2 * wad, 3 * wad) == 6 * wad, 0);
        // 6.0 / 3.0 = 2.0
        assert!(math::wad_div(6 * wad, 3 * wad) == 2 * wad, 1);
        assert!(math::wad_mul_ceil(2 * wad, 3 * wad) == 6 * wad, 2);
        let ray = 1000000000000000000000000000;
        assert!(math::ray_mul(2 * ray, 3 * ray) == 6 * ray, 3);
        assert!(math::ray_div(6 * ray, 3 * ray) == 2 * ray, 4);
    }

    // =================================================================
    // AMM helpers
    // =================================================================
    #[test]
    fun test_quote_amount_out() {
        // x*y=k: in=100, reserve_in=1000, reserve_out=1000 -> 100*1000/1100 = 90 (floor)
        assert!(math::quote_amount_out(100, 1000, 1000) == 90, 0);
        assert!(math::apply_price(100, 2, 1) == 200, 1);
        math::assert_min_out(100, 90);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_INVALID_ARG)]
    fun test_quote_zero_reserve() {
        math::quote_amount_out(100, 0, 1000);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_INVALID_ARG)]
    fun test_assert_min_out_fails() {
        math::assert_min_out(89, 90);
    }

    // =================================================================
    // Narrow widths: u8 / u16 / u32
    // =================================================================
    #[test]
    fun test_narrow_basics() {
        // min / max / diff
        assert!(math::min_u8(10, 20) == 10, 0);
        assert!(math::max_u8(10, 20) == 20, 1);
        assert!(math::diff_u8(10, 20) == 10, 2);
        assert!(math::min_u16(10, 20) == 10, 3);
        assert!(math::max_u16(10, 20) == 20, 4);
        assert!(math::diff_u16(20, 10) == 10, 5);
        assert!(math::min_u32(10, 20) == 10, 6);
        assert!(math::max_u32(10, 20) == 20, 7);
        assert!(math::diff_u32(20, 10) == 10, 8);
        // average never overflows: (MAX + MAX) / 2
        assert!(math::average_u8(MAX_U8, MAX_U8) == MAX_U8, 9);
        assert!(math::average_u8(MAX_U8, 0) == 127, 10);
        assert!(math::average_u16(MAX_U16, MAX_U16) == MAX_U16, 11);
        assert!(math::average_u32(MAX_U32, MAX_U32) == MAX_U32, 12);
        // clamp / bound
        assert!(math::clamp_u8(99, 0, 10) == 10, 13);
        assert!(math::bound_u16(99, 0, 10) == 10, 14);
        assert!(math::bound_u32(0, 1, 10) == 1, 15);
        // sqrt / log2 / pow2
        assert!(math::sqrt_u8(100) == 10, 16);
        assert!(math::sqrt_u16(10000) == 100, 17);
        assert!(math::sqrt_u32(1000000) == 1000, 18);
        assert!(math::log2_u8(8) == 3, 19);
        assert!(math::log2_u16(1024) == 10, 20);
        assert!(math::log2_u32(MAX_U32) == 31, 21);
        assert!(math::pow2_u8(7) == 128, 22);
        assert!(math::pow2_u16(15) == 32768, 23);
        assert!(math::pow2_u32(31) == 2147483648, 24);
        assert!(math::is_pow2_u8(8), 25);
        assert!(!math::is_pow2_u16(10), 26);
        // ceil_div
        assert!(math::ceil_div_u8(10, 3) == 4, 27);
        assert!(math::ceil_div_u16(10, 3) == 4, 28);
        assert!(math::ceil_div_u32(10, 3) == 4, 29);
    }

    #[test]
    fun test_narrow_pow() {
        assert!(math::pow_u8(2, 7) == 128, 0);
        assert!(math::pow_u16(2, 15) == 32768, 1);
        assert!(math::pow_u32(10, 9) == 1000000000, 2);
        let (ok, v) = math::try_pow_u8(2, 7);
        assert!(ok && v == 128, 3);
        let (ok2, _) = math::try_pow_u8(2, 8);
        assert!(!ok2, 4);
        let (ok3, _) = math::try_pow_u16(2, 16);
        assert!(!ok3, 5);
        let (ok4, _) = math::try_pow_u32(10, 10);
        assert!(!ok4, 6);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_OVERFLOW)]
    fun test_pow_u8_overflow() {
        math::pow_u8(2, 8);
    }

    #[test]
    fun test_narrow_mul_div() {
        assert!(math::mul_div_u8(10, 20, 4) == 50, 0);
        // no intermediate overflow through u128 path
        assert!(math::mul_div_u8(MAX_U8, 2, 2) == MAX_U8, 1);
        assert!(math::mul_div_u16(MAX_U16, 2, 2) == MAX_U16, 2);
        assert!(math::mul_div_u32(MAX_U32, 2, 2) == MAX_U32, 3);
        assert!(math::mul_div_ceil_u8(10, 10, 40) == 3, 4);
        assert!(math::mul_div_ceil_u16(10, 10, 40) == 3, 5);
        assert!(math::mul_div_ceil_u32(10, 10, 40) == 3, 6);
        assert!(math::mul_add_u8(10, 20, 5) == 205, 7);
        assert!(math::mul_add_u16(1000, 60, 5) == 60005, 8);
        assert!(math::mul_add_u32(100000, 20000, 5) == 2000000005, 9);
        let (ok, v) = math::try_mul_div_u8(10, 20, 4);
        assert!(ok && v == 50, 10);
        let (ok2, _) = math::try_mul_div_u16(10, 20, 0);
        assert!(!ok2, 11);
        let (ok3, _) = math::try_mul_div_u32(MAX_U32, MAX_U32, 1);
        assert!(!ok3, 12);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_OVERFLOW)]
    fun test_mul_div_u8_overflow() {
        math::mul_div_u8(MAX_U8, MAX_U8, 1);
    }

    #[test]
    fun test_narrow_bps_percent() {
        assert!(math::bps_mul_floor_u16(10000, 30) == 30, 0);
        assert!(math::bps_mul_ceil_u16(10, 1) == 1, 1);
        assert!(math::bps_mul_floor_u32(10000, 30) == 30, 2);
        // u8: 200 * 255bps / 10000 = 5 (floor)
        assert!(math::bps_mul_floor_u8(200, 255) == 5, 3);
        assert!(math::bps_mul_ceil_u8(10, 1) == 1, 4);
        assert!(math::percent_mul_floor_u8(200, 15) == 30, 5);
        assert!(math::percent_mul_ceil_u16(10, 1) == 1, 6);
        assert!(math::percent_mul_floor_u32(1000, 15) == 150, 7);
        assert!(math::within_tolerance_u8(100, 105, 5), 8);
        assert!(math::within_tolerance_u32(100, 106, 5) == false, 9);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_INVALID_ARG)]
    fun test_clamp_u8_inverted() {
        math::clamp_u8(5, 10, 0);
    }

    // =================================================================
    // u256
    // =================================================================
    #[test]
    fun test_u256_min_max_average_clamp() {
        assert!(math::min_u256(100, 200) == 100, 0);
        assert!(math::max_u256(100, 200) == 200, 1);
        assert!(math::average_u256(MAX_U256, MAX_U256) == MAX_U256, 2);
        assert!(math::average_u256(MAX_U256, 0) == MAX_U256 / 2, 3);
        assert!(math::average_u256(10, 20) == 15, 4);
        assert!(math::clamp_u256(99, 0, 10) == 10, 5);
        assert!(math::bound_u256(0, 1, 10) == 1, 6);
        assert!(math::diff_u256(100, 250) == 150, 7);
        assert!(math::within_tolerance_u256(100, 105, 5), 8);
    }

    #[test]
    fun test_u256_sqrt_pow_log2() {
        assert!(math::sqrt_u256(0) == 0, 0);
        assert!(math::sqrt_u256(1000000000000000000) == 1000000000, 1);
        // sqrt(MAX) == 2^128 - 1
        assert!(math::sqrt_u256(MAX_U256) == 340282366920938463463374607431768211455, 2);
        assert!(math::pow_u256(2, 10) == 1024, 3);
        assert!(math::pow_u256(10, 18) == 1000000000000000000, 4);
        assert!(math::pow2_u256(128) == 340282366920938463463374607431768211456, 5);
        assert!(math::log2_u256(1) == 0, 6);
        assert!(math::log2_u256(MAX_U256) == 255, 7);
        assert!(math::is_pow2_u256(1024), 8);
        let (ok, v) = math::try_pow_u256(2, 10);
        assert!(ok && v == 1024, 9);
        let (ok2, _) = math::try_pow_u256(MAX_U256, 2);
        assert!(!ok2, 10);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_OVERFLOW)]
    fun test_pow_u256_overflow() {
        math::pow_u256(MAX_U256, 2);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_INVALID_ARG)]
    fun test_log2_u256_zero() {
        math::log2_u256(0);
    }

    #[test]
    fun test_u256_mul_div() {
        assert!(math::mul_div_u256(100, 200, 4) == 5000, 0);
        // no intermediate overflow: (MAX * 2) / 2 == MAX, (MAX*MAX)/MAX == MAX
        assert!(math::mul_div_u256(MAX_U256, 2, 2) == MAX_U256, 1);
        assert!(math::mul_div_u256(MAX_U256, MAX_U256, MAX_U256) == MAX_U256, 2);
        // 10 * 10 / 40 = 2.5
        assert!(math::mul_div_round_u256(10, 10, 40, false) == 2, 3);
        assert!(math::mul_div_ceil_u256(10, 10, 40) == 3, 4);
        assert!(math::mul_add_u256(10, 20, 5) == 205, 5);
        let (ok, v) = math::try_mul_div_u256(10, 20, 5);
        assert!(ok && v == 40, 6);
        let (ok2, _) = math::try_mul_div_u256(10, 20, 0);
        assert!(!ok2, 7);
        let (ok3, _) = math::try_mul_div_u256(MAX_U256, MAX_U256, 1);
        assert!(!ok3, 8);
        assert!(math::ceil_div_u256(10, 3) == 4, 9);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_DIVIDE_BY_ZERO)]
    fun test_mul_div_u256_zero() {
        math::mul_div_u256(10, 20, 0);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_OVERFLOW)]
    fun test_mul_div_u256_overflow() {
        math::mul_div_u256(MAX_U256, MAX_U256, 1);
    }

    #[test]
    fun test_u256_bps_wad_amm() {
        assert!(math::bps_mul_floor_u256(10000, 30) == 30, 0);
        assert!(math::bps_mul_ceil_u256(10, 1) == 1, 1);
        assert!(math::add_bps_u256(10000, 30) == 10030, 2);
        assert!(math::sub_bps_u256(10000, 30) == 9970, 3);
        assert!(math::percent_mul_floor_u256(1000, 15) == 150, 4);
        assert!(math::percent_mul_ceil_u256(10, 1) == 1, 5);
        let wad = 1000000000000000000;
        assert!(math::wad_mul_u256(2 * wad, 3 * wad) == 6 * wad, 6);
        assert!(math::wad_div_u256(6 * wad, 3 * wad) == 2 * wad, 7);
        let ray = 1000000000000000000000000000;
        assert!(math::ray_mul_u256(2 * ray, 3 * ray) == 6 * ray, 8);
        assert!(math::ray_div_u256(6 * ray, 3 * ray) == 2 * ray, 9);
        assert!(math::quote_amount_out_u256(100, 1000, 1000) == 90, 10);
        assert!(math::apply_price_u256(100, 2, 1) == 200, 11);
        math::assert_min_out_u256(100, 90);
    }

    #[test]
    fun test_u64_extras() {
        assert!(math::mul_add_u64(10, 20, 5) == 205, 0);
        assert!(math::mul_add_u64(MAX_U64, 1, 0) == MAX_U64, 1);
        assert!(math::pow2_u64(10) == 1024, 2);
    }
}
