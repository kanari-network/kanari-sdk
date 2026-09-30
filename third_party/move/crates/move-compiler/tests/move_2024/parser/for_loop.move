module 0x8675309::M {
    public struct S has drop { v: u64, items: vector<u64> }

    fun single(v: vector<u64>) {
        for x in v { consume(x) };
    }

    fun destructuring(v: vector<S>) {
        for S { v: inner, items: _ } in v { consume(inner) };
    }

    fun nested_field(v: vector<S>) {
        for s in v { consume(s.v) };
    }

    fun wildcard(v: vector<u64>) {
        for _ in v { };
    }

    // The iterable is any expression; the '{' after it opens the body, not a pack.
    fun non_name_iterable() {
        for x in make() { consume(x) };
        for x in vector::empty<u64>() { consume(x) };
        for x in if (true) vector[1] else vector[2] { consume(x) };
    }

    // A leading '&' or '&mut' is what selects iteration by reference, and it composes with
    // any place expression on the right.
    fun by_reference(v: vector<u64>, s: S) {
        for x in &v { consume(*x) };
        for x in &s.items { consume(*x) };
        for x in &make() { consume(*x) };
    }

    // `&mut` needs a `mut` place to borrow, exactly like a hand-written borrow.
    fun by_mutable_reference() {
        let mut v = vector[1u64, 2];
        for x in &mut v { consume(*x) };

        let mut s = S { v: 0, items: vector[1u64] };
        for x in &mut s.items { consume(*x) };
    }

    // Braces are not required on the body, matching 'while' and 'loop'.
    fun unbraced_body(v: vector<u64>) {
        for x in v consume(x)
    }

    // A 'for' is an expression, so it composes with the rest of the grammar.
    fun as_expression(v: vector<u64>): u64 {
        let mut n = 0u64;
        if (true) { for x in v { n = n + x; }; } else { };
        while (n < 10) { for x in v { n = n + x; }; };
        loop { for x in v { n = n + x; }; break };
        n
    }

    fun nested(v: vector<u64>, w: vector<u64>) {
        for x in v {
            for y in w {
                consume(x + y)
            }
        };
    }

    fun make(): vector<u64> { vector[] }

    fun consume(_x: u64) { }
}
