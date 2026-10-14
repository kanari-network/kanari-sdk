module tester::coin_split_transfer {
    use kanari_system::coin::{Self, Coin, TreasuryCap};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    /// Test-only marker asset; this package exists solely for runtime tests
    /// and is never deployed.
    struct MOCK has drop {}

    /// Create the test currency once per chain. Further creates abort
    /// (one canonical currency per coin type); the mint cap goes to the
    /// sender for later faucet calls.
    public entry fun setup_currency(ctx: &mut TxContext) {
        let (cap, meta) = coin::create_currency(
            MOCK {},
            6,
            b"MOCK",
            b"Mock coin",
            b"runtime test faucet",
            std::option::none(),
            ctx
        );
        transfer::public_transfer(cap, tx_context::sender(ctx));
        transfer::public_freeze_object(meta);
    }

    /// Faucet: mint `amount` of `MOCK` to the transaction sender.
    public entry fun mint(cap: &mut TreasuryCap<MOCK>, amount: u64, ctx: &mut TxContext) {
        let minted = coin::mint(cap, amount, ctx);
        transfer::public_transfer(minted, tx_context::sender(ctx));
    }

    /// Mirrors `kanari_system::pay::split_and_transfer`: split `amount` off
    /// `coin` and send the split part to `recipient`; the remainder stays.
    public entry fun split_and_transfer(
        coin: &mut Coin<MOCK>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext,
    ) {
        let part = coin::split(coin, amount, ctx);
        transfer::public_transfer(part, recipient);
    }

    /// Read a coin object's balance (view used to observe persisted state).
    public fun value_of(coin: &Coin<MOCK>): u64 {
        coin::value(coin)
    }
}
