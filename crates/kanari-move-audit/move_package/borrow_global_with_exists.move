module 0x1::M {

    public fun f(addr: address) {
        assert!(exists<Coin>(addr), 0);
        let c = borrow_global<Coin>(addr);
    }

}

