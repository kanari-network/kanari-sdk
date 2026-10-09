module test::loop_control {
    public fun find(values: vector<u64>, target: u64): u64 {
        let i = 0;
        while (i < vector::length(&values)) {
            let v = *vector::borrow(&values, i);
            if (v == target) {
                return i;
            };
            if (v > target) {
                i = i + 1;
                continue;
            };
            i = i + 1;
        };
        abort 99
    }

    public fun skip_evens(values: vector<u64>): u64 {
        let total = 0;
        let i = 0;
        loop {
            if (i >= vector::length(&values)) {
                break;
            };
            i = i + 1;
            if (i % 2 == 0) {
                continue;
            };
            total = total + 1;
        };
        total
    }
}
