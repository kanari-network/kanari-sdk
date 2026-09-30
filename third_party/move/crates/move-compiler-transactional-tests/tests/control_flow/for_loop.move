//# init --edition 2024

//# publish
module 0x42::m {

    public struct NoCopy has drop { v: u64 }

    public struct Pair has copy, drop { a: u64, b: u64 }

    public fun sum(v: vector<u64>): u64 {
        let mut total = 0u64;
        for x in v {
            total = total + x;
        };
        total
    }

    // The iterable is evaluated once, so this sums 1+2+3+4 rather than counting elements.
    public fun sum_of_call(): u64 {
        let mut total = 0u64;
        for x in make() {
            total = total + x;
        };
        total
    }

    public fun make(): vector<u64> {
        vector[1, 2, 3, 4]
    }

    public fun make_nocopy(): vector<NoCopy> {
        vector[NoCopy { v: 7 }, NoCopy { v: 8 }, NoCopy { v: 9 }]
    }

    // Elements of a type without `copy` are moved out of the vector, in order.
    public fun drain(v: vector<NoCopy>): (vector<u64>, u64) {
        let mut out = vector::empty<u64>();
        let mut seen = 0u64;
        for s in v {
            vector::push_back(&mut out, s.v);
            seen = seen + 1;
        };
        (out, seen)
    }

    public fun make_pairs(): vector<Pair> {
        vector[Pair { a: 1, b: 2 }, Pair { a: 3, b: 4 }]
    }

    // The loop variable can be a destructuring pattern.
    public fun sum_pairs(v: vector<Pair>): u64 {
        let mut total = 0u64;
        for Pair { a, b } in v {
            total = total + a + b;
        };
        total
    }

    public fun sum_nested(a: vector<u64>, b: vector<u64>): u64 {
        let mut total = 0u64;
        for x in a {
            for y in b {
                total = total + x * y;
            };
        };
        total
    }

    public fun sum_with_break(v: vector<u64>): u64 {
        let mut total = 0u64;
        for x in v {
            if (x == 3) { break };
            if (x == 1) { continue };
            total = total + x;
        };
        total
    }

    public fun sum_with_return(v: vector<u64>): u64 {
        for x in v {
            if (x == 2) { return 99 };
        };
        0
    }

    // Locals declared in the body must not capture the generated local.
    public fun body_shadows(v: vector<u64>): u64 {
        let mut seen = 0u64;
        for x in v {
            let mut __for_iterable = 1000u64;
            __for_iterable = __for_iterable + x;
            seen = seen + 1;
        };
        seen
    }

    // `use` declarations at the top of the body are preserved.
    public fun body_with_use(v: vector<u64>): u64 {
        let mut total = 0u64;
        for x in v {
            use 0x42::m::add_one as inc;
            total = inc(total) + x;
        };
        total
    }

    public fun add_one(x: u64): u64 { x + 1 }

    // A body does not have to be a block.
    public fun sum_unbraced(v: vector<u64>): u64 {
        let mut total = 0u64;
        for x in v total = total + x;
        total
    }

    public fun sum_empty(): u64 {
        let mut total = 0u64;
        for x in vector::empty<u64>() {
            total = total + x;
        };
        total
    }

    // `for x in &v` borrows the vector and binds each element by reference, so the vector is
    // still there afterwards.
    public fun sum_by_ref(v: vector<u64>): u64 {
        let mut total = 0u64;
        for x in &v {
            total = total + *x;
        };
        total
    }

    public fun len_after(v: vector<u64>): u64 {
        let mut seen = 0u64;
        for x in &v {
            let _ = *x;
            seen = seen + 1;
        };
        vector::length(&v) + seen
    }

    // `for x in &mut v` binds each element mutably, so the body can write through it. The
    // target has to be a `mut` local, exactly as if the borrow were written out by hand.
    public fun double_in_place(v: vector<u64>): vector<u64> {
        let mut v = v;
        for x in &mut v {
            *x = *x * 2;
        };
        v
    }

    public fun bump_weights(v: vector<NoCopy>): vector<NoCopy> {
        let mut v = v;
        for e in &mut v {
            e.v = e.v + 1;
        };
        v
    }

    // Reading and writing a vector that lives in a struct field, which is the case that a
    // plain `for x in h.items` cannot express: that would be an implicit copy.
    public struct Holder has drop { items: vector<u64>, nc: vector<NoCopy> }

    public fun make_holder(): Holder {
        Holder { items: vector[1, 2, 3], nc: vector[NoCopy { v: 10 }] }
    }

    public fun field_total(h: &Holder): u64 {
        let mut total = 0u64;
        for x in &h.items {
            total = total + *x;
        };
        total
    }

    public fun field_double(h: &mut Holder) {
        for x in &mut h.items {
            *x = *x + 1;
        };
    }

    public fun field_bump(h: &mut Holder) {
        for e in &mut h.nc {
            e.v = e.v + 100;
        };
    }

    public fun nc_total(h: &Holder): u64 {
        let mut total = 0u64;
        for e in &h.nc {
            total = total + e.v;
        };
        total
    }

    // A `&vector<T>` parameter cannot be borrowed again (`&` on a reference is rejected), so it
    // is iterated through an explicit reborrow `&*v`.
    public fun sum_ref_param(v: &vector<u64>): u64 {
        let mut total = 0u64;
        for x in &*v {
            total = total + *x;
        };
        total
    }

    public fun sum_nocopy_ref_param(v: &vector<NoCopy>): u64 {
        let mut total = 0u64;
        for e in &*v {
            total = total + e.v;
        };
        total
    }

    public fun double_ref_param(v: &mut vector<u64>) {
        for x in &mut *v {
            *x = *x * 2;
        };
    }

    // Locals declared in the body must not capture the generated ones in either mode.
    public fun body_shadows_both(v: vector<u64>, w: vector<u64>): u64 {
        let mut seen = 0u64;
        for __for_index in v {
            let mut __for_index = 1000u64;
            __for_index = __for_index + 1;
            seen = seen + 1;
        };
        for x in &w {
            let mut __for_iterable = 5u64;
            let mut __for_index = 7u64;
            let mut __for_length = 9u64;
            __for_iterable = __for_iterable + *x;
            __for_index = __for_index + 1;
            __for_length = __for_length + 1;
            seen = seen + 1;
        };
        seen
    }

    // The generated locals are not visible to the body, so the loop still sees every element.
    public fun shadow_does_not_truncate(v: vector<u64>): u64 {
        let mut count = 0u64;
        for __for_length in v {
            count = count + 1;
        };
        count
    }

    // `for` is an expression of unit type and composes with other control flow.
    public fun sum_in_if(v: vector<u64>): u64 {
        if (vector::is_empty(&v)) 0 else {
            let mut total = 0u64;
            for x in v { total = total + x; };
            total
        }
    }

    // A `for` used as a statement, mixing with ordinary sequence items. Like `while`, a
    // non-final block-bodied `for` still needs a trailing semicolon in Move.
    public fun sum_statement(v: vector<u64>): u64 {
        let mut total = 0u64;
        let mut n = 0u64;
        for x in v { total = total + x; n = n + 1; };
        total + n * 1000
    }
}

//# run
module 0x42::main {
    use 0x42::m::{
        body_shadows, body_shadows_both, body_with_use, bump_weights, double_in_place,
        double_ref_param, drain, field_bump, field_double, field_total, len_after, make_holder,
        make_nocopy, make_pairs, nc_total, shadow_does_not_truncate, sum, sum_by_ref, sum_empty,
        sum_in_if, sum_nested, sum_nocopy_ref_param, sum_of_call, sum_pairs, sum_ref_param,
        sum_statement, sum_unbraced, sum_with_break, sum_with_return
    };

    fun main() {
        let v = vector[1u64, 2, 3, 4];
        assert!(sum(copy v) == 10, 0);
        assert!(sum(vector[]) == 0, 1);
        assert!(sum_of_call() == 10, 2);

        // Moving non-`copy` elements out consumes the vector, in order.
        let (values, seen) = drain(make_nocopy());
        assert!(values == vector[7u64, 8, 9], 3);
        assert!(seen == 3, 4);

        assert!(sum_pairs(make_pairs()) == 10, 6);
        assert!(sum_nested(vector[1u64, 2], vector[10u64, 20]) == 90, 7);

        assert!(sum_with_break(vector[1u64, 2, 3, 4]) == 2, 8);
        assert!(sum_with_break(vector[1u64]) == 0, 9);
        assert!(sum_with_return(vector[1u64, 2, 3]) == 99, 10);
        assert!(sum_with_return(vector[1u64]) == 0, 11);

        assert!(body_shadows(vector[1u64, 2, 3]) == 3, 12);
        assert!(body_with_use(vector[1u64, 2, 3]) == 9, 13);
        assert!(sum_unbraced(vector[1u64, 2, 3]) == 6, 14);
        assert!(sum_empty() == 0, 15);
        assert!(sum_in_if(vector[1u64, 2, 3]) == 6, 16);
        assert!(sum_in_if(vector[]) == 0, 17);
        assert!(sum_statement(vector[1u64, 2, 3]) == 3006, 18);

        // Iterating by shared reference: the vector survives and is unchanged.
        assert!(sum_by_ref(vector[1u64, 2, 3]) == 6, 19);
        assert!(len_after(vector[1u64, 2, 3]) == 6, 20);

        // Iterating by mutable reference: the body writes through the element.
        assert!(double_in_place(vector[1u64, 2, 3]) == vector[2u64, 4, 6], 21);
        let bumped = bump_weights(make_nocopy());
        let (after, _) = drain(bumped);
        assert!(after == vector[8u64, 9, 10], 22);

        // Iterating a vector held in a struct field, shared and mutable.
        let mut h = make_holder();
        assert!(field_total(&h) == 6, 23);
        field_double(&mut h);
        assert!(field_total(&h) == 9, 24);
        field_bump(&mut h);
        assert!(nc_total(&h) == 110, 27);

        // Body locals never capture the generated ones, in either mode.
        assert!(body_shadows_both(vector[1u64, 2], vector[3u64, 4]) == 4, 25);
        assert!(shadow_does_not_truncate(vector[1u64, 2, 3]) == 3, 26);

        // A reference parameter is iterated through an explicit reborrow.
        assert!(sum_ref_param(&vector[1u64, 2, 3]) == 6, 27);
        assert!(sum_nocopy_ref_param(&make_nocopy()) == 24, 28);
        let mut w = vector[1u64, 2, 3];
        double_ref_param(&mut w);
        assert!(w == vector[2u64, 4, 6], 29);
    }
}
