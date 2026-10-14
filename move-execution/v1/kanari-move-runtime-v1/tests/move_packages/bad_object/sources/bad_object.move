#[allow(unused_field)]
module tester::bad_object {
    /// A `key` struct WITHOUT the required leading `id: UID`.
    /// Compiles fine (the language allows it) but publishing must fail
    /// object-model verification.
    struct Bad has key, store {
        value: u64,
    }
}
