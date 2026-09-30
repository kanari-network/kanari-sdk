module 0x8675309::M {
    // `&mut *r` from a shared reference.
    fun mut_of_shared(v: vector<u64>) {
        let r = &v;
        let x = &mut *r;
    }

    // `&*r` where `r` is not a reference at all.
    fun deref_nonref(v: vector<u64>) {
        let x = &*v;
    }
}
