module 0x8675309::M {
    public struct NoCopy has drop { v: u64 }

    // `&*r` on a shared reference is a shared alias; `r` stays usable.
    fun shared_alias(v: vector<u64>): u64 {
        let r = &v;
        let x = &*r;
        let y = r;
        vector::length(x) + vector::length(y)
    }

    // No `copy` is demanded of the referent.
    fun shared_nocopy(v: vector<NoCopy>): u64 {
        let r = &v;
        let x = &*r;
        vector::length(x)
    }

    // `&*r` narrows a `&mut` to a `&`.
    fun narrow(v: vector<u64>): u64 {
        let mut v = v;
        let r = &mut v;
        let x = &*r;
        vector::length(x)
    }

    // `&mut *r` on a `&mut` is a fresh mutable reference.
    fun mut_reborrow(v: vector<u64>): vector<u64> {
        let mut v = v;
        let r = &mut v;
        let x = &mut *r;
        vector::push_back(x, 1);
        v
    }

    // Writing through a reborrowed field of a non-copy struct.
    fun mut_reborrow_field(v: vector<NoCopy>) {
        let mut v = v;
        let r = &mut v;
        let x = &mut *r;
        let e = vector::borrow_mut(x, 0);
        e.v = 7;
    }

    // `for` over a reference parameter, via an explicit reborrow.
    fun sum_ref_param(v: &vector<u64>): u64 {
        let mut total = 0u64;
        for x in &*v {
            total = total + *x;
        };
        total
    }

    fun sum_nocopy_ref_param(v: &vector<NoCopy>): u64 {
        let mut total = 0u64;
        for e in &*v {
            total = total + e.v;
        };
        total
    }

    fun double_ref_param(v: &mut vector<u64>) {
        for x in &mut *v {
            *x = *x * 2;
        };
    }
}
