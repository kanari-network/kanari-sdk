// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0
#[test_only]
module kanari_system::math_tests {
    use kanari_system::math;
    use std::option;

    const E_DIVIDE_BY_ZERO: u64 = 2;
    const E_INVALID_ARG: u64 = 3;
    const E_OUT_OF_RANGE: u64 = 4;
    const E_OVERFLOW: u64 = 1;

    const MAX_U64: u64 = 18446744073709551615;
    const MAX_U128: u128 = 340282366920938463463374607431768211455;
    const MAX_U8: u8 = 255;
    const MAX_U16: u16 = 65535;
    const MAX_U32: u32 = 4294967295;
    const MAX_U256: u256 =
        115792089237316195423570985008687907853269984665640564039457584007913129639935;

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
        assert!(
            math::average_u128(MAX_U128, 0) == MAX_U128 / 2, 5
        );
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
        // boundary values never overflow
        assert!(math::ceil_div_u64(MAX_U64, MAX_U64) == 1, 8);
        assert!(
            math::ceil_div_u64(MAX_U64, 2) == (MAX_U64 / 2) + 1, 9
        );
        assert!(math::ceil_div_u128(MAX_U128, MAX_U128) == 1, 10);
        assert!(
            math::ceil_div_u128(MAX_U128, 2) == (MAX_U128 / 2) + 1, 11
        );
        assert!(math::ceil_div_u256(MAX_U256, MAX_U256) == 1, 12);
        assert!(
            math::ceil_div_u256(MAX_U256, 2) == (MAX_U256 / 2) + 1, 13
        );
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
        assert!(
            math::mul_div_u128(MAX_U128, 2, 2) == MAX_U128,
            3
        );
        assert!(
            math::mul_div_u128(MAX_U128, MAX_U128, MAX_U128) == MAX_U128,
            4
        );
    }

    #[test]
    fun test_mul_div_round() {
        // 10 * 10 / 40 = 2.5
        assert!(
            math::mul_div_round_u128(10, 10, 40, false) == 2,
            0
        );
        assert!(
            math::mul_div_round_u128(10, 10, 40, true) == 3,
            1
        );
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
        assert!(
            math::mul_add_u128(MAX_U128, 1, 0) == MAX_U128,
            1
        );
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
        assert!(
            math::mul_add_u32(100000, 20000, 5) == 2000000005,
            9
        );
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
        assert!(
            math::within_tolerance_u32(100, 106, 5) == false,
            9
        );
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
        assert!(
            math::average_u256(MAX_U256, 0) == MAX_U256 / 2, 3
        );
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
        assert!(
            math::mul_div_u256(MAX_U256, 2, 2) == MAX_U256,
            1
        );
        assert!(
            math::mul_div_u256(MAX_U256, MAX_U256, MAX_U256) == MAX_U256,
            2
        );
        // 10 * 10 / 40 = 2.5
        assert!(
            math::mul_div_round_u256(10, 10, 40, false) == 2,
            3
        );
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
        assert!(
            math::quote_amount_out_u256(100, 1000, 1000) == 90,
            10
        );
        assert!(math::apply_price_u256(100, 2, 1) == 200, 11);
        math::assert_min_out_u256(100, 90);
    }

    #[test]
    fun test_u64_extras() {
        assert!(math::mul_add_u64(10, 20, 5) == 205, 0);
        assert!(math::mul_add_u64(MAX_U64, 1, 0) == MAX_U64, 1);
        assert!(math::pow2_u64(10) == 1024, 2);
    }

    // =================================================================
    // Checked / saturating / lossless / narrowing
    // =================================================================
    #[test]
    fun test_checked_arithmetic() {
        // u8 boundaries
        assert!(math::checked_add_u8(200, 55) == option::some(255u8), 0);
        assert!(
            math::checked_add_u8(200, 56) == option::none<u8>(),
            1
        );
        assert!(
            math::checked_sub_u8(5, 10) == option::none<u8>(),
            2
        );
        assert!(
            math::checked_mul_u8(16, 16) == option::none<u8>(),
            3
        );
        assert!(math::checked_mul_u8(0, MAX_U8) == option::some(0u8), 4);
        assert!(
            math::checked_div_u8(10, 0) == option::none<u8>(),
            5
        );
        assert!(math::checked_pow_u8(2, 7) == option::some(128u8), 6);
        assert!(
            math::checked_pow_u8(2, 8) == option::none<u8>(),
            7
        );
        // u64 / u128 / u256 spot checks
        assert!(
            math::checked_add_u64(MAX_U64, 1) == option::none<u64>(),
            8
        );
        assert!(
            math::checked_mul_u64(MAX_U64, 2) == option::none<u64>(),
            9
        );
        assert!(
            math::checked_div_u64(10, 0) == option::none<u64>(),
            10
        );
        assert!(
            math::checked_sub_u128(5, 10) == option::none<u128>(),
            11
        );
        assert!(
            math::checked_pow_u128(10, 39) == option::none<u128>(),
            12
        );
        assert!(
            math::checked_add_u256(MAX_U256, 1) == option::none<u256>(),
            13
        );
        assert!(
            math::checked_mul_u256(MAX_U256, MAX_U256) == option::none<u256>(),
            14
        );
        assert!(
            math::checked_pow_u256(2, 255) == option::some(math::pow2_u256(255)),
            15
        );
        assert!(
            math::checked_pow_u256(2, 256) == option::none<u256>(),
            16
        );
        // u16 / u32 spot checks
        assert!(
            math::checked_add_u16(MAX_U16, 1) == option::none<u16>(),
            17
        );
        assert!(
            math::checked_mul_u32(MAX_U32, 2) == option::none<u32>(),
            18
        );
    }

    #[test]
    fun test_saturating_arithmetic() {
        assert!(math::saturating_add_u8(200, 100) == 255, 0);
        assert!(math::saturating_sub_u8(5, 10) == 0, 1);
        assert!(math::saturating_mul_u8(16, 16) == 255, 2);
        assert!(math::saturating_pow_u8(2, 8) == 255, 3);
        assert!(
            math::saturating_mul_div_u8(MAX_U8, MAX_U8, 1) == 255,
            4
        );
        assert!(math::saturating_add_u64(MAX_U64, 1) == MAX_U64, 5);
        assert!(math::saturating_mul_u128(MAX_U128, 2) == MAX_U128, 6);
        assert!(math::saturating_pow_u128(10, 39) == MAX_U128, 7);
        assert!(
            math::saturating_mul_div_u128(MAX_U128, MAX_U128, 1) == MAX_U128,
            8
        );
        assert!(math::saturating_add_u256(MAX_U256, 1) == MAX_U256, 9);
        assert!(math::saturating_pow_u256(2, 300) == MAX_U256, 10);
        assert!(math::saturating_sub_u32(5, 10) == 0, 11);
        assert!(math::saturating_mul_u16(MAX_U16, 2) == MAX_U16, 12);
    }

    #[test]
    #[expected_failure(location = kanari_system::math, abort_code = E_DIVIDE_BY_ZERO)]
    fun test_saturating_mul_div_zero() {
        math::saturating_mul_div_u64(10, 20, 0);
    }

    #[test]
    fun test_shifts_and_lossless() {
        assert!(math::checked_shl_u8(1, 7) == option::some(128u8), 0);
        assert!(
            math::checked_shl_u8(1, 8) == option::none<u8>(),
            1
        );
        assert!(
            math::lossless_shl_u8(64, 2) == option::none<u8>(),
            2
        ); // 256, bits lost
        assert!(math::lossless_shl_u8(32, 2) == option::some(128u8), 3);
        assert!(math::lossless_shr_u8(128, 2) == option::some(32u8), 4);
        assert!(
            math::lossless_shr_u8(101, 2) == option::none<u8>(),
            5
        ); // 25*4=100 != 101
        assert!(math::lossless_div_u64(100, 4) == option::some(25u64), 6);
        assert!(
            math::lossless_div_u64(100, 3) == option::none<u64>(),
            7
        );
        assert!(
            math::lossless_div_u64(100, 0) == option::none<u64>(),
            8
        );
        assert!(
            math::checked_shl_u64(1, 64) == option::none<u64>(),
            9
        );
        assert!(
            math::lossless_shl_u128(1, 127) == option::some(math::pow2_u128(127)),
            10
        );
        assert!(
            math::lossless_shl_u256(1, 255) == option::some(math::pow2_u256(255)),
            11
        );
        assert!(math::bitwise_not_u8(0) == 255, 12);
        assert!(math::bitwise_not_u64(0) == MAX_U64, 13);
        assert!(math::bitwise_not_u256(0) == MAX_U256, 14);
    }

    #[test]
    fun test_try_as_narrowing() {
        assert!(math::try_as_u8_from_u16(255) == option::some(255u8), 0);
        assert!(
            math::try_as_u8_from_u16(256) == option::none<u8>(),
            1
        );
        assert!(
            math::try_as_u32_from_u64((MAX_U32 as u64)) == option::some(MAX_U32),
            2
        );
        assert!(
            math::try_as_u32_from_u64((MAX_U32 as u64) + 1) == option::none<u32>(),
            3
        );
        assert!(
            math::try_as_u64_from_u128((MAX_U64 as u128)) == option::some(MAX_U64),
            4
        );
        assert!(
            math::try_as_u128_from_u256((MAX_U128 as u256)) == option::some(MAX_U128),
            5
        );
        assert!(
            math::try_as_u128_from_u256((MAX_U128 as u256) + 1) == option::none<u128>(),
            6
        );
        assert!(math::try_as_u8_from_u256(200) == option::some(200u8), 7);
    }

    // =================================================================
    // Property tests (xorshift64 PRNG, fixed seed, bounded loops)
    // =================================================================

    /// xorshift64*: shift/xor only, so it can never abort on overflow.
    fun prng_next(s: &mut u64): u64 {
        let x = *s;
        if (x == 0) {
            x = 0x9E3779B97F4A7C15;
        };
        x = x ^ (x << 13);
        x = x ^ (x >> 7);
        x = x ^ (x << 17);
        *s = x;
        x
    }

    fun prng_u128(s: &mut u64): u128 {
        let hi = prng_next(s);
        let lo = prng_next(s);
        ((hi as u128) << 64) | (lo as u128)
    }

    // Small operands whose product fits u128: exact-division roundtrips.
    #[test]
    fun prop_sqrt_square_back_u128() {
        let s = 0x12345678;
        let i = 0;
        while (i < 100) {
            let x = prng_u128(&mut s);
            let r = math::sqrt_u128(x);
            assert!(r * r <= x, i);
            // (r+1)^2 overflows only when r == 2^64 - 1; guard it.
            if (r < 18446744073709551615) {
                assert!((r + 1) * (r + 1) > x, i);
            };
            i = i + 1;
        };
    }

    #[test]
    fun prop_sqrt_u256_agrees_u128() {
        let s = 0xABCDEF01;
        let i = 0;
        while (i < 30) {
            // 64-bit samples: square-back also fits, cross-checks both impls.
            let x = (prng_next(&mut s) as u256);
            assert!(
                math::sqrt_u256(x) == (math::sqrt_u128((x as u128)) as u256),
                i
            );
            i = i + 1;
        };
    }

    #[test]
    fun prop_mul_div_exact_roundtrip() {
        let s = 0x0F1E2D3C;
        let i = 0;
        while (i < 100) {
            // Keep a, b < 2^32 so a * b fits u64: division is then exact.
            let a = prng_next(&mut s) % 4294967296;
            let b = prng_next(&mut s) % 4294967296;
            let d = (prng_next(&mut s) % 1000000) + 1;
            let q = math::mul_div_u64(a, b, d);
            // q <= a*b/d < q+1  <=>  q*d <= a*b < (q+1)*d  (in u128: no overflow)
            let prod = (a as u128) * (b as u128);
            assert!((q as u128) * (d as u128) <= prod, i);
            assert!(prod < ((q as u128) + 1) * (d as u128), i);
            // ceil is either q or q + 1
            let c = math::mul_div_ceil_u64(a, b, d);
            assert!(c == q || c == q + 1, i);
            i = i + 1;
        };
    }

    #[test]
    fun prop_checked_saturating_agree() {
        let s = 0x55AA55AA;
        let i = 0;
        while (i < 100) {
            let a = prng_u128(&mut s);
            let b = prng_u128(&mut s);
            // saturating_add == MAX  <=>  checked_add is None
            let sat = math::saturating_add_u128(a, b);
            let chk = math::checked_add_u128(a, b);
            if (option::is_some(&chk)) {
                assert!(sat == *option::borrow(&chk), i);
                option::destroy_some(chk);
            } else {
                assert!(sat == MAX_U128, i);
                option::destroy_none(chk);
            };
            // average always inside [min, max]
            let avg = math::average_u128(a, b);
            let lo = math::min_u128(a, b);
            let hi = math::max_u128(a, b);
            assert!(avg >= lo && avg <= hi, i);
            // diff + min reconstructs max
            assert!(lo + math::diff_u128(a, b) == hi, i);
            i = i + 1;
        };
    }

    #[test]
    fun prop_pow_mul_consistent() {
        // pow(b, e+1) == pow(b, e) * b whenever the smaller side fits.
        let b = 7u64;
        let e = 0u8;
        let acc = 1u64;
        while (e < 10) {
            assert!(math::pow_u64(b, e) == acc, (e as u64));
            acc = acc * b;
            e = e + 1;
        };
    }
}

