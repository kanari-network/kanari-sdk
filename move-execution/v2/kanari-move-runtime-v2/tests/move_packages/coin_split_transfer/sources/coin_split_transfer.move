module tester::coin_split_transfer {
    use kanari_system::balance;
    use kanari_system::coin::{Self, Coin};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    /// Test-only marker asset; this package exists solely for runtime tests
    /// and is never deployed.
    struct MOCK has drop {}

    /// Faucet: mint `amount` of `MOCK` to the transaction sender.
    public entry fun mint(amount: u64, ctx: &mut TxContext) {
        let coin = coin::from_balance(balance::create<MOCK>(amount), ctx);
        transfer::public_transfer(coin, tx_context::sender(ctx));
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
