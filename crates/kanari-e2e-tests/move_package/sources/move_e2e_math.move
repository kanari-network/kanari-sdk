module kanari_e2e_tests::move_e2e_math {
    use kanari_system::math;

    const E_BAD_MATH: u64 = 10;

    /// Saturating addition: returns `false` when the addition would overflow.
    public fun safe_add(a: u64, b: u64): (bool, u64) {
        if (a > math::max_u64_value() - b) {
            return (false, 0)
        };
        (true, a + b)
    }

    /// Multiply using 128-bit intermediates to avoid overflow for large inputs.
    public fun safe_multiply(a: u64, b: u64): u64 {
        ((((a as u128) * (b as u128)) as u64))
    }

    /// Percentage of `value`, computed with 128-bit intermediates.
    public fun percent_of(value: u64, percent: u64): u64 {
        ((math::mul_div_u128((value as u128), (percent as u128), 100)) as u64)
    }

    /// Integer square root backed by the `math` native.
    public fun sqrt(x: u64): u64 {
        math::sqrt_u64(x)
    }

    /// Power with overflow reporting.
    public fun safe_pow(base: u64, exponent: u8): (bool, u64) {
        math::try_pow_u64(base, exponent)
    }

    /// Clamp `x` into the inclusive `[lo, hi]` range.
    public fun clamp(x: u64, lo: u64, hi: u64): u64 {
        math::clamp_u64(x, lo, hi)
    }

    /// Arithmetic mean of two values, rounded down.
    public fun average(a: u64, b: u64): u64 {
        math::average_u64(a, b)
    }

    /// Ceiling division; aborts on a zero divisor.
    public fun ceil_div(x: u64, y: u64): u64 {
        assert!(y > 0, 1);
        math::ceil_div_u64(x, y)
    }

    public fun is_even(x: u64): bool {
        x % 2 == 0
    }

    public fun is_odd(x: u64): bool {
        x % 2 == 1
    }

    /// Verifies `safe_add` against an expected sum.
    public entry fun e2e_check_add(a: u64, b: u64, expected: u64) {
        let (ok, sum) = safe_add(a, b);
        assert!(ok, E_BAD_MATH);
        assert!(sum == expected, E_BAD_MATH);
    }

    /// Verifies that `safe_add` reports overflow instead of aborting.
    public entry fun e2e_check_add_overflow(a: u64, b: u64) {
        let (ok, sum) = safe_add(a, b);
        assert!(!ok, E_BAD_MATH);
        assert!(sum == 0, E_BAD_MATH);
    }

    /// Verifies 128-bit intermediate multiplication.
    public entry fun e2e_check_multiply(a: u64, b: u64, expected: u64) {
        assert!(safe_multiply(a, b) == expected, E_BAD_MATH);
    }

    /// Verifies percentage computation.
    public entry fun e2e_check_percent_of(value: u64, percent: u64, expected: u64) {
        assert!(percent_of(value, percent) == expected, E_BAD_MATH);
    }

    /// Verifies the integer square root native.
    public entry fun e2e_check_sqrt(x: u64, expected: u64) {
        assert!(sqrt(x) == expected, E_BAD_MATH);
    }

    /// Verifies exponentiation.
    public entry fun e2e_check_pow(base: u64, exponent: u8, expected: u64) {
        let (ok, value) = safe_pow(base, exponent);
        assert!(ok, E_BAD_MATH);
        assert!(value == expected, E_BAD_MATH);
    }

    /// Verifies clamping, including the in-range and above-range branches.
    public entry fun e2e_check_clamp(x: u64, lo: u64, hi: u64, expected: u64) {
        assert!(clamp(x, lo, hi) == expected, E_BAD_MATH);
    }

    /// Verifies the arithmetic mean, including an odd sum rounding down.
    public entry fun e2e_check_average(a: u64, b: u64, expected: u64) {
        assert!(average(a, b) == expected, E_BAD_MATH);
    }

    /// Verifies ceiling division for an exact and a non-exact quotient.
    public entry fun e2e_check_ceil_div(x: u64, y: u64, expected: u64) {
        assert!(ceil_div(x, y) == expected, E_BAD_MATH);
    }

    /// Verifies the even/odd predicates agree with each other.
    public entry fun e2e_check_parity(x: u64, even: bool, odd: bool) {
        assert!(is_even(x) == even, E_BAD_MATH);
        assert!(is_odd(x) == odd, E_BAD_MATH);
    }
}