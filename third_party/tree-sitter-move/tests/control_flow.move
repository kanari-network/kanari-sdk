module test::control_flow {
    const E_BAD: u64 = 0;

    public fun classify(amount: u64): u64 {
        if (amount == 0) {
            abort E_BAD
        } else if (amount < 100) {
            1
        } else {
            2
        }
    }

    public fun check(amount: u64) {
        assert!(amount > 0, E_BAD);
    }

    public fun early_return(flag: bool): u64 {
        if (flag) {
            return 1;
        };
        0
    }
}
