// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// USD stablecoin for the Kanari network (`0x2::usd::USD`).
///
/// - 6 decimals (USDC-style); the smallest unit is 10^-6 USD.
/// - Whitelisted for gas settlement (see `kanari_system::gas_market` and the
///   `GasPriceTable` client default of $1.00).
/// - Permissionless mint: the `TreasuryCap` is a shared object, so anyone
///   can call `mint` with their own wallet (paying their own gas). No fixed
///   supply, no dev allocation, no custodian key or password.
///
/// Anyone with a funded wallet mints with one call, e.g.:
/// `kanari move call --package 0x2 --module usd --function mint \
///   --args <treasury_cap_object_id> 100000000 <recipient>`
module kanari_system::usd {
    use kanari_system::coin;
    use kanari_system::coin::{Coin, TreasuryCap};
    use kanari_system::tx_context::{Self, TxContext};
    use std::option;
    use kanari_system::transfer;
    use kanari_system::url;

    /// Base units per whole USD (6 decimals).
    const UNITS_PER_USD: u64 = 1_000_000;

    /// Name of the coin
    struct USD has drop {}

    #[allow(unused_function)]
    // Register the `USD` coin and share the mint authority so anyone can
    // mint. Runs once at genesis (see `usd::init` dispatch in genesis).
    fun init(witness: USD, ctx: &mut TxContext) {
        let (treasury, metadata) =
            coin::create_currency(
                witness,
                6,
                b"USD",
                b"Kanari USD Stablecoin",
                b"Permissionless USD: anyone can mint via usd::mint with the shared TreasuryCap",
                option::some(
                    url::new_unsafe_from_bytes(
                        b"https://avatars.githubusercontent.com/u/127471673?s=200&v=4"
                    )
                ),
                ctx
            );
        transfer::public_freeze_object(metadata);

        // The mint authority is shared, not held: no key, no password.
        transfer::share_object(treasury);
    }

    /// Mint new USD to `recipient`. Permissionless: pass the shared
    /// `TreasuryCap<USD>` (discoverable via object query by type).
    public entry fun mint(
        cap: &mut TreasuryCap<USD>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        let minted = coin::mint(cap, amount, ctx);
        transfer::public_transfer(minted, recipient);
    }

    /// Transfer a specific `amount` of USD from a mutable Coin held by the caller.
    public entry fun transfer_amount(
        c: &mut coin::Coin<USD>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        if (sender == recipient) { return };
        let split_coin = coin::split(c, amount, ctx);
        transfer::public_transfer(split_coin, recipient);
    }

    /// Burn a specific `amount` of USD from a mutable Coin held by the caller.
    public entry fun burn_amount(
        cap: &mut TreasuryCap<USD>,
        c: &mut Coin<USD>,
        amount: u64,
        ctx: &mut TxContext
    ) {
        let to_burn = coin::split(c, amount, ctx);
        let _burned = coin::burn(cap, to_burn);
    }

    public fun units_per_usd(): u64 {
        UNITS_PER_USD
    }
}

