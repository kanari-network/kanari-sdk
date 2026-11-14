module m::n {
    public fun f(x: u64) {
        if (x > 0) {
            y = 1;
        } else {
            y = 2;
        }
    }
}
