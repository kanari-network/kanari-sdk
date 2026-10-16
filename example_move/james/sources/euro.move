module james::euro {
    use kanari_system::coin;
    use kanari_system::coin::{Coin, TreasuryCap};
    use kanari_system::deny_list::{DenyCap, DenyList};
    use kanari_system::pay;
    use kanari_system::tx_context::{TxContext};
    use std::option;
    use kanari_system::transfer;
    use kanari_system::url;

    /// Name of the coin
    struct EURO has drop {}

    #[allow(unused_function)]
    /// Initialize and register the EURO currency as a *regulated* coin.
    ///
    /// Runs exactly once: the runtime invokes `init` on fresh publish only
    /// (never on upgrade), with `tx_context::sender` set to the publisher,
    /// so TreasuryCap, DenyCap and metadata land with the deployer and no
    /// replayable setup entry needs to exist.
    fun init(witness: EURO, ctx: &mut TxContext) {
        let (treasury, deny_cap, metadata) =
            coin::create_regulated_currency<EURO>(
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
        transfer::public_transfer(deny_cap, sender);
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
        ctx: &mut TxContext,
    ) {
        // Mint a new Coin with the specified amount
        let coin = coin::mint<EURO>(treasury_cap, amount, ctx);
        // Transfer the Coin to the recipient
        // The runtime will merge Coins of the same type automatically
        transfer::public_transfer(coin, recipient);
    }

    /// Transfer a specific `amount` of EURO from a mutable Coin held by the caller.
    /// Regulated path: aborts when `recipient` is on the EURO deny list or
    /// EURO is globally paused. Takes the shared `DenyList` singleton
    /// (`@0x403`) as a read-only reference.
    /// Usage: provide the caller's coin, the amount to send, the recipient,
    /// the shared deny-list object, and the tx context.
    public entry fun transfer_amount(
        c: &mut coin::Coin<EURO>,
        amount: u64,
        recipient: address,
        deny_list: &DenyList,
        ctx: &mut TxContext,
    ) {
        // 1. Check if sender is the same as recipient, if so, do nothing
        let sender = kanari_system::tx_context::sender(ctx);
        // 2. sender is not the same as recipient, proceed with the checked transfer
        if (sender == recipient) { return };
        pay::split_and_transfer_checked(c, amount, recipient, deny_list, ctx);
    }

    /// Burn a specific `amount` of THB from a mutable Coin held by the caller
    /// Usage: provide the TreasuryCap, a mutable coin owned by caller, amount to burn, and tx context
    public entry fun burn_amount(
        treasury_cap: &mut TreasuryCap<EURO>,
        c: &mut Coin<EURO>,
        amount: u64,
        ctx: &mut TxContext,
    ) {
        let to_burn = coin::split(c, amount, ctx);
        let _burned = coin::burn(treasury_cap, to_burn);
    }

    // ==========================================
    // Deny-list management (regulated coin)
    // ==========================================
    /// Deny-list operations are gated by `&DenyCap<EURO>` minted in `init`,
    /// so only the deployer (or whoever holds the cap) can maintain entries.

    /// Add `addr` to the EURO deny list.
    public entry fun deny_add(
        deny_list: &mut DenyList,
        cap: &DenyCap<EURO>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_add(deny_list, cap, addr, ctx);
    }

    /// Remove `addr` from the EURO deny list. Aborts when not denied.
    public entry fun deny_remove(
        deny_list: &mut DenyList,
        cap: &DenyCap<EURO>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_remove(deny_list, cap, addr, ctx);
    }

    /// Pause all regulated EURO flows (fail-closed kill-switch).
    public entry fun deny_pause(
        deny_list: &mut DenyList,
        cap: &DenyCap<EURO>,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_pause(deny_list, cap, ctx);
    }

    /// Lift the EURO pause.
    public entry fun deny_unpause(
        deny_list: &mut DenyList,
        cap: &DenyCap<EURO>,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_unpause(deny_list, cap, ctx);
    }

    /// Open read: true iff `addr` is denied for EURO.
    public fun is_denied(deny_list: &DenyList, addr: address): bool {
        coin::deny_list_contains<EURO>(deny_list, addr)
    }

    /// Open read: true iff EURO is globally paused.
    public fun is_paused(deny_list: &DenyList): bool {
        coin::deny_list_paused<EURO>(deny_list)
    }

    #[test_only]
    /// Mint a fresh regulated EURO currency for tests (the real `init` runs
    /// only on publish, so tests build the same objects directly).
    public fun create_regulated_for_test(
        ctx: &mut TxContext,
    ): (TreasuryCap<EURO>, DenyCap<EURO>, coin::CoinMetadata<EURO>) {
        coin::create_regulated_currency(
            EURO {},
            6,
            b"EURO",
            b"EURO Token",
            b"",
            option::none(),
            ctx,
        )
    }
}
