module test::macros {
    macro fun each<T>($v: &vector<T>, $f: |&T|): u64 {
        0
    }

    public fun count<T>(v: &vector<T>): u64 {
        each!(&v, |_x| {})
    }
}
