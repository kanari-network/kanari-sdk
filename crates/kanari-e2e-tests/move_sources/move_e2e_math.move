module kanari_e2e_tests::move_e2e_math {
    use kanari_system::math;

    public fun add(a: u64, b: u64): u64 {
        a + b
    }

    public fun multiply(a: u64, b: u64): u64 {
        a * b
    }

    public fun safe_add(a: u64, b: u64): (bool, u64) {
        if (a > 18446744073709551615 - b) {
            return (false, 0)
        };
        (true, a + b)
    }

    public fun compute_percentage(value: u64, percent: u64): u64 {
        (value * percent) / 100
    }

    public fun is_even(num: u64): bool {
        num % 2 == 0
    }

    public fun is_odd(num: u64): bool {
        num % 2 == 1
    }

    public fun clamp(num: u64, min_val: u64, max_val: u64): u64 {
        if (num < min_val) return min_val;
        if (num > max_val) return max_val;
        num
    }

    public fun min_val(a: u64, b: u64): u64 {
        if (a < b) a else b
    }

    public fun max_val(a: u64, b: u64): u64 {
        if (a > b) a else b
    }
}

