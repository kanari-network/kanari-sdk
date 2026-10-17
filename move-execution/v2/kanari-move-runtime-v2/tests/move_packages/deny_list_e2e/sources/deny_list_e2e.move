/// Thin entry/view wrappers over `kanari_system::deny_list` so the runtime can
/// drive the shared deny-list singleton (`@0x403`) through entry/view
/// execution. `deny_list`'s own functions are `public` (not `entry`) because the
/// `coin` module calls them internally; wrapping keeps the runtime test focused
/// on persistence rather than dispatch details.
module tester::deny_list_e2e {
    use kanari_system::coin;
    use kanari_system::deny_list::{Self, DenyCap, DenyList};
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;

    struct TEST_COIN has drop {}

    public entry fun create_cap(ctx: &mut TxContext) {
        let cap = deny_list::new_denycap<TEST_COIN>(ctx);
        transfer::public_transfer(cap, kanari_system::tx_context::sender(ctx));
    }

    public entry fun add_with_cap(
        dl: &mut DenyList,
        cap: &DenyCap<TEST_COIN>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_add(dl, cap, addr, ctx);
    }

    public entry fun add(
        dl: &mut DenyList,
        per_type_index: u64,
        ty: vector<u8>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        deny_list::v1_add(dl, per_type_index, ty, addr, ctx);
    }

    public entry fun remove(
        dl: &mut DenyList,
        per_type_index: u64,
        ty: vector<u8>,
        addr: address,
    ) {
        deny_list::v1_remove(dl, per_type_index, ty, addr);
    }

    public entry fun pause(
        dl: &mut DenyList,
        per_type_index: u64,
        ty: vector<u8>,
        ctx: &mut TxContext,
    ) {
        deny_list::v2_enable_global_pause(dl, per_type_index, ty, ctx);
    }

    public entry fun unpause(
        dl: &mut DenyList,
        per_type_index: u64,
        ty: vector<u8>,
    ) {
        deny_list::v2_disable_global_pause(dl, per_type_index, ty);
    }

    public fun contains(
        dl: &DenyList,
        per_type_index: u64,
        ty: vector<u8>,
        addr: address,
    ): bool {
        deny_list::v1_contains(dl, per_type_index, ty, addr)
    }

    public fun paused(
        dl: &DenyList,
        per_type_index: u64,
        ty: vector<u8>,
    ): bool {
        deny_list::v2_is_global_pause_enabled(dl, per_type_index, ty)
    }

    /// Singleton-based read: resolves the shared list without a declared
    /// object input. Proves the runtime preloads `@0x403` for every
    /// execution (the host-side half of the clean entry API).
    public fun contains_auto(
        per_type_index: u64,
        ty: vector<u8>,
        addr: address,
    ): bool {
        deny_list::v1_contains(
            deny_list::borrow_singleton(),
            per_type_index,
            ty,
            addr,
        )
    }

    public fun paused_auto(per_type_index: u64, ty: vector<u8>): bool {
        deny_list::v2_is_global_pause_enabled(
            deny_list::borrow_singleton(),
            per_type_index,
            ty,
        )
    }

    /// Singleton-based entry guard: aborts when `addr` is denied. Proves the
    /// entry (not just view) execution path preloads the singleton.
    public entry fun assert_not_denied(
        per_type_index: u64,
        ty: vector<u8>,
        addr: address,
    ) {
        assert!(
            !deny_list::v1_contains(
                deny_list::borrow_singleton(),
                per_type_index,
                ty,
                addr,
            ),
            1,
        );
    }
}
