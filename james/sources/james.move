module james::james {

    use kanari_system::coin;
    use kanari_system::coin::{Coin, TreasuryCap};
    use kanari_system::tx_context::{TxContext};

    use std::option;
    use kanari_system::transfer;

    /// Name of the coin
    struct JAMES has drop {}

    /// Initialize and register the JAMES currency.
    /// Returns the `TreasuryCap<JAMES>` which can be used to mint tokens.
    /// This should be invoked once (e.g., during genesis or deployment).
    fun init(witness: JAMES ,ctx: &mut TxContext): (TreasuryCap<JAMES>, coin::CoinMetadata<JAMES>) {
        let (treasury, metadata) = coin::create_currency<JAMES>(
            witness,
            9,
            b"JAMES",
            b"James Token",
            b"",
            option::none<kanari_system::url::Url>(),
            ctx,
        );
        // Return both TreasuryCap and Metadata so callers can persist them.
        (treasury, metadata)
    }

    /// Public setup entry that creates the required `JAMES` witness,
    /// invokes `init`, and transfers the created objects to the
    /// transaction sender so they are persisted in the caller's account.
    public entry fun setup(ctx: &mut TxContext) {
        let witness = JAMES {};
        let (treasury, metadata) = init(witness, ctx);
        let sender = kanari_system::tx_context::sender(ctx);
        transfer::public_transfer(treasury, sender);
        transfer::public_transfer(metadata, sender);
    }


    /// Mint new JAMES tokens
    /// Only the holder of TreasuryCap can call this
    /// Usage: kanari move call --function mint --args <amount> <recipient>
    public entry fun mint(
        treasury_cap: &mut TreasuryCap<JAMES>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        // Use `mint_and_transfer` to mint and credit the recipient in one
        // operation. This updates the runtime token balances as expected.
        coin::mint_and_transfer<JAMES>(treasury_cap, amount, recipient, ctx);
    }


    /// Transfer a specific `amount` of JAMES from a mutable Coin held by the caller
    /// Usage: provide the caller's coin, the amount to send, and the recipient
    public entry fun transfer_amount(
        c: &mut coin::Coin<JAMES>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        let split_coin = coin::split(c, amount, ctx);
        transfer::public_transfer(split_coin, recipient);
    }


    /// Burn a specific `amount` of JAMES from a mutable Coin held by the caller
    /// Usage: provide the TreasuryCap, a mutable coin owned by caller, amount to burn, and tx context
    public entry fun burn_amount(
        treasury_cap: &mut TreasuryCap<JAMES>,
        c: &mut Coin<JAMES>,
        amount: u64,
        ctx: &mut TxContext
    ) {
        let to_burn = coin::split(c, amount, ctx);
        let _burned = coin::burn(treasury_cap, to_burn);
    }
}
