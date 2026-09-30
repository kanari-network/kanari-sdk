module 0x8675309::M {
    use std::vector;

    // 'for' is a keyword from the Move 2024 edition onwards, and 'for' loops are gated on the
    // 'ForLoop' feature. In the legacy edition 'for' is not reserved at all, so it is read as an
    // ordinary identifier and the loop below does not parse.
    fun f() {
        let v = vector[1u64, 2];
        for x in v { }
    }
}
