module kanari_system::event_tests {
    use kanari_system::event;
    use std::string::{String, utf8};

    struct TestEvent has copy, drop {
        id: u64,
        msg: String,
    }

    #[test]
    fun emit_test() {
        let ev = TestEvent { id: 7u64, msg: utf8(b"hello from test") };
        event::emit(ev);
    }
}
