
<a name="0x2_math"></a>

# Module `0x2::math`

Production math library for DeFi on Kanari.

Covers all six Move integer widths (<code>u8</code> / <code>u16</code> / <code>u32</code> / <code>u64</code> / <code>u128</code> / <code>u256</code>).

Design rules:
- Hot / overflow-sensitive paths are natives (U256/U512 intermediate, single gas charge).
- Narrow widths (<code>u8</code>/<code>u16</code>/<code>u32</code>) are pure-Move wrappers over the <code>u64</code>/<code>u128</code>
natives: zero new Rust code, explicit <code><a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a></code> on narrowing.
- Everything else is pure Move built on top of those natives, so contracts get
checked fixed-point / bps / WAD / RAY helpers without reimplementing them.
- Aborting variants (<code>mul_div_*</code>, <code>pow_*</code>) abort with the codes below.
- Non-aborting variants (<code>try_*</code>) return <code>(success, value)</code> so contracts can
handle bad input gracefully (Move has no try/catch).


-  [Constants](#@Constants_0)
-  [Function `max_u8_value`](#0x2_math_max_u8_value)
-  [Function `max_u16_value`](#0x2_math_max_u16_value)
-  [Function `max_u32_value`](#0x2_math_max_u32_value)
-  [Function `max_u64_value`](#0x2_math_max_u64_value)
-  [Function `max_u128_value`](#0x2_math_max_u128_value)
-  [Function `max_u256_value`](#0x2_math_max_u256_value)
-  [Function `sqrt_u128`](#0x2_math_sqrt_u128)
-  [Function `sqrt_u64`](#0x2_math_sqrt_u64)
-  [Function `sqrt_u256`](#0x2_math_sqrt_u256)
-  [Function `pow_u64`](#0x2_math_pow_u64)
-  [Function `pow_u128`](#0x2_math_pow_u128)
-  [Function `pow_u256`](#0x2_math_pow_u256)
-  [Function `try_pow_u64`](#0x2_math_try_pow_u64)
-  [Function `try_pow_u128`](#0x2_math_try_pow_u128)
-  [Function `try_pow_u256`](#0x2_math_try_pow_u256)
-  [Function `mul_div_u128`](#0x2_math_mul_div_u128)
-  [Function `mul_div_round_u128`](#0x2_math_mul_div_round_u128)
-  [Function `try_mul_div_u128`](#0x2_math_try_mul_div_u128)
-  [Function `mul_add_u128`](#0x2_math_mul_add_u128)
-  [Function `mul_div_u256`](#0x2_math_mul_div_u256)
-  [Function `mul_div_round_u256`](#0x2_math_mul_div_round_u256)
-  [Function `try_mul_div_u256`](#0x2_math_try_mul_div_u256)
-  [Function `mul_add_u256`](#0x2_math_mul_add_u256)
-  [Function `min_u128`](#0x2_math_min_u128)
-  [Function `max_u128`](#0x2_math_max_u128)
-  [Function `min_u256`](#0x2_math_min_u256)
-  [Function `max_u256`](#0x2_math_max_u256)
-  [Function `average_u64`](#0x2_math_average_u64)
-  [Function `average_u128`](#0x2_math_average_u128)
-  [Function `average_u256`](#0x2_math_average_u256)
-  [Function `clamp_u64`](#0x2_math_clamp_u64)
-  [Function `clamp_u128`](#0x2_math_clamp_u128)
-  [Function `clamp_u256`](#0x2_math_clamp_u256)
-  [Function `log2_u64`](#0x2_math_log2_u64)
-  [Function `log2_u128`](#0x2_math_log2_u128)
-  [Function `log2_u256`](#0x2_math_log2_u256)
-  [Function `ceil_div_u64`](#0x2_math_ceil_div_u64)
-  [Function `ceil_div_u128`](#0x2_math_ceil_div_u128)
-  [Function `ceil_div_u256`](#0x2_math_ceil_div_u256)
-  [Function `min_u8`](#0x2_math_min_u8)
-  [Function `max_u8`](#0x2_math_max_u8)
-  [Function `diff_u8`](#0x2_math_diff_u8)
-  [Function `average_u8`](#0x2_math_average_u8)
-  [Function `clamp_u8`](#0x2_math_clamp_u8)
-  [Function `bound_u8`](#0x2_math_bound_u8)
-  [Function `sqrt_u8`](#0x2_math_sqrt_u8)
-  [Function `log2_u8`](#0x2_math_log2_u8)
-  [Function `is_pow2_u8`](#0x2_math_is_pow2_u8)
-  [Function `pow2_u8`](#0x2_math_pow2_u8)
-  [Function `pow_u8`](#0x2_math_pow_u8)
-  [Function `try_pow_u8`](#0x2_math_try_pow_u8)
-  [Function `ceil_div_u8`](#0x2_math_ceil_div_u8)
-  [Function `mul_div_u8`](#0x2_math_mul_div_u8)
-  [Function `mul_div_ceil_u8`](#0x2_math_mul_div_ceil_u8)
-  [Function `try_mul_div_u8`](#0x2_math_try_mul_div_u8)
-  [Function `mul_add_u8`](#0x2_math_mul_add_u8)
-  [Function `bps_mul_floor_u8`](#0x2_math_bps_mul_floor_u8)
-  [Function `bps_mul_ceil_u8`](#0x2_math_bps_mul_ceil_u8)
-  [Function `percent_mul_floor_u8`](#0x2_math_percent_mul_floor_u8)
-  [Function `percent_mul_ceil_u8`](#0x2_math_percent_mul_ceil_u8)
-  [Function `within_tolerance_u8`](#0x2_math_within_tolerance_u8)
-  [Function `min_u16`](#0x2_math_min_u16)
-  [Function `max_u16`](#0x2_math_max_u16)
-  [Function `diff_u16`](#0x2_math_diff_u16)
-  [Function `average_u16`](#0x2_math_average_u16)
-  [Function `clamp_u16`](#0x2_math_clamp_u16)
-  [Function `bound_u16`](#0x2_math_bound_u16)
-  [Function `sqrt_u16`](#0x2_math_sqrt_u16)
-  [Function `log2_u16`](#0x2_math_log2_u16)
-  [Function `is_pow2_u16`](#0x2_math_is_pow2_u16)
-  [Function `pow2_u16`](#0x2_math_pow2_u16)
-  [Function `pow_u16`](#0x2_math_pow_u16)
-  [Function `try_pow_u16`](#0x2_math_try_pow_u16)
-  [Function `ceil_div_u16`](#0x2_math_ceil_div_u16)
-  [Function `mul_div_u16`](#0x2_math_mul_div_u16)
-  [Function `mul_div_ceil_u16`](#0x2_math_mul_div_ceil_u16)
-  [Function `try_mul_div_u16`](#0x2_math_try_mul_div_u16)
-  [Function `mul_add_u16`](#0x2_math_mul_add_u16)
-  [Function `bps_mul_floor_u16`](#0x2_math_bps_mul_floor_u16)
-  [Function `bps_mul_ceil_u16`](#0x2_math_bps_mul_ceil_u16)
-  [Function `percent_mul_floor_u16`](#0x2_math_percent_mul_floor_u16)
-  [Function `percent_mul_ceil_u16`](#0x2_math_percent_mul_ceil_u16)
-  [Function `within_tolerance_u16`](#0x2_math_within_tolerance_u16)
-  [Function `min_u32`](#0x2_math_min_u32)
-  [Function `max_u32`](#0x2_math_max_u32)
-  [Function `diff_u32`](#0x2_math_diff_u32)
-  [Function `average_u32`](#0x2_math_average_u32)
-  [Function `clamp_u32`](#0x2_math_clamp_u32)
-  [Function `bound_u32`](#0x2_math_bound_u32)
-  [Function `sqrt_u32`](#0x2_math_sqrt_u32)
-  [Function `log2_u32`](#0x2_math_log2_u32)
-  [Function `is_pow2_u32`](#0x2_math_is_pow2_u32)
-  [Function `pow2_u32`](#0x2_math_pow2_u32)
-  [Function `pow_u32`](#0x2_math_pow_u32)
-  [Function `try_pow_u32`](#0x2_math_try_pow_u32)
-  [Function `ceil_div_u32`](#0x2_math_ceil_div_u32)
-  [Function `mul_div_u32`](#0x2_math_mul_div_u32)
-  [Function `mul_div_ceil_u32`](#0x2_math_mul_div_ceil_u32)
-  [Function `try_mul_div_u32`](#0x2_math_try_mul_div_u32)
-  [Function `mul_add_u32`](#0x2_math_mul_add_u32)
-  [Function `bps_mul_floor_u32`](#0x2_math_bps_mul_floor_u32)
-  [Function `bps_mul_ceil_u32`](#0x2_math_bps_mul_ceil_u32)
-  [Function `percent_mul_floor_u32`](#0x2_math_percent_mul_floor_u32)
-  [Function `percent_mul_ceil_u32`](#0x2_math_percent_mul_ceil_u32)
-  [Function `within_tolerance_u32`](#0x2_math_within_tolerance_u32)
-  [Function `min_u64`](#0x2_math_min_u64)
-  [Function `max_u64`](#0x2_math_max_u64)
-  [Function `diff_u64`](#0x2_math_diff_u64)
-  [Function `diff_u128`](#0x2_math_diff_u128)
-  [Function `diff_u256`](#0x2_math_diff_u256)
-  [Function `bound_u64`](#0x2_math_bound_u64)
-  [Function `bound_u128`](#0x2_math_bound_u128)
-  [Function `bound_u256`](#0x2_math_bound_u256)
-  [Function `is_pow2_u64`](#0x2_math_is_pow2_u64)
-  [Function `is_pow2_u128`](#0x2_math_is_pow2_u128)
-  [Function `is_pow2_u256`](#0x2_math_is_pow2_u256)
-  [Function `pow2_u128`](#0x2_math_pow2_u128)
-  [Function `pow2_u64`](#0x2_math_pow2_u64)
-  [Function `pow2_u256`](#0x2_math_pow2_u256)
-  [Function `divide_and_round_up`](#0x2_math_divide_and_round_up)
-  [Function `divide_and_round_up_u128`](#0x2_math_divide_and_round_up_u128)
-  [Function `mul_div_u64`](#0x2_math_mul_div_u64)
-  [Function `mul_div_ceil_u64`](#0x2_math_mul_div_ceil_u64)
-  [Function `mul_div_ceil_u128`](#0x2_math_mul_div_ceil_u128)
-  [Function `mul_div_ceil_u256`](#0x2_math_mul_div_ceil_u256)
-  [Function `mul_add_u64`](#0x2_math_mul_add_u64)
-  [Function `mul_div_round_up_u64`](#0x2_math_mul_div_round_up_u64)
-  [Function `mul_div_round_up_u128`](#0x2_math_mul_div_round_up_u128)
-  [Function `mul_div_round_up_u256`](#0x2_math_mul_div_round_up_u256)
-  [Function `try_mul_div_u64`](#0x2_math_try_mul_div_u64)
-  [Function `bps_mul_floor_u128`](#0x2_math_bps_mul_floor_u128)
-  [Function `bps_mul_ceil_u128`](#0x2_math_bps_mul_ceil_u128)
-  [Function `bps_mul_floor_u64`](#0x2_math_bps_mul_floor_u64)
-  [Function `bps_mul_ceil_u64`](#0x2_math_bps_mul_ceil_u64)
-  [Function `bps_mul_floor_u256`](#0x2_math_bps_mul_floor_u256)
-  [Function `bps_mul_ceil_u256`](#0x2_math_bps_mul_ceil_u256)
-  [Function `add_bps_u128`](#0x2_math_add_bps_u128)
-  [Function `sub_bps_u128`](#0x2_math_sub_bps_u128)
-  [Function `add_bps_u256`](#0x2_math_add_bps_u256)
-  [Function `sub_bps_u256`](#0x2_math_sub_bps_u256)
-  [Function `percent_mul_floor_u128`](#0x2_math_percent_mul_floor_u128)
-  [Function `percent_mul_ceil_u128`](#0x2_math_percent_mul_ceil_u128)
-  [Function `percent_mul_floor_u256`](#0x2_math_percent_mul_floor_u256)
-  [Function `percent_mul_ceil_u256`](#0x2_math_percent_mul_ceil_u256)
-  [Function `wad_mul`](#0x2_math_wad_mul)
-  [Function `wad_div`](#0x2_math_wad_div)
-  [Function `wad_mul_ceil`](#0x2_math_wad_mul_ceil)
-  [Function `wad_div_ceil`](#0x2_math_wad_div_ceil)
-  [Function `ray_mul`](#0x2_math_ray_mul)
-  [Function `ray_div`](#0x2_math_ray_div)
-  [Function `wad_mul_u256`](#0x2_math_wad_mul_u256)
-  [Function `wad_div_u256`](#0x2_math_wad_div_u256)
-  [Function `wad_mul_ceil_u256`](#0x2_math_wad_mul_ceil_u256)
-  [Function `wad_div_ceil_u256`](#0x2_math_wad_div_ceil_u256)
-  [Function `ray_mul_u256`](#0x2_math_ray_mul_u256)
-  [Function `ray_div_u256`](#0x2_math_ray_div_u256)
-  [Function `quote_amount_out`](#0x2_math_quote_amount_out)
-  [Function `quote_amount_out_u256`](#0x2_math_quote_amount_out_u256)
-  [Function `apply_price`](#0x2_math_apply_price)
-  [Function `apply_price_u256`](#0x2_math_apply_price_u256)
-  [Function `within_tolerance_u128`](#0x2_math_within_tolerance_u128)
-  [Function `within_tolerance_u64`](#0x2_math_within_tolerance_u64)
-  [Function `within_tolerance_u256`](#0x2_math_within_tolerance_u256)
-  [Function `assert_min_out`](#0x2_math_assert_min_out)
-  [Function `assert_min_out_u256`](#0x2_math_assert_min_out_u256)
-  [Function `checked_add_u8`](#0x2_math_checked_add_u8)
-  [Function `checked_sub_u8`](#0x2_math_checked_sub_u8)
-  [Function `checked_mul_u8`](#0x2_math_checked_mul_u8)
-  [Function `checked_div_u8`](#0x2_math_checked_div_u8)
-  [Function `checked_pow_u8`](#0x2_math_checked_pow_u8)
-  [Function `checked_add_u16`](#0x2_math_checked_add_u16)
-  [Function `checked_sub_u16`](#0x2_math_checked_sub_u16)
-  [Function `checked_mul_u16`](#0x2_math_checked_mul_u16)
-  [Function `checked_div_u16`](#0x2_math_checked_div_u16)
-  [Function `checked_pow_u16`](#0x2_math_checked_pow_u16)
-  [Function `checked_add_u32`](#0x2_math_checked_add_u32)
-  [Function `checked_sub_u32`](#0x2_math_checked_sub_u32)
-  [Function `checked_mul_u32`](#0x2_math_checked_mul_u32)
-  [Function `checked_div_u32`](#0x2_math_checked_div_u32)
-  [Function `checked_pow_u32`](#0x2_math_checked_pow_u32)
-  [Function `checked_add_u64`](#0x2_math_checked_add_u64)
-  [Function `checked_sub_u64`](#0x2_math_checked_sub_u64)
-  [Function `checked_mul_u64`](#0x2_math_checked_mul_u64)
-  [Function `checked_div_u64`](#0x2_math_checked_div_u64)
-  [Function `checked_pow_u64`](#0x2_math_checked_pow_u64)
-  [Function `checked_add_u128`](#0x2_math_checked_add_u128)
-  [Function `checked_sub_u128`](#0x2_math_checked_sub_u128)
-  [Function `checked_mul_u128`](#0x2_math_checked_mul_u128)
-  [Function `checked_div_u128`](#0x2_math_checked_div_u128)
-  [Function `checked_pow_u128`](#0x2_math_checked_pow_u128)
-  [Function `checked_add_u256`](#0x2_math_checked_add_u256)
-  [Function `checked_sub_u256`](#0x2_math_checked_sub_u256)
-  [Function `checked_mul_u256`](#0x2_math_checked_mul_u256)
-  [Function `checked_div_u256`](#0x2_math_checked_div_u256)
-  [Function `checked_pow_u256`](#0x2_math_checked_pow_u256)
-  [Function `saturating_add_u8`](#0x2_math_saturating_add_u8)
-  [Function `saturating_sub_u8`](#0x2_math_saturating_sub_u8)
-  [Function `saturating_mul_u8`](#0x2_math_saturating_mul_u8)
-  [Function `saturating_pow_u8`](#0x2_math_saturating_pow_u8)
-  [Function `saturating_mul_div_u8`](#0x2_math_saturating_mul_div_u8)
-  [Function `saturating_add_u16`](#0x2_math_saturating_add_u16)
-  [Function `saturating_sub_u16`](#0x2_math_saturating_sub_u16)
-  [Function `saturating_mul_u16`](#0x2_math_saturating_mul_u16)
-  [Function `saturating_pow_u16`](#0x2_math_saturating_pow_u16)
-  [Function `saturating_mul_div_u16`](#0x2_math_saturating_mul_div_u16)
-  [Function `saturating_add_u32`](#0x2_math_saturating_add_u32)
-  [Function `saturating_sub_u32`](#0x2_math_saturating_sub_u32)
-  [Function `saturating_mul_u32`](#0x2_math_saturating_mul_u32)
-  [Function `saturating_pow_u32`](#0x2_math_saturating_pow_u32)
-  [Function `saturating_mul_div_u32`](#0x2_math_saturating_mul_div_u32)
-  [Function `saturating_add_u64`](#0x2_math_saturating_add_u64)
-  [Function `saturating_sub_u64`](#0x2_math_saturating_sub_u64)
-  [Function `saturating_mul_u64`](#0x2_math_saturating_mul_u64)
-  [Function `saturating_pow_u64`](#0x2_math_saturating_pow_u64)
-  [Function `saturating_mul_div_u64`](#0x2_math_saturating_mul_div_u64)
-  [Function `saturating_add_u128`](#0x2_math_saturating_add_u128)
-  [Function `saturating_sub_u128`](#0x2_math_saturating_sub_u128)
-  [Function `saturating_mul_u128`](#0x2_math_saturating_mul_u128)
-  [Function `saturating_pow_u128`](#0x2_math_saturating_pow_u128)
-  [Function `saturating_mul_div_u128`](#0x2_math_saturating_mul_div_u128)
-  [Function `saturating_add_u256`](#0x2_math_saturating_add_u256)
-  [Function `saturating_sub_u256`](#0x2_math_saturating_sub_u256)
-  [Function `saturating_mul_u256`](#0x2_math_saturating_mul_u256)
-  [Function `saturating_pow_u256`](#0x2_math_saturating_pow_u256)
-  [Function `saturating_mul_div_u256`](#0x2_math_saturating_mul_div_u256)
-  [Function `checked_shl_u8`](#0x2_math_checked_shl_u8)
-  [Function `checked_shr_u8`](#0x2_math_checked_shr_u8)
-  [Function `lossless_shl_u8`](#0x2_math_lossless_shl_u8)
-  [Function `lossless_shr_u8`](#0x2_math_lossless_shr_u8)
-  [Function `lossless_div_u8`](#0x2_math_lossless_div_u8)
-  [Function `bitwise_not_u8`](#0x2_math_bitwise_not_u8)
-  [Function `checked_shl_u16`](#0x2_math_checked_shl_u16)
-  [Function `checked_shr_u16`](#0x2_math_checked_shr_u16)
-  [Function `lossless_shl_u16`](#0x2_math_lossless_shl_u16)
-  [Function `lossless_shr_u16`](#0x2_math_lossless_shr_u16)
-  [Function `lossless_div_u16`](#0x2_math_lossless_div_u16)
-  [Function `bitwise_not_u16`](#0x2_math_bitwise_not_u16)
-  [Function `checked_shl_u32`](#0x2_math_checked_shl_u32)
-  [Function `checked_shr_u32`](#0x2_math_checked_shr_u32)
-  [Function `lossless_shl_u32`](#0x2_math_lossless_shl_u32)
-  [Function `lossless_shr_u32`](#0x2_math_lossless_shr_u32)
-  [Function `lossless_div_u32`](#0x2_math_lossless_div_u32)
-  [Function `bitwise_not_u32`](#0x2_math_bitwise_not_u32)
-  [Function `checked_shl_u64`](#0x2_math_checked_shl_u64)
-  [Function `checked_shr_u64`](#0x2_math_checked_shr_u64)
-  [Function `lossless_shl_u64`](#0x2_math_lossless_shl_u64)
-  [Function `lossless_shr_u64`](#0x2_math_lossless_shr_u64)
-  [Function `lossless_div_u64`](#0x2_math_lossless_div_u64)
-  [Function `bitwise_not_u64`](#0x2_math_bitwise_not_u64)
-  [Function `checked_shl_u128`](#0x2_math_checked_shl_u128)
-  [Function `checked_shr_u128`](#0x2_math_checked_shr_u128)
-  [Function `lossless_shl_u128`](#0x2_math_lossless_shl_u128)
-  [Function `lossless_shr_u128`](#0x2_math_lossless_shr_u128)
-  [Function `lossless_div_u128`](#0x2_math_lossless_div_u128)
-  [Function `bitwise_not_u128`](#0x2_math_bitwise_not_u128)
-  [Function `checked_shl_u256`](#0x2_math_checked_shl_u256)
-  [Function `checked_shr_u256`](#0x2_math_checked_shr_u256)
-  [Function `lossless_shl_u256`](#0x2_math_lossless_shl_u256)
-  [Function `lossless_shr_u256`](#0x2_math_lossless_shr_u256)
-  [Function `lossless_div_u256`](#0x2_math_lossless_div_u256)
-  [Function `bitwise_not_u256`](#0x2_math_bitwise_not_u256)
-  [Function `try_as_u8_from_u16`](#0x2_math_try_as_u8_from_u16)
-  [Function `try_as_u8_from_u32`](#0x2_math_try_as_u8_from_u32)
-  [Function `try_as_u16_from_u32`](#0x2_math_try_as_u16_from_u32)
-  [Function `try_as_u8_from_u64`](#0x2_math_try_as_u8_from_u64)
-  [Function `try_as_u16_from_u64`](#0x2_math_try_as_u16_from_u64)
-  [Function `try_as_u32_from_u64`](#0x2_math_try_as_u32_from_u64)
-  [Function `try_as_u8_from_u128`](#0x2_math_try_as_u8_from_u128)
-  [Function `try_as_u16_from_u128`](#0x2_math_try_as_u16_from_u128)
-  [Function `try_as_u32_from_u128`](#0x2_math_try_as_u32_from_u128)
-  [Function `try_as_u64_from_u128`](#0x2_math_try_as_u64_from_u128)
-  [Function `try_as_u8_from_u256`](#0x2_math_try_as_u8_from_u256)
-  [Function `try_as_u16_from_u256`](#0x2_math_try_as_u16_from_u256)
-  [Function `try_as_u32_from_u256`](#0x2_math_try_as_u32_from_u256)
-  [Function `try_as_u64_from_u256`](#0x2_math_try_as_u64_from_u256)
-  [Function `try_as_u128_from_u256`](#0x2_math_try_as_u128_from_u256)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x2_math_BPS_SCALE"></a>

Basis points scale: 10_000 bps = 100%.


<pre><code><b>const</b> <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>: u128 = 10000;
</code></pre>



<a name="0x2_math_BPS_SCALE_U256"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_BPS_SCALE_U256">BPS_SCALE_U256</a>: u256 = 10000;
</code></pre>



<a name="0x2_math_BPS_SCALE_U64"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_BPS_SCALE_U64">BPS_SCALE_U64</a>: u64 = 10000;
</code></pre>



<a name="0x2_math_E_DIVIDE_BY_ZERO"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>: u64 = 2;
</code></pre>



<a name="0x2_math_E_INVALID_ARG"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_E_INVALID_ARG">E_INVALID_ARG</a>: u64 = 3;
</code></pre>



<a name="0x2_math_E_OUT_OF_RANGE"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>: u64 = 4;
</code></pre>



<a name="0x2_math_E_OVERFLOW"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>: u64 = 1;
</code></pre>



<a name="0x2_math_MAX_U128"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_MAX_U128">MAX_U128</a>: u128 = 340282366920938463463374607431768211455;
</code></pre>



<a name="0x2_math_MAX_U16"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_MAX_U16">MAX_U16</a>: u16 = 65535;
</code></pre>



<a name="0x2_math_MAX_U256"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_MAX_U256">MAX_U256</a>: u256 = 115792089237316195423570985008687907853269984665640564039457584007913129639935;
</code></pre>



<a name="0x2_math_MAX_U32"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_MAX_U32">MAX_U32</a>: u32 = 4294967295;
</code></pre>



<a name="0x2_math_MAX_U64"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_MAX_U64">MAX_U64</a>: u64 = 18446744073709551615;
</code></pre>



<a name="0x2_math_MAX_U8"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_MAX_U8">MAX_U8</a>: u8 = 255;
</code></pre>



<a name="0x2_math_PCT_SCALE"></a>

Percent scale: 100 = 100%.


<pre><code><b>const</b> <a href="math.md#0x2_math_PCT_SCALE">PCT_SCALE</a>: u128 = 100;
</code></pre>



<a name="0x2_math_PCT_SCALE_U256"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_PCT_SCALE_U256">PCT_SCALE_U256</a>: u256 = 100;
</code></pre>



<a name="0x2_math_RAY"></a>

RAY fixed point: 1e27 (e.g. Maker-style rates).


<pre><code><b>const</b> <a href="math.md#0x2_math_RAY">RAY</a>: u128 = 1000000000000000000000000000;
</code></pre>



<a name="0x2_math_RAY_U256"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_RAY_U256">RAY_U256</a>: u256 = 1000000000000000000000000000;
</code></pre>



<a name="0x2_math_WAD"></a>

WAD fixed point: 1e18 (e.g. ERC20-style 18 decimals).


<pre><code><b>const</b> <a href="math.md#0x2_math_WAD">WAD</a>: u128 = 1000000000000000000;
</code></pre>



<a name="0x2_math_WAD_U256"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_WAD_U256">WAD_U256</a>: u256 = 1000000000000000000;
</code></pre>



<a name="0x2_math_max_u8_value"></a>

## Function `max_u8_value`

Width limits, exposed so contracts can clamp / validate without hardcoding.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u8_value">max_u8_value</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u8_value">max_u8_value</a>(): u8 {
    <a href="math.md#0x2_math_MAX_U8">MAX_U8</a>
}
</code></pre>



</details>

<a name="0x2_math_max_u16_value"></a>

## Function `max_u16_value`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u16_value">max_u16_value</a>(): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u16_value">max_u16_value</a>(): u16 {
    <a href="math.md#0x2_math_MAX_U16">MAX_U16</a>
}
</code></pre>



</details>

<a name="0x2_math_max_u32_value"></a>

## Function `max_u32_value`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u32_value">max_u32_value</a>(): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u32_value">max_u32_value</a>(): u32 {
    <a href="math.md#0x2_math_MAX_U32">MAX_U32</a>
}
</code></pre>



</details>

<a name="0x2_math_max_u64_value"></a>

## Function `max_u64_value`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u64_value">max_u64_value</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u64_value">max_u64_value</a>(): u64 {
    <a href="math.md#0x2_math_MAX_U64">MAX_U64</a>
}
</code></pre>



</details>

<a name="0x2_math_max_u128_value"></a>

## Function `max_u128_value`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u128_value">max_u128_value</a>(): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u128_value">max_u128_value</a>(): u128 {
    <a href="math.md#0x2_math_MAX_U128">MAX_U128</a>
}
</code></pre>



</details>

<a name="0x2_math_max_u256_value"></a>

## Function `max_u256_value`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u256_value">max_u256_value</a>(): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u256_value">max_u256_value</a>(): u256 {
    <a href="math.md#0x2_math_MAX_U256">MAX_U256</a>
}
</code></pre>



</details>

<a name="0x2_math_sqrt_u128"></a>

## Function `sqrt_u128`

Floor sqrt of a u128.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u128">sqrt_u128</a>(x: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u128">sqrt_u128</a>(x: u128): u128;
</code></pre>



</details>

<a name="0x2_math_sqrt_u64"></a>

## Function `sqrt_u64`

Floor sqrt of a u64.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u64">sqrt_u64</a>(x: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u64">sqrt_u64</a>(x: u64): u64;
</code></pre>



</details>

<a name="0x2_math_sqrt_u256"></a>

## Function `sqrt_u256`

Floor sqrt of a u256.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u256">sqrt_u256</a>(x: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u256">sqrt_u256</a>(x: u256): u256;
</code></pre>



</details>

<a name="0x2_math_pow_u64"></a>

## Function `pow_u64`

base ^ exponent. Aborts with E_OVERFLOW on overflow.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow_u64">pow_u64</a>(base: u64, exponent: u8): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_pow_u64">pow_u64</a>(base: u64, exponent: u8): u64;
</code></pre>



</details>

<a name="0x2_math_pow_u128"></a>

## Function `pow_u128`

base ^ exponent. Aborts with E_OVERFLOW on overflow.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow_u128">pow_u128</a>(base: u128, exponent: u32): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_pow_u128">pow_u128</a>(base: u128, exponent: u32): u128;
</code></pre>



</details>

<a name="0x2_math_pow_u256"></a>

## Function `pow_u256`

base ^ exponent. Aborts with E_OVERFLOW on overflow.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow_u256">pow_u256</a>(base: u256, exponent: u32): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_pow_u256">pow_u256</a>(base: u256, exponent: u32): u256;
</code></pre>



</details>

<a name="0x2_math_try_pow_u64"></a>

## Function `try_pow_u64`

Non-aborting pow. Returns (success, value); (false, 0) on overflow.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_pow_u64">try_pow_u64</a>(base: u64, exponent: u8): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_try_pow_u64">try_pow_u64</a>(base: u64, exponent: u8): (bool, u64);
</code></pre>



</details>

<a name="0x2_math_try_pow_u128"></a>

## Function `try_pow_u128`

Non-aborting pow. Returns (success, value); (false, 0) on overflow.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_pow_u128">try_pow_u128</a>(base: u128, exponent: u32): (bool, u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_try_pow_u128">try_pow_u128</a>(base: u128, exponent: u32): (bool, u128);
</code></pre>



</details>

<a name="0x2_math_try_pow_u256"></a>

## Function `try_pow_u256`

Non-aborting pow. Returns (success, value); (false, 0) on overflow.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_pow_u256">try_pow_u256</a>(base: u256, exponent: u32): (bool, u256)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_try_pow_u256">try_pow_u256</a>(base: u256, exponent: u32): (bool, u256);
</code></pre>



</details>

<a name="0x2_math_mul_div_u128"></a>

## Function `mul_div_u128`

(x * y) / z with U256 intermediate, floor rounding.
Aborts E_DIVIDE_BY_ZERO when z == 0, E_OVERFLOW when result > MAX_U128.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>(x: u128, y: u128, z: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>(x: u128, y: u128, z: u128): u128;
</code></pre>



</details>

<a name="0x2_math_mul_div_round_u128"></a>

## Function `mul_div_round_u128`

(x * y) / z with explicit rounding: false = floor, true = ceil.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_round_u128">mul_div_round_u128</a>(x: u128, y: u128, z: u128, round_up: bool): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_round_u128">mul_div_round_u128</a>(
    x: u128, y: u128, z: u128, round_up: bool
): u128;
</code></pre>



</details>

<a name="0x2_math_try_mul_div_u128"></a>

## Function `try_mul_div_u128`

Non-aborting (x * y) / z (floor). (false, 0) on div-by-zero or overflow.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_mul_div_u128">try_mul_div_u128</a>(x: u128, y: u128, z: u128): (bool, u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_try_mul_div_u128">try_mul_div_u128</a>(x: u128, y: u128, z: u128): (bool, u128);
</code></pre>



</details>

<a name="0x2_math_mul_add_u128"></a>

## Function `mul_add_u128`

(x * y) + z with U256 intermediate. Aborts E_OVERFLOW when result > MAX_U128.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_add_u128">mul_add_u128</a>(x: u128, y: u128, z: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_mul_add_u128">mul_add_u128</a>(x: u128, y: u128, z: u128): u128;
</code></pre>



</details>

<a name="0x2_math_mul_div_u256"></a>

## Function `mul_div_u256`

(x * y) / z with U512 intermediate, floor rounding.
Aborts E_DIVIDE_BY_ZERO when z == 0, E_OVERFLOW when result > MAX_U256.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u256">mul_div_u256</a>(x: u256, y: u256, z: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u256">mul_div_u256</a>(x: u256, y: u256, z: u256): u256;
</code></pre>



</details>

<a name="0x2_math_mul_div_round_u256"></a>

## Function `mul_div_round_u256`

(x * y) / z with explicit rounding: false = floor, true = ceil.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_round_u256">mul_div_round_u256</a>(x: u256, y: u256, z: u256, round_up: bool): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_round_u256">mul_div_round_u256</a>(
    x: u256, y: u256, z: u256, round_up: bool
): u256;
</code></pre>



</details>

<a name="0x2_math_try_mul_div_u256"></a>

## Function `try_mul_div_u256`

Non-aborting (x * y) / z (floor). (false, 0) on div-by-zero or overflow.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_mul_div_u256">try_mul_div_u256</a>(x: u256, y: u256, z: u256): (bool, u256)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_try_mul_div_u256">try_mul_div_u256</a>(x: u256, y: u256, z: u256): (bool, u256);
</code></pre>



</details>

<a name="0x2_math_mul_add_u256"></a>

## Function `mul_add_u256`

(x * y) + z with U512 intermediate. Aborts E_OVERFLOW when result > MAX_U256.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_add_u256">mul_add_u256</a>(x: u256, y: u256, z: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_mul_add_u256">mul_add_u256</a>(x: u256, y: u256, z: u256): u256;
</code></pre>



</details>

<a name="0x2_math_min_u128"></a>

## Function `min_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u128">min_u128</a>(x: u128, y: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_min_u128">min_u128</a>(x: u128, y: u128): u128;
</code></pre>



</details>

<a name="0x2_math_max_u128"></a>

## Function `max_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u128">max_u128</a>(x: u128, y: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_max_u128">max_u128</a>(x: u128, y: u128): u128;
</code></pre>



</details>

<a name="0x2_math_min_u256"></a>

## Function `min_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u256">min_u256</a>(x: u256, y: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_min_u256">min_u256</a>(x: u256, y: u256): u256;
</code></pre>



</details>

<a name="0x2_math_max_u256"></a>

## Function `max_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u256">max_u256</a>(x: u256, y: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_max_u256">max_u256</a>(x: u256, y: u256): u256;
</code></pre>



</details>

<a name="0x2_math_average_u64"></a>

## Function `average_u64`

Overflow-safe average, floor: (a & b) + ((a ^ b) >> 1).


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_average_u64">average_u64</a>(a: u64, b: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_average_u64">average_u64</a>(a: u64, b: u64): u64;
</code></pre>



</details>

<a name="0x2_math_average_u128"></a>

## Function `average_u128`

Overflow-safe average, floor.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_average_u128">average_u128</a>(a: u128, b: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_average_u128">average_u128</a>(a: u128, b: u128): u128;
</code></pre>



</details>

<a name="0x2_math_average_u256"></a>

## Function `average_u256`

Overflow-safe average, floor.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_average_u256">average_u256</a>(a: u256, b: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_average_u256">average_u256</a>(a: u256, b: u256): u256;
</code></pre>



</details>

<a name="0x2_math_clamp_u64"></a>

## Function `clamp_u64`

Clamp x into [lo, hi]. Aborts E_INVALID_ARG when lo > hi.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_clamp_u64">clamp_u64</a>(x: u64, lo: u64, hi: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_clamp_u64">clamp_u64</a>(x: u64, lo: u64, hi: u64): u64;
</code></pre>



</details>

<a name="0x2_math_clamp_u128"></a>

## Function `clamp_u128`

Clamp x into [lo, hi]. Aborts E_INVALID_ARG when lo > hi.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_clamp_u128">clamp_u128</a>(x: u128, lo: u128, hi: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_clamp_u128">clamp_u128</a>(x: u128, lo: u128, hi: u128): u128;
</code></pre>



</details>

<a name="0x2_math_clamp_u256"></a>

## Function `clamp_u256`

Clamp x into [lo, hi]. Aborts E_INVALID_ARG when lo > hi.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_clamp_u256">clamp_u256</a>(x: u256, lo: u256, hi: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_clamp_u256">clamp_u256</a>(x: u256, lo: u256, hi: u256): u256;
</code></pre>



</details>

<a name="0x2_math_log2_u64"></a>

## Function `log2_u64`

Floor log2. Aborts E_INVALID_ARG on 0.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_log2_u64">log2_u64</a>(x: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_log2_u64">log2_u64</a>(x: u64): u64;
</code></pre>



</details>

<a name="0x2_math_log2_u128"></a>

## Function `log2_u128`

Floor log2. Aborts E_INVALID_ARG on 0.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_log2_u128">log2_u128</a>(x: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_log2_u128">log2_u128</a>(x: u128): u128;
</code></pre>



</details>

<a name="0x2_math_log2_u256"></a>

## Function `log2_u256`

Floor log2. Aborts E_INVALID_ARG on 0.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_log2_u256">log2_u256</a>(x: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_log2_u256">log2_u256</a>(x: u256): u256;
</code></pre>



</details>

<a name="0x2_math_ceil_div_u64"></a>

## Function `ceil_div_u64`

Round-up division: (x + y - 1) / y. Aborts E_DIVIDE_BY_ZERO on y == 0.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ceil_div_u64">ceil_div_u64</a>(x: u64, y: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_ceil_div_u64">ceil_div_u64</a>(x: u64, y: u64): u64;
</code></pre>



</details>

<a name="0x2_math_ceil_div_u128"></a>

## Function `ceil_div_u128`

Round-up division: (x + y - 1) / y. Aborts E_DIVIDE_BY_ZERO on y == 0.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ceil_div_u128">ceil_div_u128</a>(x: u128, y: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_ceil_div_u128">ceil_div_u128</a>(x: u128, y: u128): u128;
</code></pre>



</details>

<a name="0x2_math_ceil_div_u256"></a>

## Function `ceil_div_u256`

Round-up division: (x + y - 1) / y. Aborts E_DIVIDE_BY_ZERO on y == 0.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ceil_div_u256">ceil_div_u256</a>(x: u256, y: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_ceil_div_u256">ceil_div_u256</a>(x: u256, y: u256): u256;
</code></pre>



</details>

<a name="0x2_math_min_u8"></a>

## Function `min_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u8">min_u8</a>(x: u8, y: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u8">min_u8</a>(x: u8, y: u8): u8 {
    <b>if</b> (x &lt; y) { x }
    <b>else</b> { y }
}
</code></pre>



</details>

<a name="0x2_math_max_u8"></a>

## Function `max_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u8">max_u8</a>(x: u8, y: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u8">max_u8</a>(x: u8, y: u8): u8 {
    <b>if</b> (x &gt; y) { x }
    <b>else</b> { y }
}
</code></pre>



</details>

<a name="0x2_math_diff_u8"></a>

## Function `diff_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u8">diff_u8</a>(x: u8, y: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u8">diff_u8</a>(x: u8, y: u8): u8 {
    <b>if</b> (x &gt; y) { x - y }
    <b>else</b> { y - x }
}
</code></pre>



</details>

<a name="0x2_math_average_u8"></a>

## Function `average_u8`

Overflow-safe average, floor. Result always fits u8.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_average_u8">average_u8</a>(a: u8, b: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_average_u8">average_u8</a>(a: u8, b: u8): u8 {
    ((<a href="math.md#0x2_math_average_u64">average_u64</a>((a <b>as</b> u64), (b <b>as</b> u64))) <b>as</b> u8)
}
</code></pre>



</details>

<a name="0x2_math_clamp_u8"></a>

## Function `clamp_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_clamp_u8">clamp_u8</a>(x: u8, lo: u8, hi: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_clamp_u8">clamp_u8</a>(x: u8, lo: u8, hi: u8): u8 {
    <b>assert</b>!(lo &lt;= hi, <a href="math.md#0x2_math_E_INVALID_ARG">E_INVALID_ARG</a>);
    <b>if</b> (x &lt; lo) { lo }
    <b>else</b> <b>if</b> (x &gt; hi) { hi }
    <b>else</b> { x }
}
</code></pre>



</details>

<a name="0x2_math_bound_u8"></a>

## Function `bound_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bound_u8">bound_u8</a>(x: u8, lo: u8, hi: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bound_u8">bound_u8</a>(x: u8, lo: u8, hi: u8): u8 {
    <a href="math.md#0x2_math_clamp_u8">clamp_u8</a>(x, lo, hi)
}
</code></pre>



</details>

<a name="0x2_math_sqrt_u8"></a>

## Function `sqrt_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u8">sqrt_u8</a>(x: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u8">sqrt_u8</a>(x: u8): u8 {
    ((<a href="math.md#0x2_math_sqrt_u64">sqrt_u64</a>((x <b>as</b> u64))) <b>as</b> u8)
}
</code></pre>



</details>

<a name="0x2_math_log2_u8"></a>

## Function `log2_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_log2_u8">log2_u8</a>(x: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_log2_u8">log2_u8</a>(x: u8): u8 {
    ((<a href="math.md#0x2_math_log2_u64">log2_u64</a>((x <b>as</b> u64))) <b>as</b> u8)
}
</code></pre>



</details>

<a name="0x2_math_is_pow2_u8"></a>

## Function `is_pow2_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_is_pow2_u8">is_pow2_u8</a>(x: u8): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_is_pow2_u8">is_pow2_u8</a>(x: u8): bool {
    x &gt; 0 && (x & (x - 1)) == 0
}
</code></pre>



</details>

<a name="0x2_math_pow2_u8"></a>

## Function `pow2_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow2_u8">pow2_u8</a>(exp: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow2_u8">pow2_u8</a>(exp: u8): u8 {
    <a href="math.md#0x2_math_pow_u8">pow_u8</a>(2, exp)
}
</code></pre>



</details>

<a name="0x2_math_pow_u8"></a>

## Function `pow_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow_u8">pow_u8</a>(base: u8, exponent: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow_u8">pow_u8</a>(base: u8, exponent: u8): u8 {
    <b>let</b> r = <a href="math.md#0x2_math_pow_u64">pow_u64</a>((base <b>as</b> u64), exponent);
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U8">MAX_U8</a> <b>as</b> u64), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u8)
}
</code></pre>



</details>

<a name="0x2_math_try_pow_u8"></a>

## Function `try_pow_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_pow_u8">try_pow_u8</a>(base: u8, exponent: u8): (bool, u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_pow_u8">try_pow_u8</a>(base: u8, exponent: u8): (bool, u8) {
    <b>let</b> (ok, r) = <a href="math.md#0x2_math_try_pow_u64">try_pow_u64</a>((base <b>as</b> u64), exponent);
    <b>if</b> (!ok || r &gt; (<a href="math.md#0x2_math_MAX_U8">MAX_U8</a> <b>as</b> u64)) {
        (<b>false</b>, 0)
    } <b>else</b> {
        (<b>true</b>, (r <b>as</b> u8))
    }
}
</code></pre>



</details>

<a name="0x2_math_ceil_div_u8"></a>

## Function `ceil_div_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ceil_div_u8">ceil_div_u8</a>(x: u8, y: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ceil_div_u8">ceil_div_u8</a>(x: u8, y: u8): u8 {
    <b>assert</b>!(y &gt; 0, <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>if</b> (x == 0) { 0 }
    <b>else</b> {
        ((x - 1) / y) + 1
    }
}
</code></pre>



</details>

<a name="0x2_math_mul_div_u8"></a>

## Function `mul_div_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u8">mul_div_u8</a>(x: u8, y: u8, z: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u8">mul_div_u8</a>(x: u8, y: u8, z: u8): u8 {
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128));
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U8">MAX_U8</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u8)
}
</code></pre>



</details>

<a name="0x2_math_mul_div_ceil_u8"></a>

## Function `mul_div_ceil_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_ceil_u8">mul_div_ceil_u8</a>(x: u8, y: u8, z: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_ceil_u8">mul_div_ceil_u8</a>(x: u8, y: u8, z: u8): u8 {
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_round_u128">mul_div_round_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128), <b>true</b>);
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U8">MAX_U8</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u8)
}
</code></pre>



</details>

<a name="0x2_math_try_mul_div_u8"></a>

## Function `try_mul_div_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_mul_div_u8">try_mul_div_u8</a>(x: u8, y: u8, z: u8): (bool, u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_mul_div_u8">try_mul_div_u8</a>(x: u8, y: u8, z: u8): (bool, u8) {
    <b>let</b> (ok, r) = <a href="math.md#0x2_math_try_mul_div_u128">try_mul_div_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128));
    <b>if</b> (!ok || r &gt; (<a href="math.md#0x2_math_MAX_U8">MAX_U8</a> <b>as</b> u128)) {
        (<b>false</b>, 0)
    } <b>else</b> {
        (<b>true</b>, (r <b>as</b> u8))
    }
}
</code></pre>



</details>

<a name="0x2_math_mul_add_u8"></a>

## Function `mul_add_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_add_u8">mul_add_u8</a>(x: u8, y: u8, z: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_add_u8">mul_add_u8</a>(x: u8, y: u8, z: u8): u8 {
    <b>let</b> r = <a href="math.md#0x2_math_mul_add_u128">mul_add_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128));
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U8">MAX_U8</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u8)
}
</code></pre>



</details>

<a name="0x2_math_bps_mul_floor_u8"></a>

## Function `bps_mul_floor_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_floor_u8">bps_mul_floor_u8</a>(amount: u8, bps: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_floor_u8">bps_mul_floor_u8</a>(amount: u8, bps: u8): u8 {
    // 10_000 does not fit u8, so scale through u128 explicitly.
    <b>assert</b>!((bps <b>as</b> u128) &lt;= <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>((amount <b>as</b> u128), (bps <b>as</b> u128), <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>);
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U8">MAX_U8</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u8)
}
</code></pre>



</details>

<a name="0x2_math_bps_mul_ceil_u8"></a>

## Function `bps_mul_ceil_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_ceil_u8">bps_mul_ceil_u8</a>(amount: u8, bps: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_ceil_u8">bps_mul_ceil_u8</a>(amount: u8, bps: u8): u8 {
    // NOTE: u8 caps bps at 255; amounts stay exact via u128 intermediate.
    // 10_000 does not fit u8, so scale through u128 explicitly.
    <b>assert</b>!((bps <b>as</b> u128) &lt;= <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_round_u128">mul_div_round_u128</a>((amount <b>as</b> u128), (bps <b>as</b> u128), <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>, <b>true</b>);
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U8">MAX_U8</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u8)
}
</code></pre>



</details>

<a name="0x2_math_percent_mul_floor_u8"></a>

## Function `percent_mul_floor_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_floor_u8">percent_mul_floor_u8</a>(amount: u8, pct: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_floor_u8">percent_mul_floor_u8</a>(amount: u8, pct: u8): u8 {
    <b>assert</b>!((pct <b>as</b> u128) &lt;= <a href="math.md#0x2_math_PCT_SCALE">PCT_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_u8">mul_div_u8</a>(amount, pct, 100)
}
</code></pre>



</details>

<a name="0x2_math_percent_mul_ceil_u8"></a>

## Function `percent_mul_ceil_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_ceil_u8">percent_mul_ceil_u8</a>(amount: u8, pct: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_ceil_u8">percent_mul_ceil_u8</a>(amount: u8, pct: u8): u8 {
    <b>assert</b>!((pct <b>as</b> u128) &lt;= <a href="math.md#0x2_math_PCT_SCALE">PCT_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_ceil_u8">mul_div_ceil_u8</a>(amount, pct, 100)
}
</code></pre>



</details>

<a name="0x2_math_within_tolerance_u8"></a>

## Function `within_tolerance_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_within_tolerance_u8">within_tolerance_u8</a>(a: u8, b: u8, tolerance: u8): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_within_tolerance_u8">within_tolerance_u8</a>(a: u8, b: u8, tolerance: u8): bool {
    <a href="math.md#0x2_math_diff_u8">diff_u8</a>(a, b) &lt;= tolerance
}
</code></pre>



</details>

<a name="0x2_math_min_u16"></a>

## Function `min_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u16">min_u16</a>(x: u16, y: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u16">min_u16</a>(x: u16, y: u16): u16 {
    <b>if</b> (x &lt; y) { x }
    <b>else</b> { y }
}
</code></pre>



</details>

<a name="0x2_math_max_u16"></a>

## Function `max_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u16">max_u16</a>(x: u16, y: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u16">max_u16</a>(x: u16, y: u16): u16 {
    <b>if</b> (x &gt; y) { x }
    <b>else</b> { y }
}
</code></pre>



</details>

<a name="0x2_math_diff_u16"></a>

## Function `diff_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u16">diff_u16</a>(x: u16, y: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u16">diff_u16</a>(x: u16, y: u16): u16 {
    <b>if</b> (x &gt; y) { x - y }
    <b>else</b> { y - x }
}
</code></pre>



</details>

<a name="0x2_math_average_u16"></a>

## Function `average_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_average_u16">average_u16</a>(a: u16, b: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_average_u16">average_u16</a>(a: u16, b: u16): u16 {
    ((<a href="math.md#0x2_math_average_u64">average_u64</a>((a <b>as</b> u64), (b <b>as</b> u64))) <b>as</b> u16)
}
</code></pre>



</details>

<a name="0x2_math_clamp_u16"></a>

## Function `clamp_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_clamp_u16">clamp_u16</a>(x: u16, lo: u16, hi: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_clamp_u16">clamp_u16</a>(x: u16, lo: u16, hi: u16): u16 {
    <b>assert</b>!(lo &lt;= hi, <a href="math.md#0x2_math_E_INVALID_ARG">E_INVALID_ARG</a>);
    <b>if</b> (x &lt; lo) { lo }
    <b>else</b> <b>if</b> (x &gt; hi) { hi }
    <b>else</b> { x }
}
</code></pre>



</details>

<a name="0x2_math_bound_u16"></a>

## Function `bound_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bound_u16">bound_u16</a>(x: u16, lo: u16, hi: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bound_u16">bound_u16</a>(x: u16, lo: u16, hi: u16): u16 {
    <a href="math.md#0x2_math_clamp_u16">clamp_u16</a>(x, lo, hi)
}
</code></pre>



</details>

<a name="0x2_math_sqrt_u16"></a>

## Function `sqrt_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u16">sqrt_u16</a>(x: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u16">sqrt_u16</a>(x: u16): u16 {
    ((<a href="math.md#0x2_math_sqrt_u64">sqrt_u64</a>((x <b>as</b> u64))) <b>as</b> u16)
}
</code></pre>



</details>

<a name="0x2_math_log2_u16"></a>

## Function `log2_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_log2_u16">log2_u16</a>(x: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_log2_u16">log2_u16</a>(x: u16): u16 {
    ((<a href="math.md#0x2_math_log2_u64">log2_u64</a>((x <b>as</b> u64))) <b>as</b> u16)
}
</code></pre>



</details>

<a name="0x2_math_is_pow2_u16"></a>

## Function `is_pow2_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_is_pow2_u16">is_pow2_u16</a>(x: u16): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_is_pow2_u16">is_pow2_u16</a>(x: u16): bool {
    x &gt; 0 && (x & (x - 1)) == 0
}
</code></pre>



</details>

<a name="0x2_math_pow2_u16"></a>

## Function `pow2_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow2_u16">pow2_u16</a>(exp: u8): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow2_u16">pow2_u16</a>(exp: u8): u16 {
    <a href="math.md#0x2_math_pow_u16">pow_u16</a>(2, exp)
}
</code></pre>



</details>

<a name="0x2_math_pow_u16"></a>

## Function `pow_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow_u16">pow_u16</a>(base: u16, exponent: u8): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow_u16">pow_u16</a>(base: u16, exponent: u8): u16 {
    <b>let</b> r = <a href="math.md#0x2_math_pow_u64">pow_u64</a>((base <b>as</b> u64), exponent);
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U16">MAX_U16</a> <b>as</b> u64), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u16)
}
</code></pre>



</details>

<a name="0x2_math_try_pow_u16"></a>

## Function `try_pow_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_pow_u16">try_pow_u16</a>(base: u16, exponent: u8): (bool, u16)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_pow_u16">try_pow_u16</a>(base: u16, exponent: u8): (bool, u16) {
    <b>let</b> (ok, r) = <a href="math.md#0x2_math_try_pow_u64">try_pow_u64</a>((base <b>as</b> u64), exponent);
    <b>if</b> (!ok || r &gt; (<a href="math.md#0x2_math_MAX_U16">MAX_U16</a> <b>as</b> u64)) {
        (<b>false</b>, 0)
    } <b>else</b> {
        (<b>true</b>, (r <b>as</b> u16))
    }
}
</code></pre>



</details>

<a name="0x2_math_ceil_div_u16"></a>

## Function `ceil_div_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ceil_div_u16">ceil_div_u16</a>(x: u16, y: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ceil_div_u16">ceil_div_u16</a>(x: u16, y: u16): u16 {
    <b>assert</b>!(y &gt; 0, <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>if</b> (x == 0) { 0 }
    <b>else</b> {
        ((x - 1) / y) + 1
    }
}
</code></pre>



</details>

<a name="0x2_math_mul_div_u16"></a>

## Function `mul_div_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u16">mul_div_u16</a>(x: u16, y: u16, z: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u16">mul_div_u16</a>(x: u16, y: u16, z: u16): u16 {
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128));
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U16">MAX_U16</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u16)
}
</code></pre>



</details>

<a name="0x2_math_mul_div_ceil_u16"></a>

## Function `mul_div_ceil_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_ceil_u16">mul_div_ceil_u16</a>(x: u16, y: u16, z: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_ceil_u16">mul_div_ceil_u16</a>(x: u16, y: u16, z: u16): u16 {
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_round_u128">mul_div_round_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128), <b>true</b>);
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U16">MAX_U16</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u16)
}
</code></pre>



</details>

<a name="0x2_math_try_mul_div_u16"></a>

## Function `try_mul_div_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_mul_div_u16">try_mul_div_u16</a>(x: u16, y: u16, z: u16): (bool, u16)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_mul_div_u16">try_mul_div_u16</a>(x: u16, y: u16, z: u16): (bool, u16) {
    <b>let</b> (ok, r) = <a href="math.md#0x2_math_try_mul_div_u128">try_mul_div_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128));
    <b>if</b> (!ok || r &gt; (<a href="math.md#0x2_math_MAX_U16">MAX_U16</a> <b>as</b> u128)) {
        (<b>false</b>, 0)
    } <b>else</b> {
        (<b>true</b>, (r <b>as</b> u16))
    }
}
</code></pre>



</details>

<a name="0x2_math_mul_add_u16"></a>

## Function `mul_add_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_add_u16">mul_add_u16</a>(x: u16, y: u16, z: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_add_u16">mul_add_u16</a>(x: u16, y: u16, z: u16): u16 {
    <b>let</b> r = <a href="math.md#0x2_math_mul_add_u128">mul_add_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128));
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U16">MAX_U16</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u16)
}
</code></pre>



</details>

<a name="0x2_math_bps_mul_floor_u16"></a>

## Function `bps_mul_floor_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_floor_u16">bps_mul_floor_u16</a>(amount: u16, bps: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_floor_u16">bps_mul_floor_u16</a>(amount: u16, bps: u16): u16 {
    <b>assert</b>!((bps <b>as</b> u128) &lt;= <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>((amount <b>as</b> u128), (bps <b>as</b> u128), <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>);
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U16">MAX_U16</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u16)
}
</code></pre>



</details>

<a name="0x2_math_bps_mul_ceil_u16"></a>

## Function `bps_mul_ceil_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_ceil_u16">bps_mul_ceil_u16</a>(amount: u16, bps: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_ceil_u16">bps_mul_ceil_u16</a>(amount: u16, bps: u16): u16 {
    <b>assert</b>!((bps <b>as</b> u128) &lt;= <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_round_u128">mul_div_round_u128</a>((amount <b>as</b> u128), (bps <b>as</b> u128), <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>, <b>true</b>);
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U16">MAX_U16</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u16)
}
</code></pre>



</details>

<a name="0x2_math_percent_mul_floor_u16"></a>

## Function `percent_mul_floor_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_floor_u16">percent_mul_floor_u16</a>(amount: u16, pct: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_floor_u16">percent_mul_floor_u16</a>(amount: u16, pct: u16): u16 {
    <b>assert</b>!((pct <b>as</b> u128) &lt;= <a href="math.md#0x2_math_PCT_SCALE">PCT_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_u16">mul_div_u16</a>(amount, pct, 100)
}
</code></pre>



</details>

<a name="0x2_math_percent_mul_ceil_u16"></a>

## Function `percent_mul_ceil_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_ceil_u16">percent_mul_ceil_u16</a>(amount: u16, pct: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_ceil_u16">percent_mul_ceil_u16</a>(amount: u16, pct: u16): u16 {
    <b>assert</b>!((pct <b>as</b> u128) &lt;= <a href="math.md#0x2_math_PCT_SCALE">PCT_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_ceil_u16">mul_div_ceil_u16</a>(amount, pct, 100)
}
</code></pre>



</details>

<a name="0x2_math_within_tolerance_u16"></a>

## Function `within_tolerance_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_within_tolerance_u16">within_tolerance_u16</a>(a: u16, b: u16, tolerance: u16): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_within_tolerance_u16">within_tolerance_u16</a>(a: u16, b: u16, tolerance: u16): bool {
    <a href="math.md#0x2_math_diff_u16">diff_u16</a>(a, b) &lt;= tolerance
}
</code></pre>



</details>

<a name="0x2_math_min_u32"></a>

## Function `min_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u32">min_u32</a>(x: u32, y: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u32">min_u32</a>(x: u32, y: u32): u32 {
    <b>if</b> (x &lt; y) { x }
    <b>else</b> { y }
}
</code></pre>



</details>

<a name="0x2_math_max_u32"></a>

## Function `max_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u32">max_u32</a>(x: u32, y: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u32">max_u32</a>(x: u32, y: u32): u32 {
    <b>if</b> (x &gt; y) { x }
    <b>else</b> { y }
}
</code></pre>



</details>

<a name="0x2_math_diff_u32"></a>

## Function `diff_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u32">diff_u32</a>(x: u32, y: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u32">diff_u32</a>(x: u32, y: u32): u32 {
    <b>if</b> (x &gt; y) { x - y }
    <b>else</b> { y - x }
}
</code></pre>



</details>

<a name="0x2_math_average_u32"></a>

## Function `average_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_average_u32">average_u32</a>(a: u32, b: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_average_u32">average_u32</a>(a: u32, b: u32): u32 {
    ((<a href="math.md#0x2_math_average_u64">average_u64</a>((a <b>as</b> u64), (b <b>as</b> u64))) <b>as</b> u32)
}
</code></pre>



</details>

<a name="0x2_math_clamp_u32"></a>

## Function `clamp_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_clamp_u32">clamp_u32</a>(x: u32, lo: u32, hi: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_clamp_u32">clamp_u32</a>(x: u32, lo: u32, hi: u32): u32 {
    <b>assert</b>!(lo &lt;= hi, <a href="math.md#0x2_math_E_INVALID_ARG">E_INVALID_ARG</a>);
    <b>if</b> (x &lt; lo) { lo }
    <b>else</b> <b>if</b> (x &gt; hi) { hi }
    <b>else</b> { x }
}
</code></pre>



</details>

<a name="0x2_math_bound_u32"></a>

## Function `bound_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bound_u32">bound_u32</a>(x: u32, lo: u32, hi: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bound_u32">bound_u32</a>(x: u32, lo: u32, hi: u32): u32 {
    <a href="math.md#0x2_math_clamp_u32">clamp_u32</a>(x, lo, hi)
}
</code></pre>



</details>

<a name="0x2_math_sqrt_u32"></a>

## Function `sqrt_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u32">sqrt_u32</a>(x: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u32">sqrt_u32</a>(x: u32): u32 {
    ((<a href="math.md#0x2_math_sqrt_u64">sqrt_u64</a>((x <b>as</b> u64))) <b>as</b> u32)
}
</code></pre>



</details>

<a name="0x2_math_log2_u32"></a>

## Function `log2_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_log2_u32">log2_u32</a>(x: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_log2_u32">log2_u32</a>(x: u32): u32 {
    ((<a href="math.md#0x2_math_log2_u64">log2_u64</a>((x <b>as</b> u64))) <b>as</b> u32)
}
</code></pre>



</details>

<a name="0x2_math_is_pow2_u32"></a>

## Function `is_pow2_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_is_pow2_u32">is_pow2_u32</a>(x: u32): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_is_pow2_u32">is_pow2_u32</a>(x: u32): bool {
    x &gt; 0 && (x & (x - 1)) == 0
}
</code></pre>



</details>

<a name="0x2_math_pow2_u32"></a>

## Function `pow2_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow2_u32">pow2_u32</a>(exp: u8): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow2_u32">pow2_u32</a>(exp: u8): u32 {
    <a href="math.md#0x2_math_pow_u32">pow_u32</a>(2, exp)
}
</code></pre>



</details>

<a name="0x2_math_pow_u32"></a>

## Function `pow_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow_u32">pow_u32</a>(base: u32, exponent: u8): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow_u32">pow_u32</a>(base: u32, exponent: u8): u32 {
    <b>let</b> r = <a href="math.md#0x2_math_pow_u64">pow_u64</a>((base <b>as</b> u64), exponent);
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U32">MAX_U32</a> <b>as</b> u64), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u32)
}
</code></pre>



</details>

<a name="0x2_math_try_pow_u32"></a>

## Function `try_pow_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_pow_u32">try_pow_u32</a>(base: u32, exponent: u8): (bool, u32)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_pow_u32">try_pow_u32</a>(base: u32, exponent: u8): (bool, u32) {
    <b>let</b> (ok, r) = <a href="math.md#0x2_math_try_pow_u64">try_pow_u64</a>((base <b>as</b> u64), exponent);
    <b>if</b> (!ok || r &gt; (<a href="math.md#0x2_math_MAX_U32">MAX_U32</a> <b>as</b> u64)) {
        (<b>false</b>, 0)
    } <b>else</b> {
        (<b>true</b>, (r <b>as</b> u32))
    }
}
</code></pre>



</details>

<a name="0x2_math_ceil_div_u32"></a>

## Function `ceil_div_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ceil_div_u32">ceil_div_u32</a>(x: u32, y: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ceil_div_u32">ceil_div_u32</a>(x: u32, y: u32): u32 {
    <b>assert</b>!(y &gt; 0, <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>if</b> (x == 0) { 0 }
    <b>else</b> {
        ((x - 1) / y) + 1
    }
}
</code></pre>



</details>

<a name="0x2_math_mul_div_u32"></a>

## Function `mul_div_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u32">mul_div_u32</a>(x: u32, y: u32, z: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u32">mul_div_u32</a>(x: u32, y: u32, z: u32): u32 {
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128));
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U32">MAX_U32</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u32)
}
</code></pre>



</details>

<a name="0x2_math_mul_div_ceil_u32"></a>

## Function `mul_div_ceil_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_ceil_u32">mul_div_ceil_u32</a>(x: u32, y: u32, z: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_ceil_u32">mul_div_ceil_u32</a>(x: u32, y: u32, z: u32): u32 {
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_round_u128">mul_div_round_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128), <b>true</b>);
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U32">MAX_U32</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u32)
}
</code></pre>



</details>

<a name="0x2_math_try_mul_div_u32"></a>

## Function `try_mul_div_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_mul_div_u32">try_mul_div_u32</a>(x: u32, y: u32, z: u32): (bool, u32)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_mul_div_u32">try_mul_div_u32</a>(x: u32, y: u32, z: u32): (bool, u32) {
    <b>let</b> (ok, r) = <a href="math.md#0x2_math_try_mul_div_u128">try_mul_div_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128));
    <b>if</b> (!ok || r &gt; (<a href="math.md#0x2_math_MAX_U32">MAX_U32</a> <b>as</b> u128)) {
        (<b>false</b>, 0)
    } <b>else</b> {
        (<b>true</b>, (r <b>as</b> u32))
    }
}
</code></pre>



</details>

<a name="0x2_math_mul_add_u32"></a>

## Function `mul_add_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_add_u32">mul_add_u32</a>(x: u32, y: u32, z: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_add_u32">mul_add_u32</a>(x: u32, y: u32, z: u32): u32 {
    <b>let</b> r = <a href="math.md#0x2_math_mul_add_u128">mul_add_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128));
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U32">MAX_U32</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u32)
}
</code></pre>



</details>

<a name="0x2_math_bps_mul_floor_u32"></a>

## Function `bps_mul_floor_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_floor_u32">bps_mul_floor_u32</a>(amount: u32, bps: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_floor_u32">bps_mul_floor_u32</a>(amount: u32, bps: u32): u32 {
    <b>assert</b>!((bps <b>as</b> u128) &lt;= <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>((amount <b>as</b> u128), (bps <b>as</b> u128), <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>);
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U32">MAX_U32</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u32)
}
</code></pre>



</details>

<a name="0x2_math_bps_mul_ceil_u32"></a>

## Function `bps_mul_ceil_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_ceil_u32">bps_mul_ceil_u32</a>(amount: u32, bps: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_ceil_u32">bps_mul_ceil_u32</a>(amount: u32, bps: u32): u32 {
    <b>assert</b>!((bps <b>as</b> u128) &lt;= <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_round_u128">mul_div_round_u128</a>((amount <b>as</b> u128), (bps <b>as</b> u128), <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>, <b>true</b>);
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U32">MAX_U32</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u32)
}
</code></pre>



</details>

<a name="0x2_math_percent_mul_floor_u32"></a>

## Function `percent_mul_floor_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_floor_u32">percent_mul_floor_u32</a>(amount: u32, pct: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_floor_u32">percent_mul_floor_u32</a>(amount: u32, pct: u32): u32 {
    <b>assert</b>!((pct <b>as</b> u128) &lt;= <a href="math.md#0x2_math_PCT_SCALE">PCT_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_u32">mul_div_u32</a>(amount, pct, 100)
}
</code></pre>



</details>

<a name="0x2_math_percent_mul_ceil_u32"></a>

## Function `percent_mul_ceil_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_ceil_u32">percent_mul_ceil_u32</a>(amount: u32, pct: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_ceil_u32">percent_mul_ceil_u32</a>(amount: u32, pct: u32): u32 {
    <b>assert</b>!((pct <b>as</b> u128) &lt;= <a href="math.md#0x2_math_PCT_SCALE">PCT_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_ceil_u32">mul_div_ceil_u32</a>(amount, pct, 100)
}
</code></pre>



</details>

<a name="0x2_math_within_tolerance_u32"></a>

## Function `within_tolerance_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_within_tolerance_u32">within_tolerance_u32</a>(a: u32, b: u32, tolerance: u32): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_within_tolerance_u32">within_tolerance_u32</a>(a: u32, b: u32, tolerance: u32): bool {
    <a href="math.md#0x2_math_diff_u32">diff_u32</a>(a, b) &lt;= tolerance
}
</code></pre>



</details>

<a name="0x2_math_min_u64"></a>

## Function `min_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u64">min_u64</a>(x: u64, y: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u64">min_u64</a>(x: u64, y: u64): u64 {
    <b>if</b> (x &lt; y) { x }
    <b>else</b> { y }
}
</code></pre>



</details>

<a name="0x2_math_max_u64"></a>

## Function `max_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u64">max_u64</a>(x: u64, y: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u64">max_u64</a>(x: u64, y: u64): u64 {
    <b>if</b> (x &gt; y) { x }
    <b>else</b> { y }
}
</code></pre>



</details>

<a name="0x2_math_diff_u64"></a>

## Function `diff_u64`

Absolute difference. Useful for slippage / tolerance checks.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u64">diff_u64</a>(x: u64, y: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u64">diff_u64</a>(x: u64, y: u64): u64 {
    <b>if</b> (x &gt; y) { x - y }
    <b>else</b> { y - x }
}
</code></pre>



</details>

<a name="0x2_math_diff_u128"></a>

## Function `diff_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u128">diff_u128</a>(x: u128, y: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u128">diff_u128</a>(x: u128, y: u128): u128 {
    <b>if</b> (x &gt; y) { x - y }
    <b>else</b> { y - x }
}
</code></pre>



</details>

<a name="0x2_math_diff_u256"></a>

## Function `diff_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u256">diff_u256</a>(x: u256, y: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u256">diff_u256</a>(x: u256, y: u256): u256 {
    <b>if</b> (x &gt; y) { x - y }
    <b>else</b> { y - x }
}
</code></pre>



</details>

<a name="0x2_math_bound_u64"></a>

## Function `bound_u64`

Clamp with a readable abort code for inverted bounds (same as native).


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bound_u64">bound_u64</a>(x: u64, lo: u64, hi: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bound_u64">bound_u64</a>(x: u64, lo: u64, hi: u64): u64 {
    <b>assert</b>!(lo &lt;= hi, <a href="math.md#0x2_math_E_INVALID_ARG">E_INVALID_ARG</a>);
    <a href="math.md#0x2_math_clamp_u64">clamp_u64</a>(x, lo, hi)
}
</code></pre>



</details>

<a name="0x2_math_bound_u128"></a>

## Function `bound_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bound_u128">bound_u128</a>(x: u128, lo: u128, hi: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bound_u128">bound_u128</a>(x: u128, lo: u128, hi: u128): u128 {
    <b>assert</b>!(lo &lt;= hi, <a href="math.md#0x2_math_E_INVALID_ARG">E_INVALID_ARG</a>);
    <a href="math.md#0x2_math_clamp_u128">clamp_u128</a>(x, lo, hi)
}
</code></pre>



</details>

<a name="0x2_math_bound_u256"></a>

## Function `bound_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bound_u256">bound_u256</a>(x: u256, lo: u256, hi: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bound_u256">bound_u256</a>(x: u256, lo: u256, hi: u256): u256 {
    <b>assert</b>!(lo &lt;= hi, <a href="math.md#0x2_math_E_INVALID_ARG">E_INVALID_ARG</a>);
    <a href="math.md#0x2_math_clamp_u256">clamp_u256</a>(x, lo, hi)
}
</code></pre>



</details>

<a name="0x2_math_is_pow2_u64"></a>

## Function `is_pow2_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_is_pow2_u64">is_pow2_u64</a>(x: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_is_pow2_u64">is_pow2_u64</a>(x: u64): bool {
    x &gt; 0 && (x & (x - 1)) == 0
}
</code></pre>



</details>

<a name="0x2_math_is_pow2_u128"></a>

## Function `is_pow2_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_is_pow2_u128">is_pow2_u128</a>(x: u128): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_is_pow2_u128">is_pow2_u128</a>(x: u128): bool {
    x &gt; 0 && (x & (x - 1)) == 0
}
</code></pre>



</details>

<a name="0x2_math_is_pow2_u256"></a>

## Function `is_pow2_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_is_pow2_u256">is_pow2_u256</a>(x: u256): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_is_pow2_u256">is_pow2_u256</a>(x: u256): bool {
    x &gt; 0 && (x & (x - 1)) == 0
}
</code></pre>



</details>

<a name="0x2_math_pow2_u128"></a>

## Function `pow2_u128`

2^exp, aborting on overflow. Prefer over <code><a href="math.md#0x2_math_pow_u128">pow_u128</a>(2, exp)</code> for readability.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow2_u128">pow2_u128</a>(exp: u32): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow2_u128">pow2_u128</a>(exp: u32): u128 {
    <a href="math.md#0x2_math_pow_u128">pow_u128</a>(2, exp)
}
</code></pre>



</details>

<a name="0x2_math_pow2_u64"></a>

## Function `pow2_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow2_u64">pow2_u64</a>(exp: u8): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow2_u64">pow2_u64</a>(exp: u8): u64 {
    <a href="math.md#0x2_math_pow_u64">pow_u64</a>(2, exp)
}
</code></pre>



</details>

<a name="0x2_math_pow2_u256"></a>

## Function `pow2_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow2_u256">pow2_u256</a>(exp: u32): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow2_u256">pow2_u256</a>(exp: u32): u256 {
    <a href="math.md#0x2_math_pow_u256">pow_u256</a>(2, exp)
}
</code></pre>



</details>

<a name="0x2_math_divide_and_round_up"></a>

## Function `divide_and_round_up`

Legacy names kept for backwards compatibility.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_divide_and_round_up">divide_and_round_up</a>(x: u64, y: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_divide_and_round_up">divide_and_round_up</a>(x: u64, y: u64): u64 {
    <a href="math.md#0x2_math_ceil_div_u64">ceil_div_u64</a>(x, y)
}
</code></pre>



</details>

<a name="0x2_math_divide_and_round_up_u128"></a>

## Function `divide_and_round_up_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_divide_and_round_up_u128">divide_and_round_up_u128</a>(x: u128, y: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_divide_and_round_up_u128">divide_and_round_up_u128</a>(x: u128, y: u128): u128 {
    <a href="math.md#0x2_math_ceil_div_u128">ceil_div_u128</a>(x, y)
}
</code></pre>



</details>

<a name="0x2_math_mul_div_u64"></a>

## Function `mul_div_u64`

(x * y) / z for u64, floor. Routes through U256 native so
<code><a href="math.md#0x2_math_MAX_U64">MAX_U64</a> * 2 / 2</code> works. Aborts E_OVERFLOW when result > MAX_U64.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u64">mul_div_u64</a>(x: u64, y: u64, z: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u64">mul_div_u64</a>(x: u64, y: u64, z: u64): u64 {
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128));
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U64">MAX_U64</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u64)
}
</code></pre>



</details>

<a name="0x2_math_mul_div_ceil_u64"></a>

## Function `mul_div_ceil_u64`

(x * y) / z for u64, ceil.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_ceil_u64">mul_div_ceil_u64</a>(x: u64, y: u64, z: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_ceil_u64">mul_div_ceil_u64</a>(x: u64, y: u64, z: u64): u64 {
    <b>let</b> r = <a href="math.md#0x2_math_mul_div_round_u128">mul_div_round_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128), <b>true</b>);
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U64">MAX_U64</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u64)
}
</code></pre>



</details>

<a name="0x2_math_mul_div_ceil_u128"></a>

## Function `mul_div_ceil_u128`

(x * y) / z for u128, ceil.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_ceil_u128">mul_div_ceil_u128</a>(x: u128, y: u128, z: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_ceil_u128">mul_div_ceil_u128</a>(x: u128, y: u128, z: u128): u128 {
    <a href="math.md#0x2_math_mul_div_round_u128">mul_div_round_u128</a>(x, y, z, <b>true</b>)
}
</code></pre>



</details>

<a name="0x2_math_mul_div_ceil_u256"></a>

## Function `mul_div_ceil_u256`

(x * y) / z for u256, ceil.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_ceil_u256">mul_div_ceil_u256</a>(x: u256, y: u256, z: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_ceil_u256">mul_div_ceil_u256</a>(x: u256, y: u256, z: u256): u256 {
    <a href="math.md#0x2_math_mul_div_round_u256">mul_div_round_u256</a>(x, y, z, <b>true</b>)
}
</code></pre>



</details>

<a name="0x2_math_mul_add_u64"></a>

## Function `mul_add_u64`

(x * y) + z for u64. Aborts E_OVERFLOW when result > MAX_U64.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_add_u64">mul_add_u64</a>(x: u64, y: u64, z: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_add_u64">mul_add_u64</a>(x: u64, y: u64, z: u64): u64 {
    <b>let</b> r = <a href="math.md#0x2_math_mul_add_u128">mul_add_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128));
    <b>assert</b>!(r &lt;= (<a href="math.md#0x2_math_MAX_U64">MAX_U64</a> <b>as</b> u128), <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    (r <b>as</b> u64)
}
</code></pre>



</details>

<a name="0x2_math_mul_div_round_up_u64"></a>

## Function `mul_div_round_up_u64`

Legacy ceil helpers kept for backwards compatibility.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_round_up_u64">mul_div_round_up_u64</a>(x: u64, y: u64, z: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_round_up_u64">mul_div_round_up_u64</a>(x: u64, y: u64, z: u64): u64 {
    <a href="math.md#0x2_math_mul_div_ceil_u64">mul_div_ceil_u64</a>(x, y, z)
}
</code></pre>



</details>

<a name="0x2_math_mul_div_round_up_u128"></a>

## Function `mul_div_round_up_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_round_up_u128">mul_div_round_up_u128</a>(x: u128, y: u128, z: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_round_up_u128">mul_div_round_up_u128</a>(x: u128, y: u128, z: u128): u128 {
    <a href="math.md#0x2_math_mul_div_ceil_u128">mul_div_ceil_u128</a>(x, y, z)
}
</code></pre>



</details>

<a name="0x2_math_mul_div_round_up_u256"></a>

## Function `mul_div_round_up_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_round_up_u256">mul_div_round_up_u256</a>(x: u256, y: u256, z: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_round_up_u256">mul_div_round_up_u256</a>(x: u256, y: u256, z: u256): u256 {
    <a href="math.md#0x2_math_mul_div_ceil_u256">mul_div_ceil_u256</a>(x, y, z)
}
</code></pre>



</details>

<a name="0x2_math_try_mul_div_u64"></a>

## Function `try_mul_div_u64`

Non-aborting u64 variant: (false, 0) on div-by-zero or u64 overflow.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_mul_div_u64">try_mul_div_u64</a>(x: u64, y: u64, z: u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_mul_div_u64">try_mul_div_u64</a>(x: u64, y: u64, z: u64): (bool, u64) {
    <b>let</b> (ok, r) = <a href="math.md#0x2_math_try_mul_div_u128">try_mul_div_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128));
    <b>if</b> (!ok || r &gt; (<a href="math.md#0x2_math_MAX_U64">MAX_U64</a> <b>as</b> u128)) {
        (<b>false</b>, 0)
    } <b>else</b> {
        (<b>true</b>, (r <b>as</b> u64))
    }
}
</code></pre>



</details>

<a name="0x2_math_bps_mul_floor_u128"></a>

## Function `bps_mul_floor_u128`

amount * bps / 10_000, floor. <code>bps</code> must be <= 10_000.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_floor_u128">bps_mul_floor_u128</a>(amount: u128, bps: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_floor_u128">bps_mul_floor_u128</a>(amount: u128, bps: u128): u128 {
    <b>assert</b>!(bps &lt;= <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>(amount, bps, <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>)
}
</code></pre>



</details>

<a name="0x2_math_bps_mul_ceil_u128"></a>

## Function `bps_mul_ceil_u128`

amount * bps / 10_000, ceil (favors the fee receiver / pool).


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_ceil_u128">bps_mul_ceil_u128</a>(amount: u128, bps: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_ceil_u128">bps_mul_ceil_u128</a>(amount: u128, bps: u128): u128 {
    <b>assert</b>!(bps &lt;= <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_ceil_u128">mul_div_ceil_u128</a>(amount, bps, <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>)
}
</code></pre>



</details>

<a name="0x2_math_bps_mul_floor_u64"></a>

## Function `bps_mul_floor_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_floor_u64">bps_mul_floor_u64</a>(amount: u64, bps: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_floor_u64">bps_mul_floor_u64</a>(amount: u64, bps: u64): u64 {
    <b>assert</b>!((bps <b>as</b> u128) &lt;= <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_u64">mul_div_u64</a>(amount, bps, <a href="math.md#0x2_math_BPS_SCALE_U64">BPS_SCALE_U64</a>)
}
</code></pre>



</details>

<a name="0x2_math_bps_mul_ceil_u64"></a>

## Function `bps_mul_ceil_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_ceil_u64">bps_mul_ceil_u64</a>(amount: u64, bps: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_ceil_u64">bps_mul_ceil_u64</a>(amount: u64, bps: u64): u64 {
    <b>assert</b>!((bps <b>as</b> u128) &lt;= <a href="math.md#0x2_math_BPS_SCALE">BPS_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_ceil_u64">mul_div_ceil_u64</a>(amount, bps, <a href="math.md#0x2_math_BPS_SCALE_U64">BPS_SCALE_U64</a>)
}
</code></pre>



</details>

<a name="0x2_math_bps_mul_floor_u256"></a>

## Function `bps_mul_floor_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_floor_u256">bps_mul_floor_u256</a>(amount: u256, bps: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_floor_u256">bps_mul_floor_u256</a>(amount: u256, bps: u256): u256 {
    <b>assert</b>!(bps &lt;= <a href="math.md#0x2_math_BPS_SCALE_U256">BPS_SCALE_U256</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_u256">mul_div_u256</a>(amount, bps, <a href="math.md#0x2_math_BPS_SCALE_U256">BPS_SCALE_U256</a>)
}
</code></pre>



</details>

<a name="0x2_math_bps_mul_ceil_u256"></a>

## Function `bps_mul_ceil_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_ceil_u256">bps_mul_ceil_u256</a>(amount: u256, bps: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bps_mul_ceil_u256">bps_mul_ceil_u256</a>(amount: u256, bps: u256): u256 {
    <b>assert</b>!(bps &lt;= <a href="math.md#0x2_math_BPS_SCALE_U256">BPS_SCALE_U256</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_ceil_u256">mul_div_ceil_u256</a>(amount, bps, <a href="math.md#0x2_math_BPS_SCALE_U256">BPS_SCALE_U256</a>)
}
</code></pre>



</details>

<a name="0x2_math_add_bps_u128"></a>

## Function `add_bps_u128`

amount + fee where fee = amount * bps / 10_000 (floor).


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_add_bps_u128">add_bps_u128</a>(amount: u128, bps: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_add_bps_u128">add_bps_u128</a>(amount: u128, bps: u128): u128 {
    amount + <a href="math.md#0x2_math_bps_mul_floor_u128">bps_mul_floor_u128</a>(amount, bps)
}
</code></pre>



</details>

<a name="0x2_math_sub_bps_u128"></a>

## Function `sub_bps_u128`

amount - fee where fee = amount * bps / 10_000 (ceil, favors pool).
Aborts E_OVERFLOW if fee > amount (cannot happen for bps <= 10_000,
but kept as a safety net).


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sub_bps_u128">sub_bps_u128</a>(amount: u128, bps: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sub_bps_u128">sub_bps_u128</a>(amount: u128, bps: u128): u128 {
    <b>let</b> fee = <a href="math.md#0x2_math_bps_mul_ceil_u128">bps_mul_ceil_u128</a>(amount, bps);
    <b>assert</b>!(fee &lt;= amount, <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    amount - fee
}
</code></pre>



</details>

<a name="0x2_math_add_bps_u256"></a>

## Function `add_bps_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_add_bps_u256">add_bps_u256</a>(amount: u256, bps: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_add_bps_u256">add_bps_u256</a>(amount: u256, bps: u256): u256 {
    amount + <a href="math.md#0x2_math_bps_mul_floor_u256">bps_mul_floor_u256</a>(amount, bps)
}
</code></pre>



</details>

<a name="0x2_math_sub_bps_u256"></a>

## Function `sub_bps_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sub_bps_u256">sub_bps_u256</a>(amount: u256, bps: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sub_bps_u256">sub_bps_u256</a>(amount: u256, bps: u256): u256 {
    <b>let</b> fee = <a href="math.md#0x2_math_bps_mul_ceil_u256">bps_mul_ceil_u256</a>(amount, bps);
    <b>assert</b>!(fee &lt;= amount, <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>);
    amount - fee
}
</code></pre>



</details>

<a name="0x2_math_percent_mul_floor_u128"></a>

## Function `percent_mul_floor_u128`

amount * pct / 100, floor. <code>pct</code> must be <= 100.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_floor_u128">percent_mul_floor_u128</a>(amount: u128, pct: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_floor_u128">percent_mul_floor_u128</a>(amount: u128, pct: u128): u128 {
    <b>assert</b>!(pct &lt;= <a href="math.md#0x2_math_PCT_SCALE">PCT_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>(amount, pct, <a href="math.md#0x2_math_PCT_SCALE">PCT_SCALE</a>)
}
</code></pre>



</details>

<a name="0x2_math_percent_mul_ceil_u128"></a>

## Function `percent_mul_ceil_u128`

amount * pct / 100, ceil.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_ceil_u128">percent_mul_ceil_u128</a>(amount: u128, pct: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_ceil_u128">percent_mul_ceil_u128</a>(amount: u128, pct: u128): u128 {
    <b>assert</b>!(pct &lt;= <a href="math.md#0x2_math_PCT_SCALE">PCT_SCALE</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_ceil_u128">mul_div_ceil_u128</a>(amount, pct, <a href="math.md#0x2_math_PCT_SCALE">PCT_SCALE</a>)
}
</code></pre>



</details>

<a name="0x2_math_percent_mul_floor_u256"></a>

## Function `percent_mul_floor_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_floor_u256">percent_mul_floor_u256</a>(amount: u256, pct: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_floor_u256">percent_mul_floor_u256</a>(amount: u256, pct: u256): u256 {
    <b>assert</b>!(pct &lt;= <a href="math.md#0x2_math_PCT_SCALE_U256">PCT_SCALE_U256</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_u256">mul_div_u256</a>(amount, pct, <a href="math.md#0x2_math_PCT_SCALE_U256">PCT_SCALE_U256</a>)
}
</code></pre>



</details>

<a name="0x2_math_percent_mul_ceil_u256"></a>

## Function `percent_mul_ceil_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_ceil_u256">percent_mul_ceil_u256</a>(amount: u256, pct: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_percent_mul_ceil_u256">percent_mul_ceil_u256</a>(amount: u256, pct: u256): u256 {
    <b>assert</b>!(pct &lt;= <a href="math.md#0x2_math_PCT_SCALE_U256">PCT_SCALE_U256</a>, <a href="math.md#0x2_math_E_OUT_OF_RANGE">E_OUT_OF_RANGE</a>);
    <a href="math.md#0x2_math_mul_div_ceil_u256">mul_div_ceil_u256</a>(amount, pct, <a href="math.md#0x2_math_PCT_SCALE_U256">PCT_SCALE_U256</a>)
}
</code></pre>



</details>

<a name="0x2_math_wad_mul"></a>

## Function `wad_mul`

wad_mul(a, b) = a * b / 1e18, floor. Inputs and output are WAD-scaled.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_mul">wad_mul</a>(a: u128, b: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_mul">wad_mul</a>(a: u128, b: u128): u128 {
    <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>(a, b, <a href="math.md#0x2_math_WAD">WAD</a>)
}
</code></pre>



</details>

<a name="0x2_math_wad_div"></a>

## Function `wad_div`

wad_div(a, b) = a * 1e18 / b, floor.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_div">wad_div</a>(a: u128, b: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_div">wad_div</a>(a: u128, b: u128): u128 {
    <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>(a, <a href="math.md#0x2_math_WAD">WAD</a>, b)
}
</code></pre>



</details>

<a name="0x2_math_wad_mul_ceil"></a>

## Function `wad_mul_ceil`

wad_mul with ceil rounding (favors pool / lender).


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_mul_ceil">wad_mul_ceil</a>(a: u128, b: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_mul_ceil">wad_mul_ceil</a>(a: u128, b: u128): u128 {
    <a href="math.md#0x2_math_mul_div_ceil_u128">mul_div_ceil_u128</a>(a, b, <a href="math.md#0x2_math_WAD">WAD</a>)
}
</code></pre>



</details>

<a name="0x2_math_wad_div_ceil"></a>

## Function `wad_div_ceil`

wad_div with ceil rounding.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_div_ceil">wad_div_ceil</a>(a: u128, b: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_div_ceil">wad_div_ceil</a>(a: u128, b: u128): u128 {
    <a href="math.md#0x2_math_mul_div_ceil_u128">mul_div_ceil_u128</a>(a, <a href="math.md#0x2_math_WAD">WAD</a>, b)
}
</code></pre>



</details>

<a name="0x2_math_ray_mul"></a>

## Function `ray_mul`

ray_mul(a, b) = a * b / 1e27, floor.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ray_mul">ray_mul</a>(a: u128, b: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ray_mul">ray_mul</a>(a: u128, b: u128): u128 {
    <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>(a, b, <a href="math.md#0x2_math_RAY">RAY</a>)
}
</code></pre>



</details>

<a name="0x2_math_ray_div"></a>

## Function `ray_div`

ray_div(a, b) = a * 1e27 / b, floor.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ray_div">ray_div</a>(a: u128, b: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ray_div">ray_div</a>(a: u128, b: u128): u128 {
    <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>(a, <a href="math.md#0x2_math_RAY">RAY</a>, b)
}
</code></pre>



</details>

<a name="0x2_math_wad_mul_u256"></a>

## Function `wad_mul_u256`

WAD helpers for u256 (same 1e18 scale, wider range).


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_mul_u256">wad_mul_u256</a>(a: u256, b: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_mul_u256">wad_mul_u256</a>(a: u256, b: u256): u256 {
    <a href="math.md#0x2_math_mul_div_u256">mul_div_u256</a>(a, b, <a href="math.md#0x2_math_WAD_U256">WAD_U256</a>)
}
</code></pre>



</details>

<a name="0x2_math_wad_div_u256"></a>

## Function `wad_div_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_div_u256">wad_div_u256</a>(a: u256, b: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_div_u256">wad_div_u256</a>(a: u256, b: u256): u256 {
    <a href="math.md#0x2_math_mul_div_u256">mul_div_u256</a>(a, <a href="math.md#0x2_math_WAD_U256">WAD_U256</a>, b)
}
</code></pre>



</details>

<a name="0x2_math_wad_mul_ceil_u256"></a>

## Function `wad_mul_ceil_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_mul_ceil_u256">wad_mul_ceil_u256</a>(a: u256, b: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_mul_ceil_u256">wad_mul_ceil_u256</a>(a: u256, b: u256): u256 {
    <a href="math.md#0x2_math_mul_div_ceil_u256">mul_div_ceil_u256</a>(a, b, <a href="math.md#0x2_math_WAD_U256">WAD_U256</a>)
}
</code></pre>



</details>

<a name="0x2_math_wad_div_ceil_u256"></a>

## Function `wad_div_ceil_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_div_ceil_u256">wad_div_ceil_u256</a>(a: u256, b: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_wad_div_ceil_u256">wad_div_ceil_u256</a>(a: u256, b: u256): u256 {
    <a href="math.md#0x2_math_mul_div_ceil_u256">mul_div_ceil_u256</a>(a, <a href="math.md#0x2_math_WAD_U256">WAD_U256</a>, b)
}
</code></pre>



</details>

<a name="0x2_math_ray_mul_u256"></a>

## Function `ray_mul_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ray_mul_u256">ray_mul_u256</a>(a: u256, b: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ray_mul_u256">ray_mul_u256</a>(a: u256, b: u256): u256 {
    <a href="math.md#0x2_math_mul_div_u256">mul_div_u256</a>(a, b, <a href="math.md#0x2_math_RAY_U256">RAY_U256</a>)
}
</code></pre>



</details>

<a name="0x2_math_ray_div_u256"></a>

## Function `ray_div_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ray_div_u256">ray_div_u256</a>(a: u256, b: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_ray_div_u256">ray_div_u256</a>(a: u256, b: u256): u256 {
    <a href="math.md#0x2_math_mul_div_u256">mul_div_u256</a>(a, <a href="math.md#0x2_math_RAY_U256">RAY_U256</a>, b)
}
</code></pre>



</details>

<a name="0x2_math_quote_amount_out"></a>

## Function `quote_amount_out`

Constant-product quote: amount_out = amount_in * reserve_out / (reserve_in + amount_in).
Floor rounding (favors pool). Aborts on zero reserves.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_quote_amount_out">quote_amount_out</a>(amount_in: u128, reserve_in: u128, reserve_out: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_quote_amount_out">quote_amount_out</a>(
    amount_in: u128, reserve_in: u128, reserve_out: u128
): u128 {
    <b>assert</b>!(reserve_in &gt; 0 && reserve_out &gt; 0, <a href="math.md#0x2_math_E_INVALID_ARG">E_INVALID_ARG</a>);
    <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>(amount_in, reserve_out, reserve_in + amount_in)
}
</code></pre>



</details>

<a name="0x2_math_quote_amount_out_u256"></a>

## Function `quote_amount_out_u256`

Same quote for u256 reserves (deep-liquidity pools).


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_quote_amount_out_u256">quote_amount_out_u256</a>(amount_in: u256, reserve_in: u256, reserve_out: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_quote_amount_out_u256">quote_amount_out_u256</a>(
    amount_in: u256, reserve_in: u256, reserve_out: u256
): u256 {
    <b>assert</b>!(reserve_in &gt; 0 && reserve_out &gt; 0, <a href="math.md#0x2_math_E_INVALID_ARG">E_INVALID_ARG</a>);
    <a href="math.md#0x2_math_mul_div_u256">mul_div_u256</a>(amount_in, reserve_out, reserve_in + amount_in)
}
</code></pre>



</details>

<a name="0x2_math_apply_price"></a>

## Function `apply_price`

Average price as (numerator, denominator) scaled helper:
returns amount * price_num / price_den with floor rounding.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_apply_price">apply_price</a>(amount: u128, price_num: u128, price_den: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_apply_price">apply_price</a>(amount: u128, price_num: u128, price_den: u128): u128 {
    <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>(amount, price_num, price_den)
}
</code></pre>



</details>

<a name="0x2_math_apply_price_u256"></a>

## Function `apply_price_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_apply_price_u256">apply_price_u256</a>(amount: u256, price_num: u256, price_den: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_apply_price_u256">apply_price_u256</a>(
    amount: u256, price_num: u256, price_den: u256
): u256 {
    <a href="math.md#0x2_math_mul_div_u256">mul_div_u256</a>(amount, price_num, price_den)
}
</code></pre>



</details>

<a name="0x2_math_within_tolerance_u128"></a>

## Function `within_tolerance_u128`

Check <code>|a - b| &lt;= tolerance</code> without overflow.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_within_tolerance_u128">within_tolerance_u128</a>(a: u128, b: u128, tolerance: u128): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_within_tolerance_u128">within_tolerance_u128</a>(a: u128, b: u128, tolerance: u128): bool {
    <a href="math.md#0x2_math_diff_u128">diff_u128</a>(a, b) &lt;= tolerance
}
</code></pre>



</details>

<a name="0x2_math_within_tolerance_u64"></a>

## Function `within_tolerance_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_within_tolerance_u64">within_tolerance_u64</a>(a: u64, b: u64, tolerance: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_within_tolerance_u64">within_tolerance_u64</a>(a: u64, b: u64, tolerance: u64): bool {
    <a href="math.md#0x2_math_diff_u64">diff_u64</a>(a, b) &lt;= tolerance
}
</code></pre>



</details>

<a name="0x2_math_within_tolerance_u256"></a>

## Function `within_tolerance_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_within_tolerance_u256">within_tolerance_u256</a>(a: u256, b: u256, tolerance: u256): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_within_tolerance_u256">within_tolerance_u256</a>(a: u256, b: u256, tolerance: u256): bool {
    <a href="math.md#0x2_math_diff_u256">diff_u256</a>(a, b) &lt;= tolerance
}
</code></pre>



</details>

<a name="0x2_math_assert_min_out"></a>

## Function `assert_min_out`

Slippage check used by DEX front-ends:
aborts E_INVALID_ARG when actual < min_amount_out.


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_assert_min_out">assert_min_out</a>(actual_out: u128, min_amount_out: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_assert_min_out">assert_min_out</a>(actual_out: u128, min_amount_out: u128) {
    <b>assert</b>!(actual_out &gt;= min_amount_out, <a href="math.md#0x2_math_E_INVALID_ARG">E_INVALID_ARG</a>);
}
</code></pre>



</details>

<a name="0x2_math_assert_min_out_u256"></a>

## Function `assert_min_out_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_assert_min_out_u256">assert_min_out_u256</a>(actual_out: u256, min_amount_out: u256)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_assert_min_out_u256">assert_min_out_u256</a>(actual_out: u256, min_amount_out: u256) {
    <b>assert</b>!(actual_out &gt;= min_amount_out, <a href="math.md#0x2_math_E_INVALID_ARG">E_INVALID_ARG</a>);
}
</code></pre>



</details>

<a name="0x2_math_checked_add_u8"></a>

## Function `checked_add_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_add_u8">checked_add_u8</a>(x: u8, y: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_add_u8">checked_add_u8</a>(x: u8, y: u8): Option&lt;u8&gt; {
    <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U8">MAX_U8</a> - y) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x + y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_sub_u8"></a>

## Function `checked_sub_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_sub_u8">checked_sub_u8</a>(x: u8, y: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_sub_u8">checked_sub_u8</a>(x: u8, y: u8): Option&lt;u8&gt; {
    <b>if</b> (y &gt; x) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x - y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_mul_u8"></a>

## Function `checked_mul_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_mul_u8">checked_mul_u8</a>(x: u8, y: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_mul_u8">checked_mul_u8</a>(x: u8, y: u8): Option&lt;u8&gt; {
    <b>if</b> (x == 0 || y == 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(0)
    } <b>else</b> <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U8">MAX_U8</a> / y) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x * y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_div_u8"></a>

## Function `checked_div_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_div_u8">checked_div_u8</a>(x: u8, y: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_div_u8">checked_div_u8</a>(x: u8, y: u8): Option&lt;u8&gt; {
    <b>if</b> (y == 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x / y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_pow_u8"></a>

## Function `checked_pow_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_pow_u8">checked_pow_u8</a>(base: u8, exponent: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_pow_u8">checked_pow_u8</a>(base: u8, exponent: u8): Option&lt;u8&gt; {
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_pow_u8">try_pow_u8</a>(base, exponent);
    <b>if</b> (ok) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(v)
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_add_u16"></a>

## Function `checked_add_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_add_u16">checked_add_u16</a>(x: u16, y: u16): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_add_u16">checked_add_u16</a>(x: u16, y: u16): Option&lt;u16&gt; {
    <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U16">MAX_U16</a> - y) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x + y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_sub_u16"></a>

## Function `checked_sub_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_sub_u16">checked_sub_u16</a>(x: u16, y: u16): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_sub_u16">checked_sub_u16</a>(x: u16, y: u16): Option&lt;u16&gt; {
    <b>if</b> (y &gt; x) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x - y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_mul_u16"></a>

## Function `checked_mul_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_mul_u16">checked_mul_u16</a>(x: u16, y: u16): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_mul_u16">checked_mul_u16</a>(x: u16, y: u16): Option&lt;u16&gt; {
    <b>if</b> (x == 0 || y == 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(0)
    } <b>else</b> <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U16">MAX_U16</a> / y) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x * y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_div_u16"></a>

## Function `checked_div_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_div_u16">checked_div_u16</a>(x: u16, y: u16): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_div_u16">checked_div_u16</a>(x: u16, y: u16): Option&lt;u16&gt; {
    <b>if</b> (y == 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x / y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_pow_u16"></a>

## Function `checked_pow_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_pow_u16">checked_pow_u16</a>(base: u16, exponent: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_pow_u16">checked_pow_u16</a>(base: u16, exponent: u8): Option&lt;u16&gt; {
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_pow_u16">try_pow_u16</a>(base, exponent);
    <b>if</b> (ok) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(v)
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_add_u32"></a>

## Function `checked_add_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_add_u32">checked_add_u32</a>(x: u32, y: u32): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_add_u32">checked_add_u32</a>(x: u32, y: u32): Option&lt;u32&gt; {
    <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U32">MAX_U32</a> - y) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x + y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_sub_u32"></a>

## Function `checked_sub_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_sub_u32">checked_sub_u32</a>(x: u32, y: u32): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_sub_u32">checked_sub_u32</a>(x: u32, y: u32): Option&lt;u32&gt; {
    <b>if</b> (y &gt; x) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x - y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_mul_u32"></a>

## Function `checked_mul_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_mul_u32">checked_mul_u32</a>(x: u32, y: u32): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_mul_u32">checked_mul_u32</a>(x: u32, y: u32): Option&lt;u32&gt; {
    <b>if</b> (x == 0 || y == 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(0)
    } <b>else</b> <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U32">MAX_U32</a> / y) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x * y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_div_u32"></a>

## Function `checked_div_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_div_u32">checked_div_u32</a>(x: u32, y: u32): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_div_u32">checked_div_u32</a>(x: u32, y: u32): Option&lt;u32&gt; {
    <b>if</b> (y == 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x / y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_pow_u32"></a>

## Function `checked_pow_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_pow_u32">checked_pow_u32</a>(base: u32, exponent: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_pow_u32">checked_pow_u32</a>(base: u32, exponent: u8): Option&lt;u32&gt; {
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_pow_u32">try_pow_u32</a>(base, exponent);
    <b>if</b> (ok) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(v)
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_add_u64"></a>

## Function `checked_add_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_add_u64">checked_add_u64</a>(x: u64, y: u64): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_add_u64">checked_add_u64</a>(x: u64, y: u64): Option&lt;u64&gt; {
    <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U64">MAX_U64</a> - y) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x + y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_sub_u64"></a>

## Function `checked_sub_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_sub_u64">checked_sub_u64</a>(x: u64, y: u64): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_sub_u64">checked_sub_u64</a>(x: u64, y: u64): Option&lt;u64&gt; {
    <b>if</b> (y &gt; x) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x - y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_mul_u64"></a>

## Function `checked_mul_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_mul_u64">checked_mul_u64</a>(x: u64, y: u64): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_mul_u64">checked_mul_u64</a>(x: u64, y: u64): Option&lt;u64&gt; {
    <b>if</b> (x == 0 || y == 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(0)
    } <b>else</b> <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U64">MAX_U64</a> / y) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x * y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_div_u64"></a>

## Function `checked_div_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_div_u64">checked_div_u64</a>(x: u64, y: u64): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_div_u64">checked_div_u64</a>(x: u64, y: u64): Option&lt;u64&gt; {
    <b>if</b> (y == 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x / y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_pow_u64"></a>

## Function `checked_pow_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_pow_u64">checked_pow_u64</a>(base: u64, exponent: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_pow_u64">checked_pow_u64</a>(base: u64, exponent: u8): Option&lt;u64&gt; {
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_pow_u64">try_pow_u64</a>(base, exponent);
    <b>if</b> (ok) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(v)
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_add_u128"></a>

## Function `checked_add_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_add_u128">checked_add_u128</a>(x: u128, y: u128): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_add_u128">checked_add_u128</a>(x: u128, y: u128): Option&lt;u128&gt; {
    <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U128">MAX_U128</a> - y) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x + y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_sub_u128"></a>

## Function `checked_sub_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_sub_u128">checked_sub_u128</a>(x: u128, y: u128): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_sub_u128">checked_sub_u128</a>(x: u128, y: u128): Option&lt;u128&gt; {
    <b>if</b> (y &gt; x) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x - y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_mul_u128"></a>

## Function `checked_mul_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_mul_u128">checked_mul_u128</a>(x: u128, y: u128): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_mul_u128">checked_mul_u128</a>(x: u128, y: u128): Option&lt;u128&gt; {
    <b>if</b> (x == 0 || y == 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(0)
    } <b>else</b> <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U128">MAX_U128</a> / y) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x * y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_div_u128"></a>

## Function `checked_div_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_div_u128">checked_div_u128</a>(x: u128, y: u128): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_div_u128">checked_div_u128</a>(x: u128, y: u128): Option&lt;u128&gt; {
    <b>if</b> (y == 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x / y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_pow_u128"></a>

## Function `checked_pow_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_pow_u128">checked_pow_u128</a>(base: u128, exponent: u32): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_pow_u128">checked_pow_u128</a>(base: u128, exponent: u32): Option&lt;u128&gt; {
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_pow_u128">try_pow_u128</a>(base, exponent);
    <b>if</b> (ok) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(v)
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_add_u256"></a>

## Function `checked_add_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_add_u256">checked_add_u256</a>(x: u256, y: u256): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_add_u256">checked_add_u256</a>(x: u256, y: u256): Option&lt;u256&gt; {
    <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U256">MAX_U256</a> - y) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x + y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_sub_u256"></a>

## Function `checked_sub_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_sub_u256">checked_sub_u256</a>(x: u256, y: u256): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_sub_u256">checked_sub_u256</a>(x: u256, y: u256): Option&lt;u256&gt; {
    <b>if</b> (y &gt; x) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x - y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_mul_u256"></a>

## Function `checked_mul_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_mul_u256">checked_mul_u256</a>(x: u256, y: u256): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_mul_u256">checked_mul_u256</a>(x: u256, y: u256): Option&lt;u256&gt; {
    <b>if</b> (x == 0 || y == 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(0)
    } <b>else</b> <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U256">MAX_U256</a> / y) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x * y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_div_u256"></a>

## Function `checked_div_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_div_u256">checked_div_u256</a>(x: u256, y: u256): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_div_u256">checked_div_u256</a>(x: u256, y: u256): Option&lt;u256&gt; {
    <b>if</b> (y == 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x / y)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_pow_u256"></a>

## Function `checked_pow_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_pow_u256">checked_pow_u256</a>(base: u256, exponent: u32): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_pow_u256">checked_pow_u256</a>(base: u256, exponent: u32): Option&lt;u256&gt; {
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_pow_u256">try_pow_u256</a>(base, exponent);
    <b>if</b> (ok) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(v)
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a name="0x2_math_saturating_add_u8"></a>

## Function `saturating_add_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_add_u8">saturating_add_u8</a>(x: u8, y: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_add_u8">saturating_add_u8</a>(x: u8, y: u8): u8 {
    <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U8">MAX_U8</a> - y) { <a href="math.md#0x2_math_MAX_U8">MAX_U8</a> }
    <b>else</b> { x + y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_sub_u8"></a>

## Function `saturating_sub_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_sub_u8">saturating_sub_u8</a>(x: u8, y: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_sub_u8">saturating_sub_u8</a>(x: u8, y: u8): u8 {
    <b>if</b> (y &gt; x) { 0 }
    <b>else</b> { x - y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_mul_u8"></a>

## Function `saturating_mul_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_u8">saturating_mul_u8</a>(x: u8, y: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_u8">saturating_mul_u8</a>(x: u8, y: u8): u8 {
    <b>if</b> (x == 0 || y == 0) { 0 }
    <b>else</b> <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U8">MAX_U8</a> / y) { <a href="math.md#0x2_math_MAX_U8">MAX_U8</a> }
    <b>else</b> { x * y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_pow_u8"></a>

## Function `saturating_pow_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_pow_u8">saturating_pow_u8</a>(base: u8, exponent: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_pow_u8">saturating_pow_u8</a>(base: u8, exponent: u8): u8 {
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_pow_u8">try_pow_u8</a>(base, exponent);
    <b>if</b> (ok) { v }
    <b>else</b> { <a href="math.md#0x2_math_MAX_U8">MAX_U8</a> }
}
</code></pre>



</details>

<a name="0x2_math_saturating_mul_div_u8"></a>

## Function `saturating_mul_div_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_div_u8">saturating_mul_div_u8</a>(x: u8, y: u8, z: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_div_u8">saturating_mul_div_u8</a>(x: u8, y: u8, z: u8): u8 {
    <b>assert</b>!(z != 0, <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_mul_div_u8">try_mul_div_u8</a>(x, y, z);
    <b>if</b> (ok) { v }
    <b>else</b> { <a href="math.md#0x2_math_MAX_U8">MAX_U8</a> }
}
</code></pre>



</details>

<a name="0x2_math_saturating_add_u16"></a>

## Function `saturating_add_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_add_u16">saturating_add_u16</a>(x: u16, y: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_add_u16">saturating_add_u16</a>(x: u16, y: u16): u16 {
    <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U16">MAX_U16</a> - y) { <a href="math.md#0x2_math_MAX_U16">MAX_U16</a> }
    <b>else</b> { x + y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_sub_u16"></a>

## Function `saturating_sub_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_sub_u16">saturating_sub_u16</a>(x: u16, y: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_sub_u16">saturating_sub_u16</a>(x: u16, y: u16): u16 {
    <b>if</b> (y &gt; x) { 0 }
    <b>else</b> { x - y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_mul_u16"></a>

## Function `saturating_mul_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_u16">saturating_mul_u16</a>(x: u16, y: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_u16">saturating_mul_u16</a>(x: u16, y: u16): u16 {
    <b>if</b> (x == 0 || y == 0) { 0 }
    <b>else</b> <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U16">MAX_U16</a> / y) { <a href="math.md#0x2_math_MAX_U16">MAX_U16</a> }
    <b>else</b> { x * y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_pow_u16"></a>

## Function `saturating_pow_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_pow_u16">saturating_pow_u16</a>(base: u16, exponent: u8): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_pow_u16">saturating_pow_u16</a>(base: u16, exponent: u8): u16 {
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_pow_u16">try_pow_u16</a>(base, exponent);
    <b>if</b> (ok) { v }
    <b>else</b> { <a href="math.md#0x2_math_MAX_U16">MAX_U16</a> }
}
</code></pre>



</details>

<a name="0x2_math_saturating_mul_div_u16"></a>

## Function `saturating_mul_div_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_div_u16">saturating_mul_div_u16</a>(x: u16, y: u16, z: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_div_u16">saturating_mul_div_u16</a>(x: u16, y: u16, z: u16): u16 {
    <b>assert</b>!(z != 0, <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_mul_div_u16">try_mul_div_u16</a>(x, y, z);
    <b>if</b> (ok) { v }
    <b>else</b> { <a href="math.md#0x2_math_MAX_U16">MAX_U16</a> }
}
</code></pre>



</details>

<a name="0x2_math_saturating_add_u32"></a>

## Function `saturating_add_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_add_u32">saturating_add_u32</a>(x: u32, y: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_add_u32">saturating_add_u32</a>(x: u32, y: u32): u32 {
    <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U32">MAX_U32</a> - y) { <a href="math.md#0x2_math_MAX_U32">MAX_U32</a> }
    <b>else</b> { x + y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_sub_u32"></a>

## Function `saturating_sub_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_sub_u32">saturating_sub_u32</a>(x: u32, y: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_sub_u32">saturating_sub_u32</a>(x: u32, y: u32): u32 {
    <b>if</b> (y &gt; x) { 0 }
    <b>else</b> { x - y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_mul_u32"></a>

## Function `saturating_mul_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_u32">saturating_mul_u32</a>(x: u32, y: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_u32">saturating_mul_u32</a>(x: u32, y: u32): u32 {
    <b>if</b> (x == 0 || y == 0) { 0 }
    <b>else</b> <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U32">MAX_U32</a> / y) { <a href="math.md#0x2_math_MAX_U32">MAX_U32</a> }
    <b>else</b> { x * y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_pow_u32"></a>

## Function `saturating_pow_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_pow_u32">saturating_pow_u32</a>(base: u32, exponent: u8): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_pow_u32">saturating_pow_u32</a>(base: u32, exponent: u8): u32 {
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_pow_u32">try_pow_u32</a>(base, exponent);
    <b>if</b> (ok) { v }
    <b>else</b> { <a href="math.md#0x2_math_MAX_U32">MAX_U32</a> }
}
</code></pre>



</details>

<a name="0x2_math_saturating_mul_div_u32"></a>

## Function `saturating_mul_div_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_div_u32">saturating_mul_div_u32</a>(x: u32, y: u32, z: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_div_u32">saturating_mul_div_u32</a>(x: u32, y: u32, z: u32): u32 {
    <b>assert</b>!(z != 0, <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_mul_div_u32">try_mul_div_u32</a>(x, y, z);
    <b>if</b> (ok) { v }
    <b>else</b> { <a href="math.md#0x2_math_MAX_U32">MAX_U32</a> }
}
</code></pre>



</details>

<a name="0x2_math_saturating_add_u64"></a>

## Function `saturating_add_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_add_u64">saturating_add_u64</a>(x: u64, y: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_add_u64">saturating_add_u64</a>(x: u64, y: u64): u64 {
    <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U64">MAX_U64</a> - y) { <a href="math.md#0x2_math_MAX_U64">MAX_U64</a> }
    <b>else</b> { x + y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_sub_u64"></a>

## Function `saturating_sub_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_sub_u64">saturating_sub_u64</a>(x: u64, y: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_sub_u64">saturating_sub_u64</a>(x: u64, y: u64): u64 {
    <b>if</b> (y &gt; x) { 0 }
    <b>else</b> { x - y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_mul_u64"></a>

## Function `saturating_mul_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_u64">saturating_mul_u64</a>(x: u64, y: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_u64">saturating_mul_u64</a>(x: u64, y: u64): u64 {
    <b>if</b> (x == 0 || y == 0) { 0 }
    <b>else</b> <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U64">MAX_U64</a> / y) { <a href="math.md#0x2_math_MAX_U64">MAX_U64</a> }
    <b>else</b> { x * y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_pow_u64"></a>

## Function `saturating_pow_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_pow_u64">saturating_pow_u64</a>(base: u64, exponent: u8): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_pow_u64">saturating_pow_u64</a>(base: u64, exponent: u8): u64 {
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_pow_u64">try_pow_u64</a>(base, exponent);
    <b>if</b> (ok) { v }
    <b>else</b> { <a href="math.md#0x2_math_MAX_U64">MAX_U64</a> }
}
</code></pre>



</details>

<a name="0x2_math_saturating_mul_div_u64"></a>

## Function `saturating_mul_div_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_div_u64">saturating_mul_div_u64</a>(x: u64, y: u64, z: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_div_u64">saturating_mul_div_u64</a>(x: u64, y: u64, z: u64): u64 {
    <b>assert</b>!(z != 0, <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_mul_div_u64">try_mul_div_u64</a>(x, y, z);
    <b>if</b> (ok) { v }
    <b>else</b> { <a href="math.md#0x2_math_MAX_U64">MAX_U64</a> }
}
</code></pre>



</details>

<a name="0x2_math_saturating_add_u128"></a>

## Function `saturating_add_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_add_u128">saturating_add_u128</a>(x: u128, y: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_add_u128">saturating_add_u128</a>(x: u128, y: u128): u128 {
    <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U128">MAX_U128</a> - y) {
        <a href="math.md#0x2_math_MAX_U128">MAX_U128</a>
    } <b>else</b> { x + y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_sub_u128"></a>

## Function `saturating_sub_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_sub_u128">saturating_sub_u128</a>(x: u128, y: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_sub_u128">saturating_sub_u128</a>(x: u128, y: u128): u128 {
    <b>if</b> (y &gt; x) { 0 }
    <b>else</b> { x - y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_mul_u128"></a>

## Function `saturating_mul_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_u128">saturating_mul_u128</a>(x: u128, y: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_u128">saturating_mul_u128</a>(x: u128, y: u128): u128 {
    <b>if</b> (x == 0 || y == 0) { 0 }
    <b>else</b> <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U128">MAX_U128</a> / y) {
        <a href="math.md#0x2_math_MAX_U128">MAX_U128</a>
    } <b>else</b> { x * y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_pow_u128"></a>

## Function `saturating_pow_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_pow_u128">saturating_pow_u128</a>(base: u128, exponent: u32): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_pow_u128">saturating_pow_u128</a>(base: u128, exponent: u32): u128 {
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_pow_u128">try_pow_u128</a>(base, exponent);
    <b>if</b> (ok) { v }
    <b>else</b> {
        <a href="math.md#0x2_math_MAX_U128">MAX_U128</a>
    }
}
</code></pre>



</details>

<a name="0x2_math_saturating_mul_div_u128"></a>

## Function `saturating_mul_div_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_div_u128">saturating_mul_div_u128</a>(x: u128, y: u128, z: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_div_u128">saturating_mul_div_u128</a>(x: u128, y: u128, z: u128): u128 {
    <b>assert</b>!(z != 0, <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_mul_div_u128">try_mul_div_u128</a>(x, y, z);
    <b>if</b> (ok) { v }
    <b>else</b> {
        <a href="math.md#0x2_math_MAX_U128">MAX_U128</a>
    }
}
</code></pre>



</details>

<a name="0x2_math_saturating_add_u256"></a>

## Function `saturating_add_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_add_u256">saturating_add_u256</a>(x: u256, y: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_add_u256">saturating_add_u256</a>(x: u256, y: u256): u256 {
    <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U256">MAX_U256</a> - y) {
        <a href="math.md#0x2_math_MAX_U256">MAX_U256</a>
    } <b>else</b> { x + y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_sub_u256"></a>

## Function `saturating_sub_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_sub_u256">saturating_sub_u256</a>(x: u256, y: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_sub_u256">saturating_sub_u256</a>(x: u256, y: u256): u256 {
    <b>if</b> (y &gt; x) { 0 }
    <b>else</b> { x - y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_mul_u256"></a>

## Function `saturating_mul_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_u256">saturating_mul_u256</a>(x: u256, y: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_u256">saturating_mul_u256</a>(x: u256, y: u256): u256 {
    <b>if</b> (x == 0 || y == 0) { 0 }
    <b>else</b> <b>if</b> (x &gt; <a href="math.md#0x2_math_MAX_U256">MAX_U256</a> / y) {
        <a href="math.md#0x2_math_MAX_U256">MAX_U256</a>
    } <b>else</b> { x * y }
}
</code></pre>



</details>

<a name="0x2_math_saturating_pow_u256"></a>

## Function `saturating_pow_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_pow_u256">saturating_pow_u256</a>(base: u256, exponent: u32): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_pow_u256">saturating_pow_u256</a>(base: u256, exponent: u32): u256 {
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_pow_u256">try_pow_u256</a>(base, exponent);
    <b>if</b> (ok) { v }
    <b>else</b> {
        <a href="math.md#0x2_math_MAX_U256">MAX_U256</a>
    }
}
</code></pre>



</details>

<a name="0x2_math_saturating_mul_div_u256"></a>

## Function `saturating_mul_div_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_div_u256">saturating_mul_div_u256</a>(x: u256, y: u256, z: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_saturating_mul_div_u256">saturating_mul_div_u256</a>(x: u256, y: u256, z: u256): u256 {
    <b>assert</b>!(z != 0, <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>let</b> (ok, v) = <a href="math.md#0x2_math_try_mul_div_u256">try_mul_div_u256</a>(x, y, z);
    <b>if</b> (ok) { v }
    <b>else</b> {
        <a href="math.md#0x2_math_MAX_U256">MAX_U256</a>
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_shl_u8"></a>

## Function `checked_shl_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shl_u8">checked_shl_u8</a>(x: u8, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shl_u8">checked_shl_u8</a>(x: u8, shift: u8): Option&lt;u8&gt; {
    <b>if</b> (shift &gt;= 8) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x &lt;&lt; shift)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_shr_u8"></a>

## Function `checked_shr_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shr_u8">checked_shr_u8</a>(x: u8, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shr_u8">checked_shr_u8</a>(x: u8, shift: u8): Option&lt;u8&gt; {
    <b>if</b> (shift &gt;= 8) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x &gt;&gt; shift)
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_shl_u8"></a>

## Function `lossless_shl_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shl_u8">lossless_shl_u8</a>(x: u8, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shl_u8">lossless_shl_u8</a>(x: u8, shift: u8): Option&lt;u8&gt; {
    <b>if</b> (shift &gt;= 8) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> r = x &lt;&lt; shift;
        <b>if</b> ((r &gt;&gt; shift) != x) {
            <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
        } <b>else</b> {
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(r)
        }
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_shr_u8"></a>

## Function `lossless_shr_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shr_u8">lossless_shr_u8</a>(x: u8, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shr_u8">lossless_shr_u8</a>(x: u8, shift: u8): Option&lt;u8&gt; {
    <b>if</b> (shift &gt;= 8) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> r = x &gt;&gt; shift;
        <b>if</b> ((r &lt;&lt; shift) != x) {
            <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
        } <b>else</b> {
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(r)
        }
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_div_u8"></a>

## Function `lossless_div_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_div_u8">lossless_div_u8</a>(x: u8, y: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_div_u8">lossless_div_u8</a>(x: u8, y: u8): Option&lt;u8&gt; {
    <b>if</b> (y == 0 || x % y != 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x / y)
    }
}
</code></pre>



</details>

<a name="0x2_math_bitwise_not_u8"></a>

## Function `bitwise_not_u8`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bitwise_not_u8">bitwise_not_u8</a>(x: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bitwise_not_u8">bitwise_not_u8</a>(x: u8): u8 {
    x ^ <a href="math.md#0x2_math_MAX_U8">MAX_U8</a>
}
</code></pre>



</details>

<a name="0x2_math_checked_shl_u16"></a>

## Function `checked_shl_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shl_u16">checked_shl_u16</a>(x: u16, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shl_u16">checked_shl_u16</a>(x: u16, shift: u8): Option&lt;u16&gt; {
    <b>if</b> (shift &gt;= 16) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x &lt;&lt; shift)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_shr_u16"></a>

## Function `checked_shr_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shr_u16">checked_shr_u16</a>(x: u16, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shr_u16">checked_shr_u16</a>(x: u16, shift: u8): Option&lt;u16&gt; {
    <b>if</b> (shift &gt;= 16) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x &gt;&gt; shift)
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_shl_u16"></a>

## Function `lossless_shl_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shl_u16">lossless_shl_u16</a>(x: u16, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shl_u16">lossless_shl_u16</a>(x: u16, shift: u8): Option&lt;u16&gt; {
    <b>if</b> (shift &gt;= 16) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> r = x &lt;&lt; shift;
        <b>if</b> ((r &gt;&gt; shift) != x) {
            <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
        } <b>else</b> {
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(r)
        }
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_shr_u16"></a>

## Function `lossless_shr_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shr_u16">lossless_shr_u16</a>(x: u16, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shr_u16">lossless_shr_u16</a>(x: u16, shift: u8): Option&lt;u16&gt; {
    <b>if</b> (shift &gt;= 16) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> r = x &gt;&gt; shift;
        <b>if</b> ((r &lt;&lt; shift) != x) {
            <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
        } <b>else</b> {
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(r)
        }
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_div_u16"></a>

## Function `lossless_div_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_div_u16">lossless_div_u16</a>(x: u16, y: u16): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_div_u16">lossless_div_u16</a>(x: u16, y: u16): Option&lt;u16&gt; {
    <b>if</b> (y == 0 || x % y != 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x / y)
    }
}
</code></pre>



</details>

<a name="0x2_math_bitwise_not_u16"></a>

## Function `bitwise_not_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bitwise_not_u16">bitwise_not_u16</a>(x: u16): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bitwise_not_u16">bitwise_not_u16</a>(x: u16): u16 {
    x ^ <a href="math.md#0x2_math_MAX_U16">MAX_U16</a>
}
</code></pre>



</details>

<a name="0x2_math_checked_shl_u32"></a>

## Function `checked_shl_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shl_u32">checked_shl_u32</a>(x: u32, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shl_u32">checked_shl_u32</a>(x: u32, shift: u8): Option&lt;u32&gt; {
    <b>if</b> (shift &gt;= 32) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x &lt;&lt; shift)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_shr_u32"></a>

## Function `checked_shr_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shr_u32">checked_shr_u32</a>(x: u32, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shr_u32">checked_shr_u32</a>(x: u32, shift: u8): Option&lt;u32&gt; {
    <b>if</b> (shift &gt;= 32) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x &gt;&gt; shift)
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_shl_u32"></a>

## Function `lossless_shl_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shl_u32">lossless_shl_u32</a>(x: u32, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shl_u32">lossless_shl_u32</a>(x: u32, shift: u8): Option&lt;u32&gt; {
    <b>if</b> (shift &gt;= 32) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> r = x &lt;&lt; shift;
        <b>if</b> ((r &gt;&gt; shift) != x) {
            <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
        } <b>else</b> {
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(r)
        }
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_shr_u32"></a>

## Function `lossless_shr_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shr_u32">lossless_shr_u32</a>(x: u32, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shr_u32">lossless_shr_u32</a>(x: u32, shift: u8): Option&lt;u32&gt; {
    <b>if</b> (shift &gt;= 32) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> r = x &gt;&gt; shift;
        <b>if</b> ((r &lt;&lt; shift) != x) {
            <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
        } <b>else</b> {
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(r)
        }
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_div_u32"></a>

## Function `lossless_div_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_div_u32">lossless_div_u32</a>(x: u32, y: u32): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_div_u32">lossless_div_u32</a>(x: u32, y: u32): Option&lt;u32&gt; {
    <b>if</b> (y == 0 || x % y != 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x / y)
    }
}
</code></pre>



</details>

<a name="0x2_math_bitwise_not_u32"></a>

## Function `bitwise_not_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bitwise_not_u32">bitwise_not_u32</a>(x: u32): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bitwise_not_u32">bitwise_not_u32</a>(x: u32): u32 {
    x ^ <a href="math.md#0x2_math_MAX_U32">MAX_U32</a>
}
</code></pre>



</details>

<a name="0x2_math_checked_shl_u64"></a>

## Function `checked_shl_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shl_u64">checked_shl_u64</a>(x: u64, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shl_u64">checked_shl_u64</a>(x: u64, shift: u8): Option&lt;u64&gt; {
    <b>if</b> (shift &gt;= 64) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x &lt;&lt; shift)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_shr_u64"></a>

## Function `checked_shr_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shr_u64">checked_shr_u64</a>(x: u64, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shr_u64">checked_shr_u64</a>(x: u64, shift: u8): Option&lt;u64&gt; {
    <b>if</b> (shift &gt;= 64) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x &gt;&gt; shift)
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_shl_u64"></a>

## Function `lossless_shl_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shl_u64">lossless_shl_u64</a>(x: u64, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shl_u64">lossless_shl_u64</a>(x: u64, shift: u8): Option&lt;u64&gt; {
    <b>if</b> (shift &gt;= 64) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> r = x &lt;&lt; shift;
        <b>if</b> ((r &gt;&gt; shift) != x) {
            <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
        } <b>else</b> {
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(r)
        }
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_shr_u64"></a>

## Function `lossless_shr_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shr_u64">lossless_shr_u64</a>(x: u64, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shr_u64">lossless_shr_u64</a>(x: u64, shift: u8): Option&lt;u64&gt; {
    <b>if</b> (shift &gt;= 64) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> r = x &gt;&gt; shift;
        <b>if</b> ((r &lt;&lt; shift) != x) {
            <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
        } <b>else</b> {
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(r)
        }
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_div_u64"></a>

## Function `lossless_div_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_div_u64">lossless_div_u64</a>(x: u64, y: u64): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_div_u64">lossless_div_u64</a>(x: u64, y: u64): Option&lt;u64&gt; {
    <b>if</b> (y == 0 || x % y != 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x / y)
    }
}
</code></pre>



</details>

<a name="0x2_math_bitwise_not_u64"></a>

## Function `bitwise_not_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bitwise_not_u64">bitwise_not_u64</a>(x: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bitwise_not_u64">bitwise_not_u64</a>(x: u64): u64 {
    x ^ <a href="math.md#0x2_math_MAX_U64">MAX_U64</a>
}
</code></pre>



</details>

<a name="0x2_math_checked_shl_u128"></a>

## Function `checked_shl_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shl_u128">checked_shl_u128</a>(x: u128, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shl_u128">checked_shl_u128</a>(x: u128, shift: u8): Option&lt;u128&gt; {
    <b>if</b> (shift &gt;= 128) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x &lt;&lt; shift)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_shr_u128"></a>

## Function `checked_shr_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shr_u128">checked_shr_u128</a>(x: u128, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shr_u128">checked_shr_u128</a>(x: u128, shift: u8): Option&lt;u128&gt; {
    <b>if</b> (shift &gt;= 128) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x &gt;&gt; shift)
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_shl_u128"></a>

## Function `lossless_shl_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shl_u128">lossless_shl_u128</a>(x: u128, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shl_u128">lossless_shl_u128</a>(x: u128, shift: u8): Option&lt;u128&gt; {
    <b>if</b> (shift &gt;= 128) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> r = x &lt;&lt; shift;
        <b>if</b> ((r &gt;&gt; shift) != x) {
            <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
        } <b>else</b> {
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(r)
        }
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_shr_u128"></a>

## Function `lossless_shr_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shr_u128">lossless_shr_u128</a>(x: u128, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shr_u128">lossless_shr_u128</a>(x: u128, shift: u8): Option&lt;u128&gt; {
    <b>if</b> (shift &gt;= 128) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> r = x &gt;&gt; shift;
        <b>if</b> ((r &lt;&lt; shift) != x) {
            <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
        } <b>else</b> {
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(r)
        }
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_div_u128"></a>

## Function `lossless_div_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_div_u128">lossless_div_u128</a>(x: u128, y: u128): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_div_u128">lossless_div_u128</a>(x: u128, y: u128): Option&lt;u128&gt; {
    <b>if</b> (y == 0 || x % y != 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x / y)
    }
}
</code></pre>



</details>

<a name="0x2_math_bitwise_not_u128"></a>

## Function `bitwise_not_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bitwise_not_u128">bitwise_not_u128</a>(x: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bitwise_not_u128">bitwise_not_u128</a>(x: u128): u128 {
    x ^ <a href="math.md#0x2_math_MAX_U128">MAX_U128</a>
}
</code></pre>



</details>

<a name="0x2_math_checked_shl_u256"></a>

## Function `checked_shl_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shl_u256">checked_shl_u256</a>(x: u256, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shl_u256">checked_shl_u256</a>(x: u256, shift: u8): Option&lt;u256&gt; {
    <b>if</b> (shift &gt;= 255) {
        // shift == 255 is representable in u8; &gt;= 256 is not. Guard the edge explicitly.
        <b>if</b> (shift == 255) {
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x &lt;&lt; shift)
        } <b>else</b> {
            <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
        }
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x &lt;&lt; shift)
    }
}
</code></pre>



</details>

<a name="0x2_math_checked_shr_u256"></a>

## Function `checked_shr_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shr_u256">checked_shr_u256</a>(x: u256, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_checked_shr_u256">checked_shr_u256</a>(x: u256, shift: u8): Option&lt;u256&gt; {
    // u8 max is 255 &lt; 256, so every u8 shift is in range.
    <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x &gt;&gt; shift)
}
</code></pre>



</details>

<a name="0x2_math_lossless_shl_u256"></a>

## Function `lossless_shl_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shl_u256">lossless_shl_u256</a>(x: u256, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shl_u256">lossless_shl_u256</a>(x: u256, shift: u8): Option&lt;u256&gt; {
    // Same range note <b>as</b> checked_shl_u256: only 255 needs care, and it is valid.
    <b>let</b> r = x &lt;&lt; shift;
    <b>if</b> ((r &gt;&gt; shift) != x) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(r)
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_shr_u256"></a>

## Function `lossless_shr_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shr_u256">lossless_shr_u256</a>(x: u256, shift: u8): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_shr_u256">lossless_shr_u256</a>(x: u256, shift: u8): Option&lt;u256&gt; {
    <b>let</b> r = x &gt;&gt; shift;
    <b>if</b> ((r &lt;&lt; shift) != x) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(r)
    }
}
</code></pre>



</details>

<a name="0x2_math_lossless_div_u256"></a>

## Function `lossless_div_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_div_u256">lossless_div_u256</a>(x: u256, y: u256): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_lossless_div_u256">lossless_div_u256</a>(x: u256, y: u256): Option&lt;u256&gt; {
    <b>if</b> (y == 0 || x % y != 0) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(x / y)
    }
}
</code></pre>



</details>

<a name="0x2_math_bitwise_not_u256"></a>

## Function `bitwise_not_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bitwise_not_u256">bitwise_not_u256</a>(x: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_bitwise_not_u256">bitwise_not_u256</a>(x: u256): u256 {
    x ^ <a href="math.md#0x2_math_MAX_U256">MAX_U256</a>
}
</code></pre>



</details>

<a name="0x2_math_try_as_u8_from_u16"></a>

## Function `try_as_u8_from_u16`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u8_from_u16">try_as_u8_from_u16</a>(x: u16): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u8_from_u16">try_as_u8_from_u16</a>(x: u16): Option&lt;u8&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U8">MAX_U8</a> <b>as</b> u16)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u8))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u8_from_u32"></a>

## Function `try_as_u8_from_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u8_from_u32">try_as_u8_from_u32</a>(x: u32): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u8_from_u32">try_as_u8_from_u32</a>(x: u32): Option&lt;u8&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U8">MAX_U8</a> <b>as</b> u32)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u8))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u16_from_u32"></a>

## Function `try_as_u16_from_u32`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u16_from_u32">try_as_u16_from_u32</a>(x: u32): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u16_from_u32">try_as_u16_from_u32</a>(x: u32): Option&lt;u16&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U16">MAX_U16</a> <b>as</b> u32)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u16))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u8_from_u64"></a>

## Function `try_as_u8_from_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u8_from_u64">try_as_u8_from_u64</a>(x: u64): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u8_from_u64">try_as_u8_from_u64</a>(x: u64): Option&lt;u8&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U8">MAX_U8</a> <b>as</b> u64)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u8))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u16_from_u64"></a>

## Function `try_as_u16_from_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u16_from_u64">try_as_u16_from_u64</a>(x: u64): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u16_from_u64">try_as_u16_from_u64</a>(x: u64): Option&lt;u16&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U16">MAX_U16</a> <b>as</b> u64)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u16))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u32_from_u64"></a>

## Function `try_as_u32_from_u64`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u32_from_u64">try_as_u32_from_u64</a>(x: u64): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u32_from_u64">try_as_u32_from_u64</a>(x: u64): Option&lt;u32&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U32">MAX_U32</a> <b>as</b> u64)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u32))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u8_from_u128"></a>

## Function `try_as_u8_from_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u8_from_u128">try_as_u8_from_u128</a>(x: u128): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u8_from_u128">try_as_u8_from_u128</a>(x: u128): Option&lt;u8&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U8">MAX_U8</a> <b>as</b> u128)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u8))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u16_from_u128"></a>

## Function `try_as_u16_from_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u16_from_u128">try_as_u16_from_u128</a>(x: u128): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u16_from_u128">try_as_u16_from_u128</a>(x: u128): Option&lt;u16&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U16">MAX_U16</a> <b>as</b> u128)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u16))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u32_from_u128"></a>

## Function `try_as_u32_from_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u32_from_u128">try_as_u32_from_u128</a>(x: u128): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u32_from_u128">try_as_u32_from_u128</a>(x: u128): Option&lt;u32&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U32">MAX_U32</a> <b>as</b> u128)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u32))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u64_from_u128"></a>

## Function `try_as_u64_from_u128`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u64_from_u128">try_as_u64_from_u128</a>(x: u128): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u64_from_u128">try_as_u64_from_u128</a>(x: u128): Option&lt;u64&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U64">MAX_U64</a> <b>as</b> u128)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u64))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u8_from_u256"></a>

## Function `try_as_u8_from_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u8_from_u256">try_as_u8_from_u256</a>(x: u256): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u8_from_u256">try_as_u8_from_u256</a>(x: u256): Option&lt;u8&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U8">MAX_U8</a> <b>as</b> u256)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u8))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u16_from_u256"></a>

## Function `try_as_u16_from_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u16_from_u256">try_as_u16_from_u256</a>(x: u256): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u16_from_u256">try_as_u16_from_u256</a>(x: u256): Option&lt;u16&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U16">MAX_U16</a> <b>as</b> u256)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u16))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u32_from_u256"></a>

## Function `try_as_u32_from_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u32_from_u256">try_as_u32_from_u256</a>(x: u256): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u32_from_u256">try_as_u32_from_u256</a>(x: u256): Option&lt;u32&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U32">MAX_U32</a> <b>as</b> u256)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u32))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u64_from_u256"></a>

## Function `try_as_u64_from_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u64_from_u256">try_as_u64_from_u256</a>(x: u256): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u64_from_u256">try_as_u64_from_u256</a>(x: u256): Option&lt;u64&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U64">MAX_U64</a> <b>as</b> u256)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u64))
    }
}
</code></pre>



</details>

<a name="0x2_math_try_as_u128_from_u256"></a>

## Function `try_as_u128_from_u256`



<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u128_from_u256">try_as_u128_from_u256</a>(x: u256): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_try_as_u128_from_u256">try_as_u128_from_u256</a>(x: u256): Option&lt;u128&gt; {
    <b>if</b> (x &gt; (<a href="math.md#0x2_math_MAX_U128">MAX_U128</a> <b>as</b> u256)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>((x <b>as</b> u128))
    }
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
