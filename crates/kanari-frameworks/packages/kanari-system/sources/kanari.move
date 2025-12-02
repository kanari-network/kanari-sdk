/// Coin<KANARI> is the token used to pay for gas in KANARI.
/// It has 9 decimals, and the smallest unit (10^-9) is called "MIST".
module kanari_system::kanari {

    use kanari_system::coin;
    use kanari_system::coin::{Coin, TreasuryCap};
    use kanari_system::tx_context::{Self, TxContext};
    use std::string;
    use std::ascii;
    use std::option;
    use kanari_system::transfer;

    const EAlreadyMinted: u64 = 0;
    /// Sender is not @0x0 the system address.
    const ENotSystemAddress: u64 = 1;

    #[allow(unused_const)]
    /// The amount of Mist per Kanari token based on the fact that mist is
    /// 10^-9 of a Kanari token
    const MIST_PER_KANARI: u64 = 1_000_000_000;

    #[allow(unused_const)]
    /// The total supply of Kanari denominated in whole Kanari tokens (100 Million)
    const TOTAL_SUPPLY_KANARI: u64 = 100_000_000;

    /// The total supply of Kanari denominated in Mist (100 Million * 10^9)
    const TOTAL_SUPPLY_MIST: u64 = 100_000_000_000_000_000;

    /// Name of the coin
    struct KANARI has drop {}

    #[allow(unused_function)]
    // Register the `KANARI` Coin to acquire its `Supply`.
    // This should be called only once during genesis creation.
    // Mints the entire supply and transfers it to dev address @0x9.
    fun new(ctx: &mut TxContext): TreasuryCap<KANARI> {
        assert!(tx_context::sender(ctx) == @0x0, ENotSystemAddress);
        assert!(tx_context::epoch(ctx) == 0, EAlreadyMinted);

        let (treasury, metadata) = coin::create_currency(
            KANARI {},
            9,
            ascii::string(b"KANARI"),
            string::utf8(b"Kanari Network Coin"),
            string::utf8(b""),
            option::none(),
            ctx
        );
        transfer::public_freeze_object(metadata);

        // make a mutable binding for minting (use a different name than the original)
        let treasury_cap = treasury;

        // Mint the entire supply (in Mist) and transfer to dev @0x9
        let dev_address: address = @0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea;
        let minted_coin: Coin<KANARI> = coin::mint(&mut treasury_cap, TOTAL_SUPPLY_MIST, ctx);
        transfer::public_transfer(minted_coin, dev_address);

        // Return the treasury cap for further authorized minting if needed
        treasury_cap
    }


    /// KANARI tokens to the treasury
    public entry fun transfer(c: coin::Coin<KANARI>, recipient: address) {
        transfer::public_transfer(c, recipient)
    }

    /// Burns KANARI tokens, decreasing total supply
    public entry fun burn(treasury_cap: &mut TreasuryCap<KANARI>, coin: Coin<KANARI>) {
        coin::burn(treasury_cap, coin);
    }
}