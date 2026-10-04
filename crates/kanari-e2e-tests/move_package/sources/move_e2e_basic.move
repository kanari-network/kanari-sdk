module kanari_e2e_tests::move_e2e_basic {
    use std::ascii;
    use std::string;
    use kanari_system::coin::{Self, CoinMetadata};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::url;

    const E_BAD_SUPPLY: u64 = 20;
    const E_BAD_MINT: u64 = 21;

    /// Witness type for the E2E test coin.
    public struct TEST_COIN has drop {}

    /// Create the TEST_COIN currency and fund the transaction sender.
    public entry fun init_test_coin(ctx: &mut TxContext) {
        let (mut treasury_cap, metadata) = e2e_create(ctx);
        let sender = tx_context::sender(ctx);
        let minted = coin::mint<TEST_COIN>(&mut treasury_cap, 1000000000000, ctx);
        transfer::share_object(treasury_cap);
        transfer::share_object(metadata);
        transfer::public_transfer(minted, sender);
    }

    /// Mint more TEST_COIN from the shared treasury cap.
    public entry fun mint_more(
        cap: &mut coin::TreasuryCap<TEST_COIN>, amount: u64, ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        transfer::public_transfer(
            coin::mint<TEST_COIN>(cap, amount, ctx),
            sender
        );
    }

    /// Total supply tracked by the treasury cap.
    public fun total_supply(cap: &coin::TreasuryCap<TEST_COIN>): u64 {
        coin::total_supply(cap)
    }

    /// Creates the TEST_COIN currency with the canonical E2E parameters.
    fun e2e_create(ctx: &mut TxContext): (coin::TreasuryCap<TEST_COIN>, CoinMetadata<TEST_COIN>) {
        coin::create_currency<TEST_COIN>(
            TEST_COIN {},
            9,
            b"TEST",
            b"Test Coin",
            b"Kanari E2E test coin",
            option::none<url::Url>(),
            ctx
        )
    }

    /// Full currency lifecycle: create, mint, verify supply, mint again, burn.
    public entry fun e2e_coin_lifecycle(ctx: &mut TxContext) {
        let (mut treasury_cap, mut metadata) = e2e_create(ctx);

        assert!(
            coin::total_supply<TEST_COIN>(&treasury_cap) == 0,
            E_BAD_SUPPLY
        );

        let first = coin::mint<TEST_COIN>(&mut treasury_cap, 1000000000000, ctx);
        assert!(
            coin::total_supply<TEST_COIN>(&treasury_cap) == 1000000000000,
            E_BAD_SUPPLY
        );
        assert!(coin::value(&first) == 1000000000000, E_BAD_MINT);

        let second = coin::mint<TEST_COIN>(&mut treasury_cap, 7, ctx);
        assert!(
            coin::total_supply<TEST_COIN>(&treasury_cap) == 1000000000007,
            E_BAD_SUPPLY
        );

        // Split, join and zero must all preserve the aggregate value.
        let mut first_mut = first;
        let split_off = coin::split(&mut first_mut, 400000000000, ctx);
        coin::join(&mut first_mut, split_off);
        assert!(coin::value(&first_mut) == 1000000000000, E_BAD_MINT);

        // Burning reduces the recorded total supply.
        let burned = coin::burn<TEST_COIN>(&mut treasury_cap, second);
        assert!(burned == 7, E_BAD_MINT);
        assert!(
            coin::total_supply<TEST_COIN>(&treasury_cap) == 1000000000000,
            E_BAD_SUPPLY
        );

        // Metadata is mutable through the canonical setters.
        coin::update_name<TEST_COIN>(&treasury_cap, &mut metadata, string::utf8(b"Renamed"));
        coin::update_symbol<TEST_COIN>(&treasury_cap, &mut metadata, ascii::string(b"REN"));

        let sender = tx_context::sender(ctx);
        transfer::share_object(treasury_cap);
        transfer::share_object(metadata);
        transfer::public_transfer(first_mut, sender);
    }
}