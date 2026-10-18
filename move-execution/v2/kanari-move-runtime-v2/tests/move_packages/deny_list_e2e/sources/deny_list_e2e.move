/// Thin entry/view wrappers over `kanari_system::deny_list` so the runtime can
/// drive the shared deny-list singleton (`@0x403`) through entry/view
/// execution. `deny_list`'s mutators and `DenyCap` minting are
/// `public(friend)` (coin-only), so this module reaches writes through the
/// production path: `coin::create_regulated_currency` mints the `DenyCap`,
/// then the `DenyCap`-gated `coin` wrappers perform the write. Reads stay
/// `public` and are called directly.
module tester::deny_list_e2e {
    use std::option;
    use kanari_system::coin;
    use kanari_system::deny_list::{Self, DenyCap, DenyList};
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    use kanari_system::url;

    struct TEST_COIN has drop {}

    /// Create a regulated currency, which mints the `DenyCap<TEST_COIN>` that
    /// every write below requires. This is the only cap-minting path.
    public entry fun create_cap(ctx: &mut TxContext) {
        let sender = kanari_system::tx_context::sender(ctx);
        let (treasury, cap, metadata) = coin::create_regulated_currency<TEST_COIN>(
            TEST_COIN {},
            9,
            b"TEST",
            b"Test Coin",
            b"Deny-list e2e fixture",
            option::none<url::Url>(),
            ctx,
        );
        transfer::public_transfer(treasury, sender);
        transfer::public_transfer(metadata, sender);
        transfer::public_transfer(cap, sender);
    }

    public entry fun add(
        dl: &mut DenyList,
        cap: &DenyCap<TEST_COIN>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_add(dl, cap, addr, ctx);
    }

    public entry fun remove(
        dl: &mut DenyList,
        cap: &DenyCap<TEST_COIN>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_remove(dl, cap, addr, ctx);
    }

    public entry fun pause(
        dl: &mut DenyList,
        cap: &DenyCap<TEST_COIN>,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_pause(dl, cap, ctx);
    }

    public entry fun unpause(
        dl: &mut DenyList,
        cap: &DenyCap<TEST_COIN>,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_unpause(dl, cap, ctx);
    }

    public fun contains(dl: &DenyList, addr: address): bool {
        coin::deny_list_contains<TEST_COIN>(dl, addr)
    }

    public fun paused(dl: &DenyList): bool {
        coin::deny_list_paused<TEST_COIN>(dl)
    }

    /// Singleton-based read: resolves the shared list without a declared
    /// object input. Proves the runtime preloads `@0x403` for every
    /// execution (the host-side half of the clean entry API).
    public fun contains_auto(addr: address): bool {
        coin::is_denied<TEST_COIN>(addr)
    }

    public fun paused_auto(): bool {
        coin::is_paused<TEST_COIN>()
    }

    /// Singleton-based entry guard: aborts when `addr` is denied. Proves the
    /// entry (not just view) execution path preloads the singleton.
    public entry fun assert_not_denied(addr: address) {
        assert!(!coin::is_denied<TEST_COIN>(addr), 1);
    }
}
