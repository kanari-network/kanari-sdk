module m::n {
    fun f() {
        if (/* 0-9 */ 48 <= hex && hex < 58) { hex - 48 } else { 0 }
    }
}
