//# init --edition 2024

//# publish
// Differential fuzz for `for` loops and reborrows.
//
// Every check below computes the same result twice: once with the new `for` / `&*` syntax and
// once with a hand-written `while` loop that only uses long-established operations. The two must
// agree. Inputs are drawn from a deterministic xorshift PRNG so the test is reproducible, and the
// driver runs hundreds of seeds to cover lengths (including empty), values, thresholds, nesting,
// early exits, all three `for` modes, and explicit reborrows.
module 0x42::fuzz {
    public struct NoCopy has drop { v: u64 }

    public fun make_nocopy(v: u64): NoCopy { NoCopy { v } }

    public fun nocopy_v(s: &NoCopy): u64 { s.v }

    public struct Rng has drop { s: u64 }

    public fun seed(s: u64): Rng {
        Rng { s: if (s == 0) 0x853c49e6748fea9b else s }
    }

    public fun next(r: &mut Rng): u64 {
        let mut x = r.s;
        x = x ^ (x << 13);
        x = x ^ (x >> 7);
        x = x ^ (x << 17);
        r.s = x;
        x
    }

    public fun next_below(r: &mut Rng, n: u64): u64 {
        if (n == 0) 0 else next(r) % n
    }

    public fun rand_vec(r: &mut Rng): vector<u64> {
        let mut v = vector[];
        let n = next_below(r, 9);
        let mut i = 0u64;
        while (i < n) {
            vector::push_back(&mut v, next_below(r, 21));
            i = i + 1;
        };
        v
    }

    public fun rand_nocopy_vec(r: &mut Rng): vector<NoCopy> {
        let mut v = vector[];
        let n = next_below(r, 9);
        let mut i = 0u64;
        while (i < n) {
            vector::push_back(&mut v, NoCopy { v: next_below(r, 21) });
            i = i + 1;
        };
        v
    }

    // -- sum by value ---------------------------------------------------------

    public fun sum_for(v: vector<u64>): u64 {
        let mut total = 0u64;
        for x in v {
            total = total + x;
        };
        total
    }

    public fun sum_while(v: vector<u64>): u64 {
        let mut total = 0u64;
        let mut i = 0u64;
        let n = vector::length(&v);
        while (i < n) {
            total = total + *vector::borrow(&v, i);
            i = i + 1;
        };
        total
    }

    // -- sum by shared reference ----------------------------------------------

    public fun sum_ref_for(v: vector<u64>): u64 {
        let mut total = 0u64;
        for x in &v {
            total = total + *x;
        };
        total
    }

    public fun sum_ref_while(v: vector<u64>): u64 {
        let mut total = 0u64;
        let mut i = 0u64;
        let n = vector::length(&v);
        while (i < n) {
            total = total + *vector::borrow(&v, i);
            i = i + 1;
        };
        total
    }

    // -- double in place by mutable reference ----------------------------------

    public fun double_for(v: vector<u64>): vector<u64> {
        let mut v = v;
        for x in &mut v {
            *x = *x * 2;
        };
        v
    }

    public fun double_while(v: vector<u64>): vector<u64> {
        let mut v = v;
        let mut i = 0u64;
        let n = vector::length(&v);
        while (i < n) {
            *vector::borrow_mut(&mut v, i) = *vector::borrow(&v, i) * 2;
            i = i + 1;
        };
        v
    }

    // -- break / continue with a random threshold -------------------------------

    public fun bc_for(v: vector<u64>, stop: u64, skip: u64): u64 {
        let mut total = 0u64;
        for x in v {
            if (x > stop) { break };
            if (x == skip) { continue };
            total = total + x;
        };
        total
    }

    public fun bc_while(v: vector<u64>, stop: u64, skip: u64): u64 {
        let mut total = 0u64;
        let mut i = 0u64;
        let n = vector::length(&v);
        while (i < n) {
            let x = *vector::borrow(&v, i);
            if (x > stop) { break };
            if (x == skip) {
                i = i + 1;
                continue
            };
            total = total + x;
            i = i + 1;
        };
        total
    }

    // -- nested loops ------------------------------------------------------------

    public fun nested_for(a: vector<u64>, b: vector<u64>): u64 {
        let mut total = 0u64;
        for x in a {
            for y in b {
                total = total + x + y;
            };
        };
        total
    }

    public fun nested_while(a: vector<u64>, b: vector<u64>): u64 {
        let mut total = 0u64;
        let mut i = 0u64;
        let na = vector::length(&a);
        while (i < na) {
            let mut j = 0u64;
            let nb = vector::length(&b);
            while (j < nb) {
                total = total + *vector::borrow(&a, i) + *vector::borrow(&b, j);
                j = j + 1;
            };
            i = i + 1;
        };
        total
    }

    // -- early return --------------------------------------------------------------

    public fun ret_for(v: vector<u64>, sentinel: u64): u64 {
        for x in v {
            if (x == sentinel) { return 999 };
            if (x > 100) { return 998 };
        };
        0
    }

    public fun ret_while(v: vector<u64>, sentinel: u64): u64 {
        let mut i = 0u64;
        let n = vector::length(&v);
        while (i < n) {
            let x = *vector::borrow(&v, i);
            if (x == sentinel) { return 999 };
            if (x > 100) { return 998 };
            i = i + 1;
        };
        0
    }

    // -- drain a non-copy vector -----------------------------------------------------

    public fun drain_for(v: vector<NoCopy>): vector<u64> {
        let mut out = vector[];
        for s in v {
            vector::push_back(&mut out, s.v);
        };
        out
    }

    public fun drain_while(v: vector<NoCopy>): vector<u64> {
        let mut out = vector[];
        let mut w = v;
        while (!vector::is_empty(&w)) {
            let s = vector::remove(&mut w, 0);
            vector::push_back(&mut out, s.v);
        };
        out
    }

    // -- explicit reborrow of a reference parameter -------------------------------------

    public fun reborrow_for(v: &vector<u64>): u64 {
        let mut total = 0u64;
        for x in &*v {
            total = total + *x;
        };
        total
    }

    public fun reborrow_while(v: &vector<u64>): u64 {
        let mut total = 0u64;
        let mut i = 0u64;
        let n = vector::length(v);
        while (i < n) {
            total = total + *vector::borrow(v, i);
            i = i + 1;
        };
        total
    }

    public fun reborrow_mut_for(v: &mut vector<u64>) {
        for x in &mut *v {
            *x = *x + 1;
        };
    }

    public fun reborrow_mut_while(v: &mut vector<u64>) {
        let mut i = 0u64;
        let n = vector::length(v);
        while (i < n) {
            *vector::borrow_mut(v, i) = *vector::borrow(v, i) + 1;
            i = i + 1;
        };
    }

    // -- destructuring ------------------------------------------------------------------

    public fun destructure_for(v: vector<NoCopy>): u64 {
        let mut total = 0u64;
        for NoCopy { v: w } in v {
            total = total + w;
        };
        total
    }

    public fun destructure_while(v: vector<NoCopy>): u64 {
        let mut total = 0u64;
        let mut w = v;
        while (!vector::is_empty(&w)) {
            let NoCopy { v } = vector::remove(&mut w, 0);
            total = total + v;
        };
        total
    }
}

//# run
module 0x42::drive {
    use 0x42::fuzz;

    fun main() {
        let mut seed = 0u64;
        while (seed < 200) {
            let mut r = fuzz::seed(seed);
            let v = fuzz::rand_vec(&mut r);
            let w = fuzz::rand_vec(&mut r);
            let nc = fuzz::rand_nocopy_vec(&mut r);
            let stop = fuzz::next_below(&mut r, 22);
            let skip = fuzz::next_below(&mut r, 21);
            let sentinel = fuzz::next_below(&mut r, 25);

            assert!(fuzz::sum_for(copy v) == fuzz::sum_while(copy v), seed);
            assert!(fuzz::sum_ref_for(copy v) == fuzz::sum_ref_while(copy v), seed);
            assert!(fuzz::double_for(copy v) == fuzz::double_while(copy v), seed);
            assert!(
                fuzz::bc_for(copy v, stop, skip) == fuzz::bc_while(copy v, stop, skip),
                seed
            );
            assert!(
                fuzz::nested_for(copy v, copy w) == fuzz::nested_while(copy v, copy w),
                seed
            );
            assert!(
                fuzz::ret_for(copy v, sentinel) == fuzz::ret_while(copy v, sentinel),
                seed
            );

            // Non-copy vectors cannot be copied, so rebuild identical contents three times
            // from the drawn values.
            let mut nc1 = vector[];
            let mut nc2 = vector[];
            let mut nc3 = vector[];
            let mut vals = vector[];
            let mut i = 0u64;
            while (i < vector::length(&nc)) {
                let val = fuzz::nocopy_v(vector::borrow(&nc, i));
                vector::push_back(&mut nc1, fuzz::make_nocopy(val));
                vector::push_back(&mut nc2, fuzz::make_nocopy(val));
                vector::push_back(&mut nc3, fuzz::make_nocopy(val));
                vector::push_back(&mut vals, val);
                i = i + 1;
            };
            assert!(fuzz::drain_for(nc1) == fuzz::drain_while(nc2), seed);
            assert!(fuzz::destructure_for(nc3) == fuzz::sum_while(vals), seed);

            assert!(fuzz::reborrow_for(&v) == fuzz::reborrow_while(&v), seed);

            let mut m1 = copy v;
            let mut m2 = copy v;
            fuzz::reborrow_mut_for(&mut m1);
            fuzz::reborrow_mut_while(&mut m2);
            assert!(m1 == m2, seed);

            seed = seed + 1;
        };
    }
}
