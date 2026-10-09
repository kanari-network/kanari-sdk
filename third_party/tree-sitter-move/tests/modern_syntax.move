module test::modern_syntax {
    use std::vector;

    public fun total(values: vector<u64>): u64 {
        let sum = 0;
        vector::for_each(values, |v| {
            sum = sum + v;
        });
        sum
    }

    public fun first_three(): vector<u64> {
        vector[1, 2, 3]
    }
}
