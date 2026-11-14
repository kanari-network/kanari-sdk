module 0x1::M {

    public fun f(addr: address) {
        let c = borrow_global<Coin>(addr);
    }

}

