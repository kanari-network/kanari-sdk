module tester::init_e2e {
    use kanari_system::event;
    use kanari_system::tx_context::TxContext;

    public entry fun noop() {}

    fun init(_ctx: &mut TxContext) {
        event::emit<u64>(888);
    }
}

