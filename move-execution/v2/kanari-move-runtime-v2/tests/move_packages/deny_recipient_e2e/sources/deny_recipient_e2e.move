/// Destination-side deny enforcement fixture (`deny_list_recipient` test).
///
/// The transfer performed by `split_then_transfer` is the deliberately
/// unchecked plain path (`coin::split` + `transfer::public_transfer`, no
/// deny reads) — the runtime's post-execution output policy must catch a
/// denied recipient. Other entries set-up a regulated currency and drive
/// the deny list through the production `coin` wrappers.
module tester::deny_recipient_e2e {
    use std::option;
    use kanari_system::coin::{Self, Coin, TreasuryCap};
    use kanari_system::deny_list::{DenyCap, DenyList};
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    use kanari_system::url;
    use std::vector;

    struct TOKEN has drop {}

    /// Create a regulated currency; mints the `DenyCap<TOKEN>` the writes
    /// below require. The only cap-minting path.
    public entry fun create_currency(ctx: &mut TxContext) {
        let sender = kanari_system::tx_context::sender(ctx);
        let (treasury, cap, metadata) = coin::create_regulated_currency<TOKEN>(
            TOKEN {},
            9,
            b"TOKEN",
            b"Recipient E2E Token",
            b"deny recipient enforcement fixture",
            option::none<url::Url>(),
            ctx,
        );
        transfer::public_transfer(treasury, sender);
        transfer::public_transfer(metadata, sender);
        transfer::public_transfer(cap, sender);
    }

    public entry fun mint(treasury: &mut TreasuryCap<TOKEN>, amount: u64, ctx: &mut TxContext) {
        let sender = kanari_system::tx_context::sender(ctx);
        let c = coin::mint(treasury, amount, ctx);
        transfer::public_transfer(c, sender);
    }

    public entry fun add_denied(
        dl: &mut DenyList,
        cap: &DenyCap<TOKEN>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_add(dl, cap, addr, ctx);
    }

    /// Deny a whole batch in one transaction. Used by the capacity test to
    /// grow the list large enough that the runtime's inner policy-read
    /// budget (`DENY_POLICY_GAS_BUDGET`) is a real constraint, not trivia.
    public entry fun add_denied_many(
        dl: &mut DenyList,
        cap: &DenyCap<TOKEN>,
        addrs: vector<address>,
        ctx: &mut TxContext,
    ) {
        let i = 0;
        let n = vector::length(&addrs);
        while (i < n) {
            coin::deny_list_add(dl, cap, *vector::borrow(&addrs, i), ctx);
            i = i + 1;
        };
    }

    public entry fun pause(dl: &mut DenyList, cap: &DenyCap<TOKEN>, ctx: &mut TxContext) {
        coin::deny_list_pause(dl, cap, ctx);
    }

    public entry fun unpause(dl: &mut DenyList, cap: &DenyCap<TOKEN>, ctx: &mut TxContext) {
        coin::deny_list_unpause(dl, cap, ctx);
    }

    /// Deliberately unchecked plain transfer path.
    public entry fun split_then_transfer(
        coin: &mut Coin<TOKEN>,
        amount: u64,
        to: address,
        ctx: &mut TxContext,
    ) {
        let piece = coin::split(coin, amount, ctx);
        transfer::public_transfer(piece, to);
    }
}
