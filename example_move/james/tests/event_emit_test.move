module james::event_emit_test {
    use kanari_system::event;
    use std::string::{String, utf8};

    struct TestEvent has copy, drop {
        seq: u64,
        note: String,
    }

    #[test]
    fun emit_from_james() {
        let e = TestEvent { seq: 1u64, note: utf8(b"james emit test") };
        event::emit(e);
    }
}
