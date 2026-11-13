module m::n {
    struct S has drop { x: u64 }
    fun f() { let s = S { x: 1 }; let y = s.x; }
}
