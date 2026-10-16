/// Thin entry/view wrappers over `kanari_system::deny_list` so the runtime can
/// drive the shared deny-list singleton (`@0x403`) through entry/view
/// execution. `deny_list`'s own functions are `public` (not `entry`) because the
/// `coin` module calls them internally; wrapping keeps the runtime test focused
/// on persistence rather than dispatch details.
module tester::deny_list_e2e {
    use kanari_system::deny_list::{Self, DenyList};
    use kanari_system::tx_context::TxContext;

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
}
