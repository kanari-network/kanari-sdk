// Kanari Labs - KariKid Deny List
//
// The framework `kanari_system::deny_list` is coin-only: its mutators are
// `public(friend)` and `DenyCap<T>` can only be minted by
// `coin::create_regulated_currency`, so no outside package can ever gain
// write access to the shared `@0x403` registry.
//
// KariKid therefore keeps its own blocklist here, gated by the issuer's
// `collection::NftCap`. This is also the correct authority model: only the
// collection issuer decides who may receive its NFTs.
module james::kdeny {
    use kanari_system::collection;
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    use kanari_system::vec_set::{Self, VecSet};

    /// Abort code when removing an address that is not on the list.
    const ENOT_DENIED: u64 = 1;

    /// Shared, issuer-controlled blocklist for the KariKid family.
    struct DenyList has key, store {
        id: UID,
        denied: VecSet<address>,
        paused: bool,
    }

    /// Create and share the family deny list. Runs once at publication from
    /// `nft::init`; the resulting shared object id is what clients pass to
    /// `nft::transfer`.
    public fun create_shared(ctx: &mut TxContext) {
        transfer::share_object(DenyList {
            id: object::new(ctx),
            denied: vec_set::empty(),
            paused: false,
        });
    }

    #[test_only]
    /// Unshared list for unit tests, which run without a shared-object store.
    public fun new_for_testing(ctx: &mut TxContext): DenyList {
        DenyList {
            id: object::new(ctx),
            denied: vec_set::empty(),
            paused: false,
        }
    }

    /// Deny `addr`. Only the collection issuer (`&NftCap`) may write.
    public entry fun deny_add(
        list: &mut DenyList,
        _cap: &collection::NftCap,
        addr: address,
        _: &mut TxContext,
    ) {
        if (vec_set::contains(&list.denied, &addr)) return;
        vec_set::insert(&mut list.denied, addr);
    }

    /// Re-allow `addr`. Aborts `ENOT_DENIED` when not denied.
    public entry fun deny_remove(
        list: &mut DenyList,
        _cap: &collection::NftCap,
        addr: address,
    ) {
        assert!(vec_set::contains(&list.denied, &addr), ENOT_DENIED);
        vec_set::remove(&mut list.denied, &addr);
    }

    /// Fail-closed kill switch: block every KariKid transfer.
    public entry fun deny_pause(
        list: &mut DenyList,
        _cap: &collection::NftCap,
        _: &mut TxContext,
    ) {
        list.paused = true;
    }

    /// Lift the global pause.
    public entry fun deny_unpause(
        list: &mut DenyList,
        _cap: &collection::NftCap,
        _: &mut TxContext,
    ) {
        list.paused = false;
    }

    /// Open read: true iff `addr` may not receive a KariKid.
    public fun is_denied(list: &DenyList, addr: address): bool {
        vec_set::contains(&list.denied, &addr)
    }

    /// Open read: true iff every KariKid transfer is blocked.
    public fun is_paused(list: &DenyList): bool {
        list.paused
    }
}
