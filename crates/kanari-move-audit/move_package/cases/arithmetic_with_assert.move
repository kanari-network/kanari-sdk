module 0x1::M {

    public fun add(a: u64, b: u64): u64 {
        assert!(a <= 100, 0);
        a + b
    }

}

