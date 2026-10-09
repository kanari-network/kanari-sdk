#[test_only]
module test::test_helpers {
    use kanari_system::tx_context;

    public fun dummy_sender(): address {
        let ctx = tx_context::dummy();
        tx_context::sender(&ctx)
    }

    #[test]
    fun works() {
        assert!(dummy_sender() != @0x0, 0);
    }

    #[test]
    #[expected_failure(abort_code = 1)]
    fun fails() {
        abort 1
    }
}
