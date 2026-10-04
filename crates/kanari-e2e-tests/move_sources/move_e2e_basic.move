module kanari_e2e_tests::move_e2e_basic {
    use kanari_system::coin;
    use kanari_system::tx_context::TxContext;

    struct TEST_COIN has drop {}

    /// Initialize a test coin for e2e verification.
    public fun init_test_coin(ctx: &mut TxContext) {
        let (treasury_cap, metadata) = coin::create_currency<TEST_COIN>(
            TEST_COIN {},
            9,
            b"TEST",
            b"Test Coin",
            b"E2E test coin",
            std::option::none(),
            ctx,
        );
        let sender = kanari_system::tx_context::sender(ctx);
        kanari_system::transfer::public_share_object(treasury_cap);
        kanari_system::transfer::public_share_object(metadata);
        kanari_system::transfer::public_transfer(
            coin::mint<TEST_COIN>(&mut treasury_cap, 1_000_000_000_000, ctx),
            sender,
        );
    }
}
