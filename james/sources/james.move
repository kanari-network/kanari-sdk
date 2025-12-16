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
    public entry fun init(witness: JAMES ,ctx: &mut TxContext): TreasuryCap<JAMES> {
        let (treasury, metadata) = coin::create_currency<JAMES>(
            witness,
            9,
            b"JAMES",
            b"James Token",
            b"",
            option::none<kanari_system::url::Url>(),
            ctx,
        );
        transfer::public_freeze_object(metadata);
        treasury
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
        let coin = coin::mint(treasury_cap, amount, ctx);
        transfer::public_transfer(coin, recipient);
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
