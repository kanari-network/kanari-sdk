/// Negative fixture for `deny_list_friend_locked_is_rejected`.
///
/// `kanari_system::deny_list`'s mutators are `public(friend)` with
/// `kanari_system::coin` as the sole friend, so no outside package may reach
/// them. This module calls them directly with a raw
/// `(per_type_index, type_bytes)` pair and no `DenyCap<T>` — exactly the
/// bypass the restriction exists to block.
///
/// Capability forgery is covered separately by the `deny_list_cap_forgery`
/// fixture; keeping the two apart means each test names the exact control
/// that was lost.
///
/// This package MUST NOT compile.
module tester::deny_list_friend_violation {
    use kanari_system::deny_list::{Self, DenyList};
    use kanari_system::tx_context::TxContext;

    public entry fun deny_without_cap(
        dl: &mut DenyList,
        per_type_index: u64,
        `type`: vector<u8>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        deny_list::v1_add(dl, per_type_index, `type`, addr, ctx);
    }

    public entry fun undeny_without_cap(
        dl: &mut DenyList,
        per_type_index: u64,
        `type`: vector<u8>,
        addr: address,
    ) {
        deny_list::v1_remove(dl, per_type_index, `type`, addr);
    }

    public entry fun pause_without_cap(
        dl: &mut DenyList,
        per_type_index: u64,
        `type`: vector<u8>,
        ctx: &mut TxContext,
    ) {
        deny_list::v2_enable_global_pause(dl, per_type_index, `type`, ctx);
    }

    public entry fun unpause_without_cap(
        dl: &mut DenyList,
        per_type_index: u64,
        `type`: vector<u8>,
    ) {
        deny_list::v2_disable_global_pause(dl, per_type_index, `type`);
    }
}
