// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Production math library for DeFi on Kanari.
///
/// Covers all six Move integer widths (`u8` / `u16` / `u32` / `u64` / `u128` / `u256`).
///
/// Design rules:
/// - Hot / overflow-sensitive paths are natives (U256/U512 intermediate, single gas charge).
/// - Narrow widths (`u8`/`u16`/`u32`) are pure-Move wrappers over the `u64`/`u128`
///   natives: zero new Rust code, explicit `E_OVERFLOW` on narrowing.
/// - Everything else is pure Move built on top of those natives, so contracts get
///   checked fixed-point / bps / WAD / RAY helpers without reimplementing them.
/// - Aborting variants (`mul_div_*`, `pow_*`) abort with the codes below.
/// - Non-aborting variants (`try_*`) return `(success, value)` so contracts can
///   handle bad input gracefully (Move has no try/catch).
module kanari_system::math {

    // =================================================================
    // Error Codes (must match math.rs)
    // =================================================================
    const E_OVERFLOW: u64 = 1;
    const E_DIVIDE_BY_ZERO: u64 = 2;
    const E_INVALID_ARG: u64 = 3;
    const E_OUT_OF_RANGE: u64 = 4;

    // =================================================================
    // Constants
    // =================================================================
    const MAX_U8: u8 = 255;
    const MAX_U16: u16 = 65535;
    const MAX_U32: u32 = 4294967295;
    const MAX_U64: u64 = 18446744073709551615;
    const MAX_U128: u128 = 340282366920938463463374607431768211455;
    const MAX_U256: u256 = 115792089237316195423570985008687907853269984665640564039457584007913129639935;

    /// Width limits, exposed so contracts can clamp / validate without hardcoding.
    public fun max_u8_value(): u8 { MAX_U8 }
    public fun max_u16_value(): u16 { MAX_U16 }
    public fun max_u32_value(): u32 { MAX_U32 }
    public fun max_u64_value(): u64 { MAX_U64 }
    public fun max_u128_value(): u128 { MAX_U128 }
    public fun max_u256_value(): u256 { MAX_U256 }

    /// Basis points scale: 10_000 bps = 100%.
    const BPS_SCALE: u128 = 10000;
    const BPS_SCALE_U64: u64 = 10000;
    const BPS_SCALE_U256: u256 = 10000;
    /// Percent scale: 100 = 100%.
    const PCT_SCALE: u128 = 100;
    const PCT_SCALE_U256: u256 = 100;
    /// WAD fixed point: 1e18 (e.g. ERC20-style 18 decimals).
    const WAD: u128 = 1000000000000000000;
    const WAD_U256: u256 = 1000000000000000000;
    /// RAY fixed point: 1e27 (e.g. Maker-style rates).
    const RAY: u128 = 1000000000000000000000000000;
    const RAY_U256: u256 = 1000000000000000000000000000;

    // =================================================================
    // Native Functions (implemented in Rust)
    // =================================================================

    /// Floor sqrt of a u128.
    public native fun sqrt_u128(x: u128): u128;
    /// Floor sqrt of a u64.
    public native fun sqrt_u64(x: u64): u64;
    /// Floor sqrt of a u256.
    public native fun sqrt_u256(x: u256): u256;

    /// base ^ exponent. Aborts with E_OVERFLOW on overflow.
    public native fun pow_u64(base: u64, exponent: u8): u64;
    /// base ^ exponent. Aborts with E_OVERFLOW on overflow.
    public native fun pow_u128(base: u128, exponent: u32): u128;
    /// base ^ exponent. Aborts with E_OVERFLOW on overflow.
    public native fun pow_u256(base: u256, exponent: u32): u256;

    /// Non-aborting pow. Returns (success, value); (false, 0) on overflow.
    public native fun try_pow_u64(base: u64, exponent: u8): (bool, u64);
    /// Non-aborting pow. Returns (success, value); (false, 0) on overflow.
    public native fun try_pow_u128(base: u128, exponent: u32): (bool, u128);
    /// Non-aborting pow. Returns (success, value); (false, 0) on overflow.
    public native fun try_pow_u256(base: u256, exponent: u32): (bool, u256);

    /// (x * y) / z with U256 intermediate, floor rounding.
    /// Aborts E_DIVIDE_BY_ZERO when z == 0, E_OVERFLOW when result > MAX_U128.
    public native fun mul_div_u128(x: u128, y: u128, z: u128): u128;

    /// (x * y) / z with explicit rounding: false = floor, true = ceil.
    public native fun mul_div_round_u128(x: u128, y: u128, z: u128, round_up: bool): u128;

    /// Non-aborting (x * y) / z (floor). (false, 0) on div-by-zero or overflow.
    public native fun try_mul_div_u128(x: u128, y: u128, z: u128): (bool, u128);

    /// (x * y) + z with U256 intermediate. Aborts E_OVERFLOW when result > MAX_U128.
    public native fun mul_add_u128(x: u128, y: u128, z: u128): u128;

    /// (x * y) / z with U512 intermediate, floor rounding.
    /// Aborts E_DIVIDE_BY_ZERO when z == 0, E_OVERFLOW when result > MAX_U256.
    public native fun mul_div_u256(x: u256, y: u256, z: u256): u256;

    /// (x * y) / z with explicit rounding: false = floor, true = ceil.
    public native fun mul_div_round_u256(x: u256, y: u256, z: u256, round_up: bool): u256;

    /// Non-aborting (x * y) / z (floor). (false, 0) on div-by-zero or overflow.
    public native fun try_mul_div_u256(x: u256, y: u256, z: u256): (bool, u256);

    /// (x * y) + z with U512 intermediate. Aborts E_OVERFLOW when result > MAX_U256.
    public native fun mul_add_u256(x: u256, y: u256, z: u256): u256;

    public native fun min_u128(x: u128, y: u128): u128;
    public native fun max_u128(x: u128, y: u128): u128;
    public native fun min_u256(x: u256, y: u256): u256;
    public native fun max_u256(x: u256, y: u256): u256;

    /// Overflow-safe average, floor: (a & b) + ((a ^ b) >> 1).
    public native fun average_u64(a: u64, b: u64): u64;
    /// Overflow-safe average, floor.
    public native fun average_u128(a: u128, b: u128): u128;
    /// Overflow-safe average, floor.
    public native fun average_u256(a: u256, b: u256): u256;

    /// Clamp x into [lo, hi]. Aborts E_INVALID_ARG when lo > hi.
    public native fun clamp_u64(x: u64, lo: u64, hi: u64): u64;
    /// Clamp x into [lo, hi]. Aborts E_INVALID_ARG when lo > hi.
    public native fun clamp_u128(x: u128, lo: u128, hi: u128): u128;
    /// Clamp x into [lo, hi]. Aborts E_INVALID_ARG when lo > hi.
    public native fun clamp_u256(x: u256, lo: u256, hi: u256): u256;

    /// Floor log2. Aborts E_INVALID_ARG on 0.
    public native fun log2_u64(x: u64): u64;
    /// Floor log2. Aborts E_INVALID_ARG on 0.
    public native fun log2_u128(x: u128): u128;
    /// Floor log2. Aborts E_INVALID_ARG on 0.
    public native fun log2_u256(x: u256): u256;

    // =================================================================
    // u8 helpers (pure Move over u64/u128 natives)
    // =================================================================

    public fun min_u8(x: u8, y: u8): u8 {
        if (x < y) { x } else { y }
    }

    public fun max_u8(x: u8, y: u8): u8 {
        if (x > y) { x } else { y }
    }

    public fun diff_u8(x: u8, y: u8): u8 {
        if (x > y) { x - y } else { y - x }
    }

    /// Overflow-safe average, floor. Result always fits u8.
    public fun average_u8(a: u8, b: u8): u8 {
        ((average_u64((a as u64), (b as u64))) as u8)
    }

    public fun clamp_u8(x: u8, lo: u8, hi: u8): u8 {
        assert!(lo <= hi, E_INVALID_ARG);
        if (x < lo) { lo } else if (x > hi) { hi } else { x }
    }

    public fun bound_u8(x: u8, lo: u8, hi: u8): u8 {
        clamp_u8(x, lo, hi)
    }

    public fun sqrt_u8(x: u8): u8 {
        ((sqrt_u64((x as u64))) as u8)
    }

    public fun log2_u8(x: u8): u8 {
        ((log2_u64((x as u64))) as u8)
    }

    public fun is_pow2_u8(x: u8): bool {
        x > 0 && (x & (x - 1)) == 0
    }

    public fun pow2_u8(exp: u8): u8 {
        pow_u8(2, exp)
    }

    public fun pow_u8(base: u8, exponent: u8): u8 {
        let r = pow_u64((base as u64), exponent);
        assert!(r <= (MAX_U8 as u64), E_OVERFLOW);
        (r as u8)
    }

    public fun try_pow_u8(base: u8, exponent: u8): (bool, u8) {
        let (ok, r) = try_pow_u64((base as u64), exponent);
        if (!ok || r > (MAX_U8 as u64)) { (false, 0) } else { (true, (r as u8)) }
    }

    public fun ceil_div_u8(x: u8, y: u8): u8 {
        assert!(y > 0, E_DIVIDE_BY_ZERO);
        if (x == 0) { 0 } else { ((x - 1) / y) + 1 }
    }

    public fun mul_div_u8(x: u8, y: u8, z: u8): u8 {
        let r = mul_div_u128((x as u128), (y as u128), (z as u128));
        assert!(r <= (MAX_U8 as u128), E_OVERFLOW);
        (r as u8)
    }

    public fun mul_div_ceil_u8(x: u8, y: u8, z: u8): u8 {
        let r = mul_div_round_u128((x as u128), (y as u128), (z as u128), true);
        assert!(r <= (MAX_U8 as u128), E_OVERFLOW);
        (r as u8)
    }

    public fun try_mul_div_u8(x: u8, y: u8, z: u8): (bool, u8) {
        let (ok, r) = try_mul_div_u128((x as u128), (y as u128), (z as u128));
        if (!ok || r > (MAX_U8 as u128)) { (false, 0) } else { (true, (r as u8)) }
    }

    public fun mul_add_u8(x: u8, y: u8, z: u8): u8 {
        let r = mul_add_u128((x as u128), (y as u128), (z as u128));
        assert!(r <= (MAX_U8 as u128), E_OVERFLOW);
        (r as u8)
    }

    public fun bps_mul_floor_u8(amount: u8, bps: u8): u8 {
        // 10_000 does not fit u8, so scale through u128 explicitly.
        assert!((bps as u128) <= BPS_SCALE, E_OUT_OF_RANGE);
        let r = mul_div_u128((amount as u128), (bps as u128), BPS_SCALE);
        assert!(r <= (MAX_U8 as u128), E_OVERFLOW);
        (r as u8)
    }

    public fun bps_mul_ceil_u8(amount: u8, bps: u8): u8 {
        // NOTE: u8 caps bps at 255; amounts stay exact via u128 intermediate.
        // 10_000 does not fit u8, so scale through u128 explicitly.
        assert!((bps as u128) <= BPS_SCALE, E_OUT_OF_RANGE);
        let r = mul_div_round_u128((amount as u128), (bps as u128), BPS_SCALE, true);
        assert!(r <= (MAX_U8 as u128), E_OVERFLOW);
        (r as u8)
    }

    public fun percent_mul_floor_u8(amount: u8, pct: u8): u8 {
        assert!((pct as u128) <= PCT_SCALE, E_OUT_OF_RANGE);
        mul_div_u8(amount, pct, 100)
    }

    public fun percent_mul_ceil_u8(amount: u8, pct: u8): u8 {
        assert!((pct as u128) <= PCT_SCALE, E_OUT_OF_RANGE);
        mul_div_ceil_u8(amount, pct, 100)
    }

    public fun within_tolerance_u8(a: u8, b: u8, tolerance: u8): bool {
        diff_u8(a, b) <= tolerance
    }

    // =================================================================
    // u16 helpers (pure Move over u64/u128 natives)
    // =================================================================

    public fun min_u16(x: u16, y: u16): u16 {
        if (x < y) { x } else { y }
    }

    public fun max_u16(x: u16, y: u16): u16 {
        if (x > y) { x } else { y }
    }

    public fun diff_u16(x: u16, y: u16): u16 {
        if (x > y) { x - y } else { y - x }
    }

    public fun average_u16(a: u16, b: u16): u16 {
        ((average_u64((a as u64), (b as u64))) as u16)
    }

    public fun clamp_u16(x: u16, lo: u16, hi: u16): u16 {
        assert!(lo <= hi, E_INVALID_ARG);
        if (x < lo) { lo } else if (x > hi) { hi } else { x }
    }

    public fun bound_u16(x: u16, lo: u16, hi: u16): u16 {
        clamp_u16(x, lo, hi)
    }

    public fun sqrt_u16(x: u16): u16 {
        ((sqrt_u64((x as u64))) as u16)
    }

    public fun log2_u16(x: u16): u16 {
        ((log2_u64((x as u64))) as u16)
    }

    public fun is_pow2_u16(x: u16): bool {
        x > 0 && (x & (x - 1)) == 0
    }

    public fun pow2_u16(exp: u8): u16 {
        pow_u16(2, exp)
    }

    public fun pow_u16(base: u16, exponent: u8): u16 {
        let r = pow_u64((base as u64), exponent);
        assert!(r <= (MAX_U16 as u64), E_OVERFLOW);
        (r as u16)
    }

    public fun try_pow_u16(base: u16, exponent: u8): (bool, u16) {
        let (ok, r) = try_pow_u64((base as u64), exponent);
        if (!ok || r > (MAX_U16 as u64)) { (false, 0) } else { (true, (r as u16)) }
    }

    public fun ceil_div_u16(x: u16, y: u16): u16 {
        assert!(y > 0, E_DIVIDE_BY_ZERO);
        if (x == 0) { 0 } else { ((x - 1) / y) + 1 }
    }

    public fun mul_div_u16(x: u16, y: u16, z: u16): u16 {
        let r = mul_div_u128((x as u128), (y as u128), (z as u128));
        assert!(r <= (MAX_U16 as u128), E_OVERFLOW);
        (r as u16)
    }

    public fun mul_div_ceil_u16(x: u16, y: u16, z: u16): u16 {
        let r = mul_div_round_u128((x as u128), (y as u128), (z as u128), true);
        assert!(r <= (MAX_U16 as u128), E_OVERFLOW);
        (r as u16)
    }

    public fun try_mul_div_u16(x: u16, y: u16, z: u16): (bool, u16) {
        let (ok, r) = try_mul_div_u128((x as u128), (y as u128), (z as u128));
        if (!ok || r > (MAX_U16 as u128)) { (false, 0) } else { (true, (r as u16)) }
    }

    public fun mul_add_u16(x: u16, y: u16, z: u16): u16 {
        let r = mul_add_u128((x as u128), (y as u128), (z as u128));
        assert!(r <= (MAX_U16 as u128), E_OVERFLOW);
        (r as u16)
    }

    public fun bps_mul_floor_u16(amount: u16, bps: u16): u16 {
        assert!((bps as u128) <= BPS_SCALE, E_OUT_OF_RANGE);
        let r = mul_div_u128((amount as u128), (bps as u128), BPS_SCALE);
        assert!(r <= (MAX_U16 as u128), E_OVERFLOW);
        (r as u16)
    }

    public fun bps_mul_ceil_u16(amount: u16, bps: u16): u16 {
        assert!((bps as u128) <= BPS_SCALE, E_OUT_OF_RANGE);
        let r = mul_div_round_u128((amount as u128), (bps as u128), BPS_SCALE, true);
        assert!(r <= (MAX_U16 as u128), E_OVERFLOW);
        (r as u16)
    }

    public fun percent_mul_floor_u16(amount: u16, pct: u16): u16 {
        assert!((pct as u128) <= PCT_SCALE, E_OUT_OF_RANGE);
        mul_div_u16(amount, pct, 100)
    }

    public fun percent_mul_ceil_u16(amount: u16, pct: u16): u16 {
        assert!((pct as u128) <= PCT_SCALE, E_OUT_OF_RANGE);
        mul_div_ceil_u16(amount, pct, 100)
    }

    public fun within_tolerance_u16(a: u16, b: u16, tolerance: u16): bool {
        diff_u16(a, b) <= tolerance
    }

    // =================================================================
    // u32 helpers (pure Move over u64/u128 natives)
    // =================================================================

    public fun min_u32(x: u32, y: u32): u32 {
        if (x < y) { x } else { y }
    }

    public fun max_u32(x: u32, y: u32): u32 {
        if (x > y) { x } else { y }
    }

    public fun diff_u32(x: u32, y: u32): u32 {
        if (x > y) { x - y } else { y - x }
    }

    public fun average_u32(a: u32, b: u32): u32 {
        ((average_u64((a as u64), (b as u64))) as u32)
    }

    public fun clamp_u32(x: u32, lo: u32, hi: u32): u32 {
        assert!(lo <= hi, E_INVALID_ARG);
        if (x < lo) { lo } else if (x > hi) { hi } else { x }
    }

    public fun bound_u32(x: u32, lo: u32, hi: u32): u32 {
        clamp_u32(x, lo, hi)
    }

    public fun sqrt_u32(x: u32): u32 {
        ((sqrt_u64((x as u64))) as u32)
    }

    public fun log2_u32(x: u32): u32 {
        ((log2_u64((x as u64))) as u32)
    }

    public fun is_pow2_u32(x: u32): bool {
        x > 0 && (x & (x - 1)) == 0
    }

    public fun pow2_u32(exp: u8): u32 {
        pow_u32(2, exp)
    }

    public fun pow_u32(base: u32, exponent: u8): u32 {
        let r = pow_u64((base as u64), exponent);
        assert!(r <= (MAX_U32 as u64), E_OVERFLOW);
        (r as u32)
    }

    public fun try_pow_u32(base: u32, exponent: u8): (bool, u32) {
        let (ok, r) = try_pow_u64((base as u64), exponent);
        if (!ok || r > (MAX_U32 as u64)) { (false, 0) } else { (true, (r as u32)) }
    }

    public fun ceil_div_u32(x: u32, y: u32): u32 {
        assert!(y > 0, E_DIVIDE_BY_ZERO);
        if (x == 0) { 0 } else { ((x - 1) / y) + 1 }
    }

    public fun mul_div_u32(x: u32, y: u32, z: u32): u32 {
        let r = mul_div_u128((x as u128), (y as u128), (z as u128));
        assert!(r <= (MAX_U32 as u128), E_OVERFLOW);
        (r as u32)
    }

    public fun mul_div_ceil_u32(x: u32, y: u32, z: u32): u32 {
        let r = mul_div_round_u128((x as u128), (y as u128), (z as u128), true);
        assert!(r <= (MAX_U32 as u128), E_OVERFLOW);
        (r as u32)
    }

    public fun try_mul_div_u32(x: u32, y: u32, z: u32): (bool, u32) {
        let (ok, r) = try_mul_div_u128((x as u128), (y as u128), (z as u128));
        if (!ok || r > (MAX_U32 as u128)) { (false, 0) } else { (true, (r as u32)) }
    }

    public fun mul_add_u32(x: u32, y: u32, z: u32): u32 {
        let r = mul_add_u128((x as u128), (y as u128), (z as u128));
        assert!(r <= (MAX_U32 as u128), E_OVERFLOW);
        (r as u32)
    }

    public fun bps_mul_floor_u32(amount: u32, bps: u32): u32 {
        assert!((bps as u128) <= BPS_SCALE, E_OUT_OF_RANGE);
        let r = mul_div_u128((amount as u128), (bps as u128), BPS_SCALE);
        assert!(r <= (MAX_U32 as u128), E_OVERFLOW);
        (r as u32)
    }

    public fun bps_mul_ceil_u32(amount: u32, bps: u32): u32 {
        assert!((bps as u128) <= BPS_SCALE, E_OUT_OF_RANGE);
        let r = mul_div_round_u128((amount as u128), (bps as u128), BPS_SCALE, true);
        assert!(r <= (MAX_U32 as u128), E_OVERFLOW);
        (r as u32)
    }

    public fun percent_mul_floor_u32(amount: u32, pct: u32): u32 {
        assert!((pct as u128) <= PCT_SCALE, E_OUT_OF_RANGE);
        mul_div_u32(amount, pct, 100)
    }

    public fun percent_mul_ceil_u32(amount: u32, pct: u32): u32 {
        assert!((pct as u128) <= PCT_SCALE, E_OUT_OF_RANGE);
        mul_div_ceil_u32(amount, pct, 100)
    }

    public fun within_tolerance_u32(a: u32, b: u32, tolerance: u32): bool {
        diff_u32(a, b) <= tolerance
    }

    // =================================================================
    // u64 / u128 basics
    // =================================================================

    public fun min_u64(x: u64, y: u64): u64 {
        if (x < y) { x } else { y }
    }

    public fun max_u64(x: u64, y: u64): u64 {
        if (x > y) { x } else { y }
    }

    /// Absolute difference. Useful for slippage / tolerance checks.
    public fun diff_u64(x: u64, y: u64): u64 {
        if (x > y) { x - y } else { y - x }
    }

    public fun diff_u128(x: u128, y: u128): u128 {
        if (x > y) { x - y } else { y - x }
    }

    public fun diff_u256(x: u256, y: u256): u256 {
        if (x > y) { x - y } else { y - x }
    }

    /// Clamp with a readable abort code for inverted bounds (same as native).
    public fun bound_u64(x: u64, lo: u64, hi: u64): u64 {
        assert!(lo <= hi, E_INVALID_ARG);
        clamp_u64(x, lo, hi)
    }

    public fun bound_u128(x: u128, lo: u128, hi: u128): u128 {
        assert!(lo <= hi, E_INVALID_ARG);
        clamp_u128(x, lo, hi)
    }

    public fun bound_u256(x: u256, lo: u256, hi: u256): u256 {
        assert!(lo <= hi, E_INVALID_ARG);
        clamp_u256(x, lo, hi)
    }

    public fun is_pow2_u64(x: u64): bool {
        x > 0 && (x & (x - 1)) == 0
    }

    public fun is_pow2_u128(x: u128): bool {
        x > 0 && (x & (x - 1)) == 0
    }

    public fun is_pow2_u256(x: u256): bool {
        x > 0 && (x & (x - 1)) == 0
    }

    /// 2^exp, aborting on overflow. Prefer over `pow_u128(2, exp)` for readability.
    public fun pow2_u128(exp: u32): u128 {
        pow_u128(2, exp)
    }

    public fun pow2_u64(exp: u8): u64 {
        pow_u64(2, exp)
    }

    public fun pow2_u256(exp: u32): u256 {
        pow_u256(2, exp)
    }

    // =================================================================
    // Division with rounding
    // =================================================================

    /// Ceil division: (x + y - 1) / y. Aborts E_DIVIDE_BY_ZERO on y == 0.
    public fun ceil_div_u64(x: u64, y: u64): u64 {
        assert!(y > 0, E_DIVIDE_BY_ZERO);
        if (x == 0) { 0 } else { ((x - 1) / y) + 1 }
    }

    public fun ceil_div_u128(x: u128, y: u128): u128 {
        assert!(y > 0, E_DIVIDE_BY_ZERO);
        if (x == 0) { 0 } else { ((x - 1) / y) + 1 }
    }

    public fun ceil_div_u256(x: u256, y: u256): u256 {
        assert!(y > 0, E_DIVIDE_BY_ZERO);
        if (x == 0) { 0 } else { ((x - 1) / y) + 1 }
    }

    /// Legacy names kept for backwards compatibility.
    public fun divide_and_round_up(x: u64, y: u64): u64 { ceil_div_u64(x, y) }
    public fun divide_and_round_up_u128(x: u128, y: u128): u128 { ceil_div_u128(x, y) }

    // =================================================================
    // mul_div / mul_add with explicit overflow checks
    // =================================================================

    /// (x * y) / z for u64, floor. Routes through U256 native so
    /// `MAX_U64 * 2 / 2` works. Aborts E_OVERFLOW when result > MAX_U64.
    public fun mul_div_u64(x: u64, y: u64, z: u64): u64 {
        let r = mul_div_u128((x as u128), (y as u128), (z as u128));
        assert!(r <= (MAX_U64 as u128), E_OVERFLOW);
        (r as u64)
    }

    /// (x * y) / z for u64, ceil.
    public fun mul_div_ceil_u64(x: u64, y: u64, z: u64): u64 {
        let r = mul_div_round_u128((x as u128), (y as u128), (z as u128), true);
        assert!(r <= (MAX_U64 as u128), E_OVERFLOW);
        (r as u64)
    }

    /// (x * y) / z for u128, ceil.
    public fun mul_div_ceil_u128(x: u128, y: u128, z: u128): u128 {
        mul_div_round_u128(x, y, z, true)
    }

    /// (x * y) / z for u256, ceil.
    public fun mul_div_ceil_u256(x: u256, y: u256, z: u256): u256 {
        mul_div_round_u256(x, y, z, true)
    }

    /// (x * y) + z for u64. Aborts E_OVERFLOW when result > MAX_U64.
    public fun mul_add_u64(x: u64, y: u64, z: u64): u64 {
        let r = mul_add_u128((x as u128), (y as u128), (z as u128));
        assert!(r <= (MAX_U64 as u128), E_OVERFLOW);
        (r as u64)
    }

    /// Legacy ceil helpers kept for backwards compatibility.
    public fun mul_div_round_up_u64(x: u64, y: u64, z: u64): u64 {
        mul_div_ceil_u64(x, y, z)
    }

    public fun mul_div_round_up_u128(x: u128, y: u128, z: u128): u128 {
        mul_div_ceil_u128(x, y, z)
    }

    public fun mul_div_round_up_u256(x: u256, y: u256, z: u256): u256 {
        mul_div_ceil_u256(x, y, z)
    }

    /// Non-aborting u64 variant: (false, 0) on div-by-zero or u64 overflow.
    public fun try_mul_div_u64(x: u64, y: u64, z: u64): (bool, u64) {
        let (ok, r) = try_mul_div_u128((x as u128), (y as u128), (z as u128));
        if (!ok || r > (MAX_U64 as u128)) { (false, 0) } else { (true, (r as u64)) }
    }

    // =================================================================
    // Percent / basis-point helpers (fee, slippage, interest)
    // =================================================================

    /// amount * bps / 10_000, floor. `bps` must be <= 10_000.
    public fun bps_mul_floor_u128(amount: u128, bps: u128): u128 {
        assert!(bps <= BPS_SCALE, E_OUT_OF_RANGE);
        mul_div_u128(amount, bps, BPS_SCALE)
    }

    /// amount * bps / 10_000, ceil (favors the fee receiver / pool).
    public fun bps_mul_ceil_u128(amount: u128, bps: u128): u128 {
        assert!(bps <= BPS_SCALE, E_OUT_OF_RANGE);
        mul_div_ceil_u128(amount, bps, BPS_SCALE)
    }

    public fun bps_mul_floor_u64(amount: u64, bps: u64): u64 {
        assert!((bps as u128) <= BPS_SCALE, E_OUT_OF_RANGE);
        mul_div_u64(amount, bps, BPS_SCALE_U64)
    }

    public fun bps_mul_ceil_u64(amount: u64, bps: u64): u64 {
        assert!((bps as u128) <= BPS_SCALE, E_OUT_OF_RANGE);
        mul_div_ceil_u64(amount, bps, BPS_SCALE_U64)
    }

    public fun bps_mul_floor_u256(amount: u256, bps: u256): u256 {
        assert!(bps <= BPS_SCALE_U256, E_OUT_OF_RANGE);
        mul_div_u256(amount, bps, BPS_SCALE_U256)
    }

    public fun bps_mul_ceil_u256(amount: u256, bps: u256): u256 {
        assert!(bps <= BPS_SCALE_U256, E_OUT_OF_RANGE);
        mul_div_ceil_u256(amount, bps, BPS_SCALE_U256)
    }

    /// amount + fee where fee = amount * bps / 10_000 (floor).
    public fun add_bps_u128(amount: u128, bps: u128): u128 {
        amount + bps_mul_floor_u128(amount, bps)
    }

    /// amount - fee where fee = amount * bps / 10_000 (ceil, favors pool).
    /// Aborts E_OVERFLOW if fee > amount (cannot happen for bps <= 10_000,
    /// but kept as a safety net).
    public fun sub_bps_u128(amount: u128, bps: u128): u128 {
        let fee = bps_mul_ceil_u128(amount, bps);
        assert!(fee <= amount, E_OVERFLOW);
        amount - fee
    }

    public fun add_bps_u256(amount: u256, bps: u256): u256 {
        amount + bps_mul_floor_u256(amount, bps)
    }

    public fun sub_bps_u256(amount: u256, bps: u256): u256 {
        let fee = bps_mul_ceil_u256(amount, bps);
        assert!(fee <= amount, E_OVERFLOW);
        amount - fee
    }

    /// amount * pct / 100, floor. `pct` must be <= 100.
    public fun percent_mul_floor_u128(amount: u128, pct: u128): u128 {
        assert!(pct <= PCT_SCALE, E_OUT_OF_RANGE);
        mul_div_u128(amount, pct, PCT_SCALE)
    }

    /// amount * pct / 100, ceil.
    public fun percent_mul_ceil_u128(amount: u128, pct: u128): u128 {
        assert!(pct <= PCT_SCALE, E_OUT_OF_RANGE);
        mul_div_ceil_u128(amount, pct, PCT_SCALE)
    }

    public fun percent_mul_floor_u256(amount: u256, pct: u256): u256 {
        assert!(pct <= PCT_SCALE_U256, E_OUT_OF_RANGE);
        mul_div_u256(amount, pct, PCT_SCALE_U256)
    }

    public fun percent_mul_ceil_u256(amount: u256, pct: u256): u256 {
        assert!(pct <= PCT_SCALE_U256, E_OUT_OF_RANGE);
        mul_div_ceil_u256(amount, pct, PCT_SCALE_U256)
    }

    // =================================================================
    // Fixed-point helpers (WAD 1e18 / RAY 1e27)
    // =================================================================

    /// wad_mul(a, b) = a * b / 1e18, floor. Inputs and output are WAD-scaled.
    public fun wad_mul(a: u128, b: u128): u128 {
        mul_div_u128(a, b, WAD)
    }

    /// wad_div(a, b) = a * 1e18 / b, floor.
    public fun wad_div(a: u128, b: u128): u128 {
        mul_div_u128(a, WAD, b)
    }

    /// wad_mul with ceil rounding (favors pool / lender).
    public fun wad_mul_ceil(a: u128, b: u128): u128 {
        mul_div_ceil_u128(a, b, WAD)
    }

    /// wad_div with ceil rounding.
    public fun wad_div_ceil(a: u128, b: u128): u128 {
        mul_div_ceil_u128(a, WAD, b)
    }

    /// ray_mul(a, b) = a * b / 1e27, floor.
    public fun ray_mul(a: u128, b: u128): u128 {
        mul_div_u128(a, b, RAY)
    }

    /// ray_div(a, b) = a * 1e27 / b, floor.
    public fun ray_div(a: u128, b: u128): u128 {
        mul_div_u128(a, RAY, b)
    }

    /// WAD helpers for u256 (same 1e18 scale, wider range).
    public fun wad_mul_u256(a: u256, b: u256): u256 {
        mul_div_u256(a, b, WAD_U256)
    }

    public fun wad_div_u256(a: u256, b: u256): u256 {
        mul_div_u256(a, WAD_U256, b)
    }

    public fun wad_mul_ceil_u256(a: u256, b: u256): u256 {
        mul_div_ceil_u256(a, b, WAD_U256)
    }

    public fun wad_div_ceil_u256(a: u256, b: u256): u256 {
        mul_div_ceil_u256(a, WAD_U256, b)
    }

    public fun ray_mul_u256(a: u256, b: u256): u256 {
        mul_div_u256(a, b, RAY_U256)
    }

    public fun ray_div_u256(a: u256, b: u256): u256 {
        mul_div_u256(a, RAY_U256, b)
    }

    // =================================================================
    // AMM helpers
    // =================================================================

    /// Constant-product quote: amount_out = amount_in * reserve_out / (reserve_in + amount_in).
    /// Floor rounding (favors pool). Aborts on zero reserves.
    public fun quote_amount_out(amount_in: u128, reserve_in: u128, reserve_out: u128): u128 {
        assert!(reserve_in > 0 && reserve_out > 0, E_INVALID_ARG);
        mul_div_u128(amount_in, reserve_out, reserve_in + amount_in)
    }

    /// Same quote for u256 reserves (deep-liquidity pools).
    public fun quote_amount_out_u256(amount_in: u256, reserve_in: u256, reserve_out: u256): u256 {
        assert!(reserve_in > 0 && reserve_out > 0, E_INVALID_ARG);
        mul_div_u256(amount_in, reserve_out, reserve_in + amount_in)
    }

    /// Average price as (numerator, denominator) scaled helper:
    /// returns amount * price_num / price_den with floor rounding.
    public fun apply_price(amount: u128, price_num: u128, price_den: u128): u128 {
        mul_div_u128(amount, price_num, price_den)
    }

    public fun apply_price_u256(amount: u256, price_num: u256, price_den: u256): u256 {
        mul_div_u256(amount, price_num, price_den)
    }

    /// Check `|a - b| <= tolerance` without overflow.
    public fun within_tolerance_u128(a: u128, b: u128, tolerance: u128): bool {
        diff_u128(a, b) <= tolerance
    }

    public fun within_tolerance_u64(a: u64, b: u64, tolerance: u64): bool {
        diff_u64(a, b) <= tolerance
    }

    public fun within_tolerance_u256(a: u256, b: u256, tolerance: u256): bool {
        diff_u256(a, b) <= tolerance
    }

    /// Slippage check used by DEX front-ends:
    /// aborts E_INVALID_ARG when actual < min_amount_out.
    public fun assert_min_out(actual_out: u128, min_amount_out: u128) {
        assert!(actual_out >= min_amount_out, E_INVALID_ARG);
    }

    public fun assert_min_out_u256(actual_out: u256, min_amount_out: u256) {
        assert!(actual_out >= min_amount_out, E_INVALID_ARG);
    }
}
