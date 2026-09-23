module james::euro {

    use kanari_system::coin;
    use kanari_system::coin::{Coin, TreasuryCap};
    use kanari_system::tx_context::{TxContext};

    use std::option;
    use kanari_system::transfer;
    use kanari_system::url;

    /// Name of the coin
    struct EURO has drop {}

    #[allow(unused_function)]
    /// Initialize and register the EURO currency.
    ///
    /// Runs exactly once: the runtime invokes `init` on fresh publish only
    /// (never on upgrade), with `tx_context::sender` set to the publisher,
    /// so TreasuryCap and metadata land with the deployer and no replayable
    /// setup entry needs to exist.
    fun init(witness: EURO, ctx: &mut TxContext) {
        let (treasury, metadata) =
            coin::create_currency<EURO>(
                witness,
                6,
                b"EURO",
                b"EURO Token",
                b"",
                option::some(
                    url::new_unsafe_from_bytes(
                        b"https://assets.coingecko.com/coins/images/26045/standard/EURC.png?1769615705"
                    )
                ),
                ctx
            );
        let sender = kanari_system::tx_context::sender(ctx);
        transfer::public_transfer(treasury, sender);
        transfer::public_transfer(metadata, sender);
    }

    /// Mint new EURO tokens
    /// Only the holder of TreasuryCap can call this
    /// Usage: kanari move call --function mint --args <amount> <recipient>
    ///
    /// This function mints tokens directly to the recipient's address
    /// The runtime will automatically create or update the recipient's Coin object
    public entry fun mint(
        treasury_cap: &mut TreasuryCap<EURO>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        // Mint a new Coin with the specified amount
        let coin = coin::mint<EURO>(treasury_cap, amount, ctx);

        // Transfer the Coin to the recipient
        // The runtime will merge Coins of the same type automatically
        transfer::public_transfer(coin, recipient);
    }

    /// Transfer a specific `amount` of EURO from a mutable Coin held by the caller
    /// Usage: provide the caller's coin, the amount to send, and the recipient
    public entry fun transfer_amount(
        c: &mut coin::Coin<EURO>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        // 1. Check if sender is the same as recipient, if so, do nothing
        let sender = kanari_system::tx_context::sender(ctx);

        // 2. sender is not the same as recipient, proceed with transfer
        if (sender == recipient) { return };

        // 3. Split the specified amount from the sender's coin
        let split_coin = coin::split(c, amount, ctx);
        transfer::public_transfer(split_coin, recipient);
    }

    /// Burn a specific `amount` of THB from a mutable Coin held by the caller
    /// Usage: provide the TreasuryCap, a mutable coin owned by caller, amount to burn, and tx context
    public entry fun burn_amount(
        treasury_cap: &mut TreasuryCap<EURO>,
        c: &mut Coin<EURO>,
        amount: u64,
        ctx: &mut TxContext
    ) {
        let to_burn = coin::split(c, amount, ctx);
        let _burned = coin::burn(treasury_cap, to_burn);
    }
}

