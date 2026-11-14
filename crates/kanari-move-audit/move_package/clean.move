module 0x1::Clean {

    /// Add two numbers safely.
    public fun add(a: u64, b: u64): u64 {
        assert!(a <= 100, 0);
        a + b
    }

    /// Do thing.
    public entry fun do_thing(signer: signer) {
        let _ = signer;
    }

}

