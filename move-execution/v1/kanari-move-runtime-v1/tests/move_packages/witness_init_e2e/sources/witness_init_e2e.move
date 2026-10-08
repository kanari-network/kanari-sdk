module tester::witness_init_e2e {
    use kanari_system::event;
    use kanari_system::tx_context::TxContext;

    struct WITNESS_INIT_E2E has drop {}

    public entry fun noop() {}

    fun init(_witness: WITNESS_INIT_E2E, ctx: &mut TxContext) {
        let _ = ctx;
        event::emit<u64>(999);
    }
}

