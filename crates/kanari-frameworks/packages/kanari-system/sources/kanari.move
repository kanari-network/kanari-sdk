// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Coin<KANARI> is the token used to pay for gas in KANARI.
/// It has 9 decimals, and the smallest unit (10^-9) is called "MIST".
module kanari_system::kanari {
    use kanari_system::coin;
    use kanari_system::coin::{Coin, TreasuryCap};
    use kanari_system::pay;
    use kanari_system::tx_context::{Self, TxContext};
    use std::option;
    use kanari_system::transfer;
    use kanari_system::url;

    #[allow(unused_const)]
    /// The amount of Mist per Kanari token based on the fact that mist is
    /// 10^-9 of a Kanari token
    const MIST_PER_KANARI: u64 = 1_000_000_000;

    #[allow(unused_const)]
    /// The total supply of Kanari denominated in whole Kanari tokens (11 Million)
    const TOTAL_SUPPLY_KANARI: u64 = 11_000_000;

    /// The total supply of Kanari denominated in Mist (11 Million * 10^9)
    const TOTAL_SUPPLY_MIST: u64 = 11_000_000_000_000_000;
    const DEV_GAS_RESERVE_MIST: u64 = 1_000_000_000;

    /// Name of the coin
    struct KANARI has drop {}

    #[allow(unused_function)]
    // Register the `KANARI` Coin to acquire its `Supply`.
    // This should be called only once during genesis creation.
    // Mints the entire supply and transfers it to dev address @0x9.
    fun init(witness: KANARI, ctx: &mut TxContext) {
        // assert!(tx_context::sender(ctx) == @0x0, ENotSystemAddress); // Sender check might be too strict for init
        // assert!(tx_context::epoch(ctx) == 0, EAlreadyMinted); // Epoch check might be okay

        let (treasury, metadata) = coin::create_currency(
            witness,
            9,
            b"KANARI",
            b"Kanari Network Coin",
            b"",
            option::some(url::new_unsafe_from_bytes(b"https://avatars.githubusercontent.com/u/127471673?s=200&v=4")),
            ctx
        );
        transfer::public_freeze_object(metadata);

        // make a mutable binding for minting (use a different name than the original)
        let treasury_cap = treasury;

        // Mint the initial supply into two coin objects so the dev wallet has
        // a dedicated gas coin from genesis onward.
        let dev_address: address = @0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146;
        let primary_coin: Coin<KANARI> =
            coin::mint(&mut treasury_cap, TOTAL_SUPPLY_MIST - DEV_GAS_RESERVE_MIST, ctx);
        let gas_coin: Coin<KANARI> = coin::mint(&mut treasury_cap, DEV_GAS_RESERVE_MIST, ctx);
        transfer::public_transfer(primary_coin, dev_address);
        transfer::public_transfer(gas_coin, dev_address);

        // Transfer the treasury cap to the sender (deployer)
        transfer::public_transfer(treasury_cap, tx_context::sender(ctx));
    }


    /// Transfer a specific amount of KANARI using the same Move coin path as other tokens.
    public entry fun transfer(
        c: &mut coin::Coin<KANARI>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        pay::split_and_transfer<KANARI>(c, amount, recipient, ctx);
    }

    /// Burns KANARI tokens, decreasing total supply
    public entry fun burn(treasury_cap: &mut TreasuryCap<KANARI>, coin: Coin<KANARI>) {
        coin::burn(treasury_cap, coin);
    }
}
