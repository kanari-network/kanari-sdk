/// Coin<KARI> is the token used to pay for gas in Kari.
/// It has 9 decimals, and the smallest unit (10^-9) is called "mist".
module kanari_framework::kari {
    use std::option;
    use kanari_framework::tx_context::{Self, TxContext};
    use kanari_framework::balance::{Self, Balance};
    use kanari_framework::transfer;
    use kanari_framework::coin::{Self, Coin, TreasuryCap};

    const EAlreadyMinted: u64 = 0;
    /// Sender is not @0x0 the system address.
    const ENotSystemAddress: u64 = 1;

    #[allow(unused_const)]
    /// The amount of Mist per Kari token based on the the fact that mist is
    /// 10^-9 of a Kari token
    const MIST_PER_KARI: u64 = 1_000_000_000;

    #[allow(unused_const)]
    /// The total supply of Kari denominated in whole Kari tokens (200 Million)
    const TOTAL_SUPPLY_KARI: u64 = 100_000_000;

    /// The total supply of Kari denominated in Mist (200 Million * 10^9)
    const TOTAL_SUPPLY_MIST: u64 = 100_000_000_000_000_000;

    /// Name of the coin
    struct KARI has drop {}

    #[allow(unused_function)]
    // Register the `KARI` Coin to acquire its `Supply`.
    // This should be called only once during genesis creation.
    fun new(ctx: &mut TxContext): Balance<KARI> {
        assert!(tx_context::sender(ctx) == @0x0, ENotSystemAddress);
        assert!(tx_context::epoch(ctx) == 0, EAlreadyMinted);

        let (treasury, metadata) = coin::create_currency(
            KARI {},
            9,
            b"KARI",
            b"Kanara Network Coin",
            // TODO: add appropriate description and logo url
            b"",
            option::none(),
            ctx
        );
        transfer::public_freeze_object(metadata);
        let supply = coin::treasury_into_supply(treasury);
        let total_kari = balance::increase_supply(&mut supply, TOTAL_SUPPLY_MIST);
        balance::destroy_supply(supply);
        total_kari
    }

    /// KARI tokens to the treasury
    public entry fun transfer(c: coin::Coin<KARI>, recipient: address) {
        transfer::public_transfer(c, recipient)
    }

    /// Burns KARI tokens, decreasing total supply
    public entry fun burn(treasury_cap: &mut TreasuryCap<KARI>, coin: Coin<KARI>) {
        coin::burn(treasury_cap, coin);
    }
}