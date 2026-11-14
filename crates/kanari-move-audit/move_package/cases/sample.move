module 0x1::Example {

    entry fun bad_entry(x: u64) {}
    entry fun good_entry(signer: signer) {}
    public fun uses_borrow(addr: address) {
        let c = borrow_global<Coin>(addr);
    }
    public fun safe_borrow(addr: address) {
        assert!(exists<Coin>(addr), 0);
        let c = borrow_global<Coin>(addr);
    }
    public fun hardcoded() {
        let a = 0x12345678;
    }
    public entry fun external_call(signer: signer) {
        let _ = 0x2::Other::foo(&mut ctx);
    }
    public fun undocumented() {}
    public fun documented() {
        // docs
    }

}

