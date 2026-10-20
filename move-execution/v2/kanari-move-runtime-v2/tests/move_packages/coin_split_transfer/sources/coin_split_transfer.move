module tester::coin_split_transfer {
    use kanari_system::balance::{Self, Supply};
    use kanari_system::coin::{Self, Coin};
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    /// Test-only marker asset; this package exists solely for runtime tests
    /// and is never deployed.
    struct MOCK has drop {}

    /// Faucet holding the mint authority for `MOCK`. Production-shaped:
    /// the `Supply` handle is created witness-gated by `balance::new_supply`,
    /// so no value is ever conjured without a registered supply source.
    struct Faucet has key, store {
        id: UID,
        supply: Supply<MOCK>,
    }

    /// Bootstrap the faucet: transfers it to the transaction sender.
    public entry fun create_faucet(ctx: &mut TxContext) {
        let faucet = Faucet {
            id: object::new(ctx),
            supply: balance::new_supply(MOCK {}),
        };
        object::save_object(&faucet);
        transfer::public_transfer(faucet, tx_context::sender(ctx));
    }

    /// Faucet: mint `amount` of `MOCK` to the transaction sender. Value is
    /// backed by the faucet's owned supply handle.
    public entry fun mint(faucet: &mut Faucet, amount: u64, ctx: &mut TxContext) {
        let balance = balance::increase_supply(&mut faucet.supply, amount);
        let coin = coin::from_balance(balance, ctx);
        object::save_object(faucet);
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
