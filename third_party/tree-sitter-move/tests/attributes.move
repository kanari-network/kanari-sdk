#[test_only]
module test::attributes {
    use std::vector;

    #[test_only]
    struct Witness has drop {}

    #[test_only]
    const MARKER: u64 = 7; // trailing note

    #[test]
    fun test_witness() {
        let w = Witness {};
        let _ = w;
    }

    public native fun native_len<T>(v: &vector<T>): u64;

    native struct Handle has key, store;
}
