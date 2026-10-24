// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// KRC-20 is Kanari's fungible-token standard.
///
/// A token is identified by its Move witness type `T`; its value lives in
/// `coin::Coin<T>`, and mint authority lives exclusively in
/// `coin::TreasuryCap<T>`. This module deliberately adds no parallel balance
/// or supply ledger: `coin` is the canonical implementation, including its
/// one-currency-per-witness check, overflow checks, and object ownership.
module kanari_system::krc_20 {
    use std::option::Option;
    use kanari_system::coin::{Self, Coin, CoinMetadata, DenyCapV2, TreasuryCap};
    use kanari_system::deny_list::DenyList;
    use kanari_system::pay;
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::url::Url;

    /// Sender, recipient, or token type is blocked by its deny-list policy.
    const E_DENIED_OR_PAUSED: u64 = 0;

    /// Create the canonical KRC-20 currency for witness `T`.
    /// `coin::create_currency` claims the witness before returning, so no
    /// second treasury cap can be created for the same token type.
    public fun create<T: drop>(
        witness: T,
        decimals: u8,
        symbol: vector<u8>,
        name: vector<u8>,
        description: vector<u8>,
        icon_url: Option<Url>,
        ctx: &mut TxContext
    ): (TreasuryCap<T>, CoinMetadata<T>) {
        coin::create_currency(
            witness,
            decimals,
            symbol,
            name,
            description,
            icon_url,
            ctx
        )
    }

    /// Create a regulated KRC-20 token with an immediate deny list and a
    /// global-pause capability. Transfers use `transfer_checked` for enforcement.
    public fun create_regulated<T: drop>(
        witness: T,
        decimals: u8,
        symbol: vector<u8>,
        name: vector<u8>,
        description: vector<u8>,
        icon_url: Option<Url>,
        allow_global_pause: bool,
        ctx: &mut TxContext
    ): (TreasuryCap<T>, DenyCapV2<T>, CoinMetadata<T>) {
        coin::create_regulated_currency_v2(
            witness,
            decimals,
            symbol,
            name,
            description,
            icon_url,
            allow_global_pause,
            ctx
        )
    }

    /// Mint a coin object. Only a holder of the token's treasury cap can mint.
    public fun mint<T>(
        cap: &mut TreasuryCap<T>, amount: u64, ctx: &mut TxContext
    ): Coin<T> {
        coin::mint(cap, amount, ctx)
    }

    /// Mint directly to an address. This is the issuer's airdrop primitive.
    public entry fun mint_to<T>(
        cap: &mut TreasuryCap<T>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        coin::mint_and_transfer(cap, amount, recipient, ctx)
    }

    /// Burn a complete coin object and reduce total supply.
    public fun burn<T>(cap: &mut TreasuryCap<T>, token: Coin<T>): u64 {
        coin::burn(cap, token)
    }

    /// Send an amount by splitting the sender-owned coin object.
    ///
    /// The canonical KRC-20 path always consults the singleton deny list. For
    /// ordinary tokens the lookup is empty and has no effect; regulated tokens
    /// reject a denied sender, a denied recipient, or a global pause.
    public entry fun transfer<T>(
        token: &mut Coin<T>, amount: u64, recipient: address, ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        assert!(!coin::is_denied<T>(sender), E_DENIED_OR_PAUSED);
        assert!(!coin::is_denied<T>(recipient), E_DENIED_OR_PAUSED);
        assert!(!coin::is_paused<T>(), E_DENIED_OR_PAUSED);
        pay::split_and_transfer(token, amount, recipient, ctx)
    }

    /// Regulated KRC-20 transfer. Both sender and recipient are checked
    /// against `deny`; a globally paused token also aborts.
    public entry fun transfer_checked<T>(
        token: &mut Coin<T>,
        amount: u64,
        recipient: address,
        deny: &DenyList,
        ctx: &mut TxContext
    ) {
        pay::split_and_transfer_checked(token, amount, recipient, deny, ctx)
    }

    /// Merge `other` into `token`, reducing coin-object fragmentation.
    public entry fun join<T>(token: &mut Coin<T>, other: Coin<T>) {
        coin::join(token, other)
    }

    /// Current base-unit balance of one coin object.
    public fun value<T>(token: &Coin<T>): u64 {
        coin::value(token)
    }

    /// Current circulating supply in base units.
    public fun total_supply<T>(cap: &TreasuryCap<T>): u64 {
        coin::total_supply(cap)
    }

    #[test_only]
    struct TEST has drop {}

    #[test]
    fun test_create_mint_and_burn_conserves_supply() {
        let ctx = kanari_system::tx_context::dummy();
        let (cap, metadata) =
            create(
                TEST {},
                6,
                b"TEST",
                b"Test Token",
                b"",
                std::option::none(),
                &mut ctx
            );
        let token = mint(&mut cap, 1_000, &mut ctx);
        assert!(value(&token) == 1_000, 0);
        assert!(total_supply(&cap) == 1_000, 1);
        assert!(burn(&mut cap, token) == 1_000, 2);
        assert!(total_supply(&cap) == 0, 3);
        kanari_system::transfer::public_freeze_object(cap);
        kanari_system::transfer::public_freeze_object(metadata);
    }
}

