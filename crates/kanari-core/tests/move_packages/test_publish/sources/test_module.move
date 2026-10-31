module 0x1::test_module {
    const VALUE: u64 = 42;

    public fun value(): u64 {
        Self::VALUE
    }
}
