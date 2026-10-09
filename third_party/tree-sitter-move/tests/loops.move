module test::loops {
    public fun sum_to(n: u64): u64 {
        let total = 0;
        let i = 0;
        while (i < n) {
            total = total + i;
            i = i + 1;
        };
        total
    }

    public fun find_first(v: &vector<u64>, target: u64): u64 {
        let i = 0;
        loop {
            if (i >= vector::length(v)) {
                break;
            };
            if (*vector::borrow(v, i) == target) {
                return i;
            };
            i = i + 1;
        };
        abort 0
    }
}
