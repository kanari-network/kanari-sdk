module test::first {
    public fun one(): u64 {
        1
    }
}

module test::second {
    use test::first;

    public fun two(): u64 {
        first::one() + 1
    }
}
