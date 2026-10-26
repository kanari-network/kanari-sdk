// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Sui-style deny list: a shared registry object that restricts addresses
/// from using instances of regulated coin types.
///
/// Deny entries live in a single shared `DenyList` object (one per network),
/// keyed by coin type, so they persist across transactions and are visible to
/// every participant. Writes are authorized by a `DenyCap<T>` minted by
/// `coin::create_regulated_currency`; membership reads are open to everyone.
///
/// Regulated transfer paths (`pay::split_and_transfer_checked`,
/// `pay::join_vec_and_transfer_checked`, `multisig::execute_transfer_checked`)
/// consult this list via `coin::deny_list_contains`.
module kanari_system::deny_list {
    // Only framework wrappers may mutate the registry. They derive the key
    // from a type or collection and gate writes behind the matching cap.
    // Without this, any package could deny-list an arbitrary
    // `(per_type_index, type_bytes)` pair by calling `v1_add`/`v1_remove`.
    friend kanari_system::coin;

    friend kanari_system::krc_721;

    use kanari_system::bag::{Self, Bag};
    use kanari_system::dynamic_object_field as ofield;
    use kanari_system::object::{Self, UID};
    use kanari_system::table::{Self, Table};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::vec_set::{Self, VecSet};

    /// `create` was not called by the system address.
    const ENotSystemAddress: u64 = 0;

    /// The address being removed is not on the list.
    const ENotDenied: u64 = 1;

    /// Reserved system addresses can never be denied.
    const EInvalidAddress: u64 = 2;

    /// Index into the deny-list bag for `coin::Coin` entries.
    const COIN_INDEX: u64 = 0;

    /// Well-known object id of the shared deny-list singleton, created at
    /// genesis by the runtime (Sui uses `0x403` for the same purpose).
    /// Transfer-time enforcement consults the list at this address.
    public fun deny_list_object_id(): address {
        @0x403
    }

    /// Borrow the shared singleton without receiving it as a transaction
    /// argument. The runtime preloads `@0x403` for every execution, so entry
    /// functions stay free of deny-list parameters. Aborts when the
    /// singleton is absent (e.g. the bare Move unit runner, which has no
    /// genesis state): unit tests should build a local list with
    /// `new_for_testing` and pass it explicitly instead.
    public fun borrow_singleton(): &DenyList {
        object::borrow_global<DenyList>(deny_list_object_id())
    }

    /// A shared object holding every per-type deny list. Like Sui, it uses
    /// both stores: the `lists` bag holds one `PerTypeList` per object
    /// family (fast `denied_count` skip table), while each per-type address
    /// set lives as a dynamic object field on this object's id, keyed by
    /// `(per_type_index, per_type_key)`.
    struct DenyList has key, store {
        id: UID,
        lists: Bag,
    }

    /// Deny data for one object family: counts of how many types each
    /// address is denied for. Lets `contains` skip the set lookup for the
    /// overwhelmingly common undenied case.
    struct PerTypeList has key, store {
        id: UID,
        denied_count: Table<address, u64>,
    }

    /// The address set denied for one `(per_type_index, per_type_key)`,
    /// e.g. `0x2::my_coin::MY_COIN`, plus that type's pause flag. Stored
    /// as a dynamic object field so each type's set stays a live object in
    /// storage. There is exactly one key type for these objects (`DenyKey`):
    /// this runtime keys dynamic fields by name *bytes*, so two key types
    /// with the same layout would silently collide.
    struct TypeDenySet has key, store {
        id: UID,
        addresses: VecSet<address>,
        paused: bool,
    }

    /// Dynamic-object-field key for a per-type set: essentially a
    /// `(per_type_index, per_type_key)` pair.
    struct DenyKey has copy, drop, store {
        per_type_index: u64,
        per_type_key: vector<u8>,
    }

    /// Capability minted by `coin::create_regulated_currency`. Possession of
    /// `&DenyCap<T>` authorizes writes to `T`'s deny entries.
    struct DenyCap<phantom T> has key, store, drop {
        id: object::UID,
    }

    /// Bag index reserved for coin deny entries (used by `coin.move`).
    public fun coin_index(): u64 {
        COIN_INDEX
    }

    /// Framework/system addresses (`@0x0`–`@0xF`) can never be denied.
    fun is_reserved(addr: address): bool {
        addr == @0x0
            || addr == @0x1
            || addr == @0x2
            || addr == @0x3
            || addr == @0x4
            || addr == @0x5
            || addr == @0x6
            || addr == @0x7
            || addr == @0x8
            || addr == @0x9
            || addr == @0xA
            || addr == @0xB
            || addr == @0xC
            || addr == @0xD
            || addr == @0xE
            || addr == @0xF
    }

    #[allow(unused_function)]
    /// Genesis-equivalent constructor for unit tests (dummy ctx sender is
    /// `@0x0`, so the system-address gate passes). Production networks get
    /// the singleton from runtime genesis at `deny_list_object_id()`.
    fun create(ctx: &mut TxContext) {
        assert!(tx_context::sender(ctx) == @0x0, ENotSystemAddress);
        let lists = bag::new(ctx);
        bag::add(&mut lists, COIN_INDEX, per_type_list(ctx));
        transfer::share_object(DenyList { id: object::new(ctx), lists });
    }

    fun per_type_list(ctx: &mut TxContext): PerTypeList {
        PerTypeList {
            id: object::new(ctx),
            denied_count: table::new(ctx),
        }
    }

    #[test_only]
    /// Genesis-equivalent constructor for unit tests (dummy ctx sender is
    /// `@0x0`, so the system-address gate passes).
    public fun create_for_test(ctx: &mut TxContext) {
        create(ctx);
    }

    #[test_only]
    /// Unshared list for unit tests that do not need the shared-object path.
    public fun new_for_testing(ctx: &mut TxContext): DenyList {
        let lists = bag::new(ctx);
        bag::add(&mut lists, COIN_INDEX, per_type_list(ctx));
        DenyList { id: object::new(ctx), lists }
    }

    /// Create a `DenyCap` while constructing a regulated currency.
    ///
    /// Friend-only on purpose: every write path requires `&DenyCap<T>`, so a
    /// publicly mintable capability would let any published package freeze
    /// arbitrary addresses for arbitrary coin types and bypass every check in
    /// `coin`. `coin::create_regulated_currency` is the only minting path.
    public(friend) fun new_denycap<T>(ctx: &mut TxContext): DenyCap<T> {
        DenyCap<T> { id: object::new(ctx) }
    }

    #[test_only]
    public fun new_denycap_for_testing<T>(ctx: &mut TxContext): DenyCap<T> {
        DenyCap<T> { id: object::new(ctx) }
    }

    /// Add `addr` to the deny set for `type` under `per_type_index`. The
    /// per-type set object is created on first use.
    ///
    /// Friend-only: callers reach this through coin or KRC-721 wrappers, which
    /// gate writes on their matching capability. Raw `(per_type_index, type)`
    /// pairs are never accepted from outside the framework.
    public(friend) fun v1_add(
        deny_list: &mut DenyList,
        per_type_index: u64,
        `type`: vector<u8>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        assert!(!is_reserved(addr), EInvalidAddress);
        // The genesis singleton is created with an empty `lists` bag (its
        // object id must be the well-known `deny_list_object_id()`, which
        // `create`'s `object::new` cannot produce); materialize the per-type
        // list on first use.
        if (!bag::contains(&deny_list.lists, per_type_index)) {
            bag::add(&mut deny_list.lists, per_type_index, per_type_list(ctx));
        };
        let key = DenyKey { per_type_index, per_type_key: `type` };
        if (ofield::exists_(&deny_list.id, key)) {
            let set: &mut TypeDenySet = ofield::borrow_mut(&mut deny_list.id, key);
            if (vec_set::contains(&set.addresses, &addr)) return;
            vec_set::insert(&mut set.addresses, addr);
        } else {
            let set = TypeDenySet {
                id: object::new(ctx),
                addresses: vec_set::singleton(addr),
                paused: false,
            };
            ofield::add(&mut deny_list.id, key, set);
        };
        let list: &mut PerTypeList = bag::borrow_mut(&mut deny_list.lists, per_type_index);
        if (!table::contains(&list.denied_count, addr)) {
            table::add(&mut list.denied_count, addr, 0);
        };
        let count = table::borrow_mut(&mut list.denied_count, addr);
        *count = *count + 1;
    }

    /// Remove a previously denied address. Aborts `ENotDenied` when the
    /// address is not on the list for `type`.
    ///
    /// Friend-only: callers reach this through coin or KRC-721 wrappers.
    public(friend) fun v1_remove(
        deny_list: &mut DenyList,
        per_type_index: u64,
        `type`: vector<u8>,
        addr: address,
    ) {
        assert!(!is_reserved(addr), EInvalidAddress);
        let key = DenyKey { per_type_index, per_type_key: `type` };
        assert!(ofield::exists_(&deny_list.id, key), ENotDenied);
        let set: &mut TypeDenySet = ofield::borrow_mut(&mut deny_list.id, key);
        assert!(vec_set::contains(&set.addresses, &addr), ENotDenied);
        vec_set::remove(&mut set.addresses, &addr);
        let list: &mut PerTypeList = bag::borrow_mut(&mut deny_list.lists, per_type_index);
        let count = table::borrow_mut(&mut list.denied_count, addr);
        *count = *count - 1;
        if (*count == 0) {
            table::remove(&mut list.denied_count, addr);
        };
    }

    /// Open membership read: anyone may check whether `addr` is denied for
    /// `type`. Reserved addresses are never denied.
    public fun v1_contains(
        deny_list: &DenyList,
        per_type_index: u64,
        `type`: vector<u8>,
        addr: address,
    ): bool {
        if (is_reserved(addr)) return false;
        if (!bag::contains(&deny_list.lists, per_type_index)) return false;
        let list: &PerTypeList = bag::borrow(&deny_list.lists, per_type_index);
        if (!table::contains(&list.denied_count, addr)) return false;
        if (*table::borrow(&list.denied_count, addr) == 0) return false;
        let key = DenyKey { per_type_index, per_type_key: `type` };
        if (!ofield::exists_(&deny_list.id, key)) return false;
        let set: &TypeDenySet = ofield::borrow(&deny_list.id, key);
        vec_set::contains(&set.addresses, &addr)
    }

    // === V2: per-type global pause ===
    //
    // A paused type is blocked on every regulated path (fail-closed
    // kill-switch), independent of individual deny entries. The flag lives
    // on the type's own `TypeDenySet` object (no sibling object), because
    // dynamic fields here are keyed by name bytes: a second key type with
    // the same layout would collide with `DenyKey`. Pauses take effect
    // immediately; unlike Sui there is no next-epoch staging here because
    // this runtime applies config without epoch-gated governance.

    /// Pause all regulated flows for `type`. Creates the type's set object
    /// (unpaused-addresses, paused flag) on first use. Only coin and KRC-721
    /// capability-gated wrappers may call this.
    public(friend) fun v2_enable_global_pause(
        deny_list: &mut DenyList,
        per_type_index: u64,
        `type`: vector<u8>,
        ctx: &mut TxContext,
    ) {
        let key = DenyKey { per_type_index, per_type_key: `type` };
        if (ofield::exists_(&deny_list.id, key)) {
            let set: &mut TypeDenySet = ofield::borrow_mut(&mut deny_list.id, key);
            set.paused = true;
        } else {
            let set = TypeDenySet {
                id: object::new(ctx),
                addresses: vec_set::empty(),
                paused: true,
            };
            ofield::add(&mut deny_list.id, key, set);
        };
    }

    /// Lift the pause for `type`. Missing set object means not paused:
    /// a no-op rather than an abort. Friend-only.
    public(friend) fun v2_disable_global_pause(
        deny_list: &mut DenyList,
        per_type_index: u64,
        `type`: vector<u8>,
    ) {
        let key = DenyKey { per_type_index, per_type_key: `type` };
        if (!ofield::exists_(&deny_list.id, key)) return;
        let set: &mut TypeDenySet = ofield::borrow_mut(&mut deny_list.id, key);
        set.paused = false;
    }

    /// Open read: true iff `type` is currently paused.
    public fun v2_is_global_pause_enabled(
        deny_list: &DenyList,
        per_type_index: u64,
        `type`: vector<u8>,
    ): bool {
        let key = DenyKey { per_type_index, per_type_key: `type` };
        if (!ofield::exists_(&deny_list.id, key)) return false;
        let set: &TypeDenySet = ofield::borrow(&deny_list.id, key);
        set.paused
    }
}
