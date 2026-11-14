module M::n { fun f(c: u8): u8 { if (c >= 48 && c <= 57) c - 48
 // 0-9
 else if (c >= 65) c - 55
 else abort E } }
