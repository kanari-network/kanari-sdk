// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Kanari's collection-bound unique NFT standard. Every production mint and
/// burn is paired with the matching `Collection` and `NftCap`; metadata and
/// attributes live on the object, while ownership stays in the object runtime.
/// Issuer modules may wrap `transfer` to enforce their own policies.
module kanari_system::krc_721 {
    use std::string::{Self, String};
    use std::vector;
    use std::option::{Self, Option};
    use kanari_system::address;
    use kanari_system::collection::{Self, NftCap};
    use kanari_system::deny_list::{Self, DenyList};
    use kanari_system::event;
    use kanari_system::object::{Self, ID, UID};
    use kanari_system::transfer;
    use kanari_system::transfer_policy::{
        Self,
        TransferPolicy,
        TransferPolicyCap,
        TransferRequest,
    };
    use kanari_system::tx_context::TxContext;
    use kanari_system::tx_context;
    use kanari_system::url::Url;
    use kanari_system::vec_map::{Self, VecMap};

    /// `keys` and `values` were different lengths, so the attributes could not
    /// be paired up.
    const E_ATTRIBUTE_LENGTH_MISMATCH: u64 = 0;

    /// `attributes` holds more entries than `MAX_ATTRIBUTES`.
    const E_TOO_MANY_ATTRIBUTES: u64 = 1;

    /// The NFT does not belong to the supplied collection.
    const E_NFT_COLLECTION_MISMATCH: u64 = 2;

    /// Display seed keys and values have different lengths.
    const E_DISPLAY_FIELD_COUNT_MISMATCH: u64 = 3;

    /// The canonical NFT type field cannot be removed or overwritten.
    const E_PROTECTED_DISPLAY_FIELD: u64 = 4;

    /// The capability belongs to another collection.
    const E_DISPLAY_COLLECTION_MISMATCH: u64 = 5;

    /// The policy capability does not govern the supplied policy.
    const E_POLICY_MISMATCH: u64 = 6;

    /// The NFT is outside the collection bound to a policy scope.
    const E_POLICY_COLLECTION_MISMATCH: u64 = 7;

    /// The sender, recipient, or collection is blocked by its deny policy.
    const E_DENIED_OR_PAUSED: u64 = 8;

    /// Upper bound on attribute count per token. Not a storage limit — it is a
    /// bound on how much arbitrary text one token can carry, which matters
    /// because anyone holding the object can rewrite its attributes and every
    /// rewrite costs the chain.
    const MAX_ATTRIBUTES: u64 = 64;

    /// Separate deny-list namespace from regulated coins.
    const KRC_721_DENY_LIST_INDEX: u64 = 1;

    #[test_only]
    use std::option as option_alias;

    #[test_only]
    use std::string as string_alias;

    /// A single NFT. The UID is its identity; `number` is its mint ordinal.
    struct Nft has key, store {
        id: UID,
        collection: address,
        number: u64,
        name: String,
        description: Option<String>,
        image_url: Option<Url>,
        /// Per-token traits, keyed by name. Insertion order is preserved so a
        /// renderer shows them in the order the issuer chose.
        attributes: VecMap<String, String>,
    }

    /// Mint index event: object id, original creator, then collection id.
    struct MintLog has copy, drop {
        object_id: address,
        creator: address,
        collection_id: address,
    }

    /// Collection-specific display fields for KRC-721 NFTs. The collection's
    /// mint capability controls mutations; this display is independent of
    /// generic type displays and is discoverable by its collection id.
    struct CollectionDisplay has key, store {
        id: UID,
        collection_id: address,
        fields: VecMap<String, String>,
    }

    /// Shared binding between one KRC-721 collection and its transfer policy.
    /// Trade entry points must receive this object, so an NFT cannot be traded
    /// through a policy belonging to a different collection.
    struct CollectionPolicyScope has key, store {
        id: UID,
        policy_id: ID,
        collection_id: address,
    }

    /// Collection-owner authority for one KRC-721 transfer policy.
    struct CollectionPolicyCap has key, store {
        id: UID,
        policy_id: ID,
        collection_id: address,
        cap: TransferPolicyCap<Nft>,
    }

    /// Create metadata for a collection. The canonical KRC-721 type name is
    /// inserted automatically as the first field.
    public fun create_collection_display(
        cap: &NftCap,
        keys: vector<vector<u8>>,
        values: vector<vector<u8>>,
        ctx: &mut TxContext,
    ): CollectionDisplay {
        assert!(
            vector::length(&keys) == vector::length(&values),
            E_DISPLAY_FIELD_COUNT_MISMATCH
        );
        let type_name = string::utf8(nft_type_name_bytes());
        let fields = vec_map::empty<String, String>();
        vec_map::insert(&mut fields, type_name, type_name);
        let n = vector::length(&keys);
        let i = 0;
        while (i < n) {
            vec_map::insert(
                &mut fields,
                string::utf8(*vector::borrow(&keys, i)),
                string::utf8(*vector::borrow(&values, i)),
            );
            i = i + 1;
        };
        CollectionDisplay {
            id: object::new(ctx),
            collection_id: collection::cap_collection_id(cap),
            fields,
        }
    }

    /// Share collection metadata for public discovery by wallets and indexers.
    public fun share_collection_display(display: CollectionDisplay) {
        transfer::share_object(display);
    }

    /// Read a collection display field, or `none` if it is absent.
    public fun collection_display_field(
        display: &CollectionDisplay,
        name: vector<u8>,
    ): Option<String> {
        let key = string::utf8(name);
        if (vec_map::contains(&display.fields, &key)) {
            option::some(*vec_map::get(&display.fields, &key))
        } else {
            option::none<String>()
        }
    }

    /// Number of fields in a collection display.
    public fun collection_display_field_count(display: &CollectionDisplay): u64 {
        vec_map::size(&display.fields)
    }

    /// Add or replace a field, authorized by the matching collection cap.
    public fun add_collection_display_field(
        display: &mut CollectionDisplay,
        cap: &NftCap,
        name: vector<u8>,
        value: vector<u8>,
    ) {
        assert!(
            display.collection_id == collection::cap_collection_id(cap),
            E_DISPLAY_COLLECTION_MISMATCH
        );
        let key = string::utf8(name);
        assert!(key != string::utf8(nft_type_name_bytes()), E_PROTECTED_DISPLAY_FIELD);
        if (vec_map::contains(&display.fields, &key)) {
            let (_old_key, _old_value) = vec_map::remove(&mut display.fields, &key);
        };
        vec_map::insert(&mut display.fields, key, string::utf8(value));
    }

    /// Remove a field, while preserving the canonical NFT type field.
    public fun remove_collection_display_field(
        display: &mut CollectionDisplay,
        cap: &NftCap,
        name: vector<u8>,
    ) {
        assert!(
            display.collection_id == collection::cap_collection_id(cap),
            E_DISPLAY_COLLECTION_MISMATCH
        );
        let key = string::utf8(name);
        assert!(key != string::utf8(nft_type_name_bytes()), E_PROTECTED_DISPLAY_FIELD);
        vec_map::remove(&mut display.fields, &key);
    }

    /// Address of this collection display object.
    public fun collection_display_address(display: &CollectionDisplay): address {
        object::uid_address(&display.id)
    }

    /// Collection id governed by the display.
    public fun collection_display_collection_id(display: &CollectionDisplay): address {
        display.collection_id
    }

    // ==================== Collection transfer policy ====================

    /// Create a KRC-721 policy bound to the collection authorized by `cap`.
    public fun new_collection_policy(
        cap: &NftCap,
        collection_ref: &collection::Collection,
        ctx: &mut TxContext,
    ): (TransferPolicy<Nft>, CollectionPolicyScope, CollectionPolicyCap) {
        let collection_id = collection::collection_id(collection_ref);
        assert!(collection::cap_collection_id(cap) == collection_id, E_POLICY_COLLECTION_MISMATCH);
        let (policy, inner_cap) = transfer_policy::new_for_collection<Nft>(
            cap, collection_ref, ctx
        );
        let policy_id = transfer_policy::id(&policy);
        let scope = CollectionPolicyScope {
            id: object::new(ctx),
            policy_id,
            collection_id,
        };
        let policy_cap = CollectionPolicyCap {
            id: object::new(ctx),
            policy_id,
            collection_id,
            cap: inner_cap,
        };
        (policy, scope, policy_cap)
    }

    /// Share a KRC-721 policy so it can be used by trade contracts.
    public fun share_collection_policy(policy: TransferPolicy<Nft>) {
        transfer_policy::share<Nft>(policy);
    }

    /// Share the public scope used to check collection ownership during trades.
    public fun share_collection_policy_scope(scope: CollectionPolicyScope) {
        transfer::share_object(scope);
    }

    /// Transfer collection policy administration to `recipient`.
    public fun transfer_collection_policy_cap(cap: CollectionPolicyCap, recipient: address) {
        transfer::public_transfer(cap, recipient);
    }

    /// Collection id governed by this policy scope.
    public fun collection_policy_scope_collection_id(scope: &CollectionPolicyScope): address {
        scope.collection_id
    }

    /// Register a rule on a collection-bound KRC-721 policy.
    public fun add_collection_policy_rule<Rule: drop>(
        rule: Rule,
        policy: &mut TransferPolicy<Nft>,
        cap: &CollectionPolicyCap,
        config: vector<u8>,
    ) {
        assert_collection_policy_cap(policy, cap);
        transfer_policy::add_rule<Rule, Nft>(rule, policy, &cap.cap, config);
    }

    /// Replace a rule's configuration on a collection-bound KRC-721 policy.
    public fun replace_collection_policy_rule<Rule: drop>(
        rule: Rule,
        policy: &mut TransferPolicy<Nft>,
        cap: &CollectionPolicyCap,
        config: vector<u8>,
    ) {
        assert_collection_policy_cap(policy, cap);
        transfer_policy::replace_rule<Rule, Nft>(rule, policy, &cap.cap, config);
    }

    /// Remove a rule from a collection-bound KRC-721 policy.
    public fun remove_collection_policy_rule<Rule: drop>(
        rule: Rule,
        policy: &mut TransferPolicy<Nft>,
        cap: &CollectionPolicyCap,
    ) {
        assert_collection_policy_cap(policy, cap);
        transfer_policy::remove_rule<Rule, Nft>(rule, policy, &cap.cap);
    }

    /// Open a transfer request only when `nft` belongs to the scoped collection.
    public fun new_collection_request(
        policy: &TransferPolicy<Nft>,
        scope: &CollectionPolicyScope,
        nft: &Nft,
        paid: u64,
        from: ID,
    ): TransferRequest<Nft> {
        assert!(scope.policy_id == transfer_policy::id(policy), E_POLICY_MISMATCH);
        transfer_policy::assert_collection_binding(policy, scope.collection_id);
        assert!(scope.collection_id == collection_of(nft), E_POLICY_COLLECTION_MISMATCH);
        transfer_policy::new_scoped_request<Nft>(policy, id(nft), paid, from)
    }

    fun assert_collection_policy_cap(policy: &TransferPolicy<Nft>, cap: &CollectionPolicyCap) {
        assert!(cap.policy_id == transfer_policy::id(policy), E_POLICY_MISMATCH);
        transfer_policy::assert_collection_binding(policy, cap.collection_id);
    }

    /// Canonical type name used by display indexers.
    public fun nft_type_name(): vector<u8> {
        nft_type_name_bytes()
    }

    fun nft_type_name_bytes(): vector<u8> {
        let name = std::type_name::get<Nft>();
        *std::ascii::as_bytes(std::type_name::borrow_string(&name))
    }

    /// Add an address to the deny list for the collection governed by `cap`.
    public fun deny_add(
        deny_list: &mut DenyList,
        cap: &NftCap,
        addr: address,
        ctx: &mut TxContext,
    ) {
        deny_list::v1_add(
            deny_list,
            KRC_721_DENY_LIST_INDEX,
            address::to_bytes(collection::cap_collection_id(cap)),
            addr,
            ctx
        );
    }

    /// Remove an address from the deny list for the collection governed by `cap`.
    public fun deny_remove(
        deny_list: &mut DenyList,
        cap: &NftCap,
        addr: address,
    ) {
        deny_list::v1_remove(
            deny_list,
            KRC_721_DENY_LIST_INDEX,
            address::to_bytes(collection::cap_collection_id(cap)),
            addr
        );
    }

    /// Pause all KRC-721 transfers for the collection governed by `cap`.
    public fun deny_pause(
        deny_list: &mut DenyList,
        cap: &NftCap,
        ctx: &mut TxContext,
    ) {
        deny_list::v2_enable_global_pause(
            deny_list,
            KRC_721_DENY_LIST_INDEX,
            address::to_bytes(collection::cap_collection_id(cap)),
            ctx
        );
    }

    /// Lift the KRC-721 transfer pause for the collection governed by `cap`.
    public fun deny_unpause(deny_list: &mut DenyList, cap: &NftCap) {
        deny_list::v2_disable_global_pause(
            deny_list,
            KRC_721_DENY_LIST_INDEX,
            address::to_bytes(collection::cap_collection_id(cap))
        );
    }

    /// Return whether `addr` is denied for `collection_id`.
    public fun is_denied(deny_list: &DenyList, collection_id: address, addr: address): bool {
        deny_list::v1_contains(
            deny_list,
            KRC_721_DENY_LIST_INDEX,
            address::to_bytes(collection_id),
            addr
        )
    }

    /// Return whether transfers are paused for `collection_id`.
    public fun is_paused(deny_list: &DenyList, collection_id: address): bool {
        deny_list::v2_is_global_pause_enabled(
            deny_list,
            KRC_721_DENY_LIST_INDEX,
            address::to_bytes(collection_id)
        )
    }

    // Construct an unindexed NFT fixture for unit tests only.
    #[test_only]
    public fun create(
        name: String,
        description: Option<String>,
        image_url: Option<Url>,
        ctx: &mut TxContext,
    ): Nft {
        create_with_attributes(
            name,
            description,
            image_url,
            vector::empty(),
            vector::empty(),
            ctx
        )
    }

    // Construct a test fixture from parallel key and value vectors.
    #[test_only]
    public fun create_with_attributes(
        name: String,
        description: Option<String>,
        image_url: Option<Url>,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
        ctx: &mut TxContext,
    ): Nft {
        let attribute_keys = attribute_keys;
        let attribute_values = attribute_values;
        let attributes = build_attributes(attribute_keys, attribute_values);
        Nft {
            id: object::new(ctx),
            collection: @0x0,
            number: 0,
            name,
            description,
            image_url,
            attributes
        }
    }

    // Construct a test fixture whose attributes are already a `VecMap`.
    #[test_only]
    public fun from_vec_map(
        name: String,
        description: Option<String>,
        image_url: Option<Url>,
        attributes: VecMap<String, String>,
        ctx: &mut TxContext,
    ): Nft {
        assert!(vec_map::size(&attributes) <= MAX_ATTRIBUTES, E_TOO_MANY_ATTRIBUTES);
        Nft {
            id: object::new(ctx),
            collection: @0x0,
            number: 0,
            name,
            description,
            image_url,
            attributes
        }
    }

    /// Transfer ownership. This is the whole ERC-721 `transferFrom` analogue:
    /// moving the object *is* the transfer, so there is no separate approval
    /// surface to get wrong.
    public fun transfer(nft: Nft, recipient: address) {
        transfer::public_transfer(nft, recipient);
    }

    /// KRC-721 transfer with explicit deny-list enforcement. This is useful to
    /// issuers and tests that already pass the shared deny-list object.
    public fun transfer_checked(
        nft: Nft,
        recipient: address,
        deny: &DenyList,
        ctx: &TxContext,
    ) {
        let collection_id = nft.collection;
        let sender = tx_context::sender(ctx);
        assert!(!is_denied(deny, collection_id, sender), E_DENIED_OR_PAUSED);
        assert!(!is_denied(deny, collection_id, recipient), E_DENIED_OR_PAUSED);
        assert!(!is_paused(deny, collection_id), E_DENIED_OR_PAUSED);
        transfer::public_transfer(nft, recipient);
    }

    /// Canonical production transfer. The runtime-created deny-list singleton
    /// is read automatically, so callers do not need to supply it as input.
    public fun transfer_with_global_deny(
        nft: Nft,
        recipient: address,
        ctx: &TxContext,
    ) {
        transfer_checked(nft, recipient, deny_list::borrow_singleton(), ctx)
    }

    /// Update the mutable descriptive fields.
    ///
    /// Deliberately does **not** touch `attributes`. They are a separate
    /// operation with separate entry points (`set_attribute` and friends), so a
    /// caller renaming a token cannot silently drop its traits, and a caller
    /// editing traits does not have to restate the name.
    public fun update(
        nft: &mut Nft,
        name: String,
        description: Option<String>,
        image_url: Option<Url>,
    ) {
        nft.name = name;
        nft.description = description;
        nft.image_url = image_url;
    }

    // Test-only destruction for metadata and attribute unit tests.
    #[test_only]
    public fun destroy_for_test(nft: Nft) {
        let Nft {
            id,
            collection: _,
            number: _,
            name: _,
            description: _,
            image_url: _,
            attributes: _
        } = nft;
        object::delete(id);
    }

    /// Mint one token against the supplied collection's capped supply.
    /// Both objects are required so the collection module verifies that the
    /// cap belongs to this collection. The caller may then transfer the result.
    public fun mint(
        cap: &mut NftCap,
        collection_ref: &collection::Collection,
        name: String,
        description: Option<String>,
        image_url: Option<Url>,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
        ctx: &mut TxContext,
    ): Nft {
        let attributes = build_attributes(attribute_keys, attribute_values);
        collection::consume_for_mint(cap, collection_ref);
        let number = collection::issued(cap);
        let nft = Nft {
            id: object::new(ctx),
            collection: collection::collection_id(collection_ref),
            number,
            name,
            description,
            image_url,
            attributes
        };
        event::emit(MintLog {
            object_id: object::uid_address(&nft.id),
            creator: tx_context::sender(ctx),
            collection_id: nft.collection
        });
        nft
    }

    /// Mint with an attribute map already assembled by the issuer module.
    public fun mint_with_attributes(
        cap: &mut NftCap,
        collection_ref: &collection::Collection,
        name: String,
        description: Option<String>,
        image_url: Option<Url>,
        attributes: VecMap<String, String>,
        ctx: &mut TxContext,
    ): Nft {
        assert!(vec_map::size(&attributes) <= MAX_ATTRIBUTES, E_TOO_MANY_ATTRIBUTES);
        collection::consume_for_mint(cap, collection_ref);
        let number = collection::issued(cap);
        let nft = Nft {
            id: object::new(ctx),
            collection: collection::collection_id(collection_ref),
            number,
            name,
            description,
            image_url,
            attributes
        };
        event::emit(MintLog {
            object_id: object::uid_address(&nft.id),
            creator: tx_context::sender(ctx),
            collection_id: nft.collection
        });
        nft
    }

    /// Burn a token from its original collection and release one remaining
    /// supply unit. The historical issued ordinal is never reused.
    public fun burn(
        cap: &mut NftCap,
        collection_ref: &collection::Collection,
        nft: Nft,
    ) {
        assert!(
            nft.collection == collection::collection_id(collection_ref),
            E_NFT_COLLECTION_MISMATCH
        );
        collection::return_from_burn(cap, collection_ref);
        let Nft {
            id,
            collection: _,
            number: _,
            name: _,
            description: _,
            image_url: _,
            attributes: _
        } = nft;
        object::delete(id);
    }

    /// The token id.
    public fun id(nft: &Nft): ID {
        object::uid_to_inner(&nft.id)
    }

    /// The NFT's object address.
    public fun nft_address(nft: &Nft): address {
        object::uid_address(&nft.id)
    }

    public fun collection_of(nft: &Nft): address {
        nft.collection
    }

    /// 1-based historical mint ordinal within the collection.
    public fun number(nft: &Nft): u64 {
        nft.number
    }

    /// The name field.
    public fun name(nft: &Nft): String {
        nft.name
    }

    /// The description field.
    public fun description(nft: &Nft): Option<String> {
        nft.description
    }

    /// The image URL field.
    public fun image_url(nft: &Nft): Option<Url> {
        nft.image_url
    }

    // ==================== Attributes ====================

    /// The value stored under `key`, or `none`.
    ///
    /// This is the ERC-721 `tokenURI`-style attribute read the design doc
    /// specifies: a keyed lookup rather than positional indexing, so a missing
    /// trait is absent rather than shifted onto the wrong one.
    public fun attribute(nft: &Nft, key: &String): Option<String> {
        attribute_of(&nft.attributes, key)
    }

    /// Insert or overwrite `key`. Upsert rather than insert-only, because the
    /// common case is "this trait's value changed", and forcing callers to
    /// remove-then-insert would let a read-modify-write race leave the map
    /// empty in between.
    ///
    /// Overwriting an existing key does not consume another slot, so the count
    /// check is skipped on that path — otherwise a legitimate update at the
    /// limit would be refused.
    public fun set_attribute(nft: &mut Nft, key: String, value: String) {
        set_attribute_on(&mut nft.attributes, key, value);
    }

    /// Remove `key` and return its value. Aborts if absent, so a caller cannot
    /// believe it cleared a trait that was never there.
    public fun remove_attribute(nft: &mut Nft, key: &String): String {
        remove_attribute_from(&mut nft.attributes, key)
    }

    /// Replace every attribute at once, from parallel vectors.
    ///
    /// The wholesale form, matching how a legacy caller that stored
    /// `attribute_keys` / `attribute_values` updates them. Length and count are
    /// checked **before** anything is mutated, so a bad call leaves the token's
    /// existing attributes intact rather than half-applied — the replacement is
    /// built first and only then swapped in.
    public fun set_attributes(
        nft: &mut Nft,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
    ) {
        let attribute_keys = attribute_keys;
        let attribute_values = attribute_values;
        // Built and validated before the swap, so a bad call leaves the token's
        // existing attributes intact rather than half-applied.
        let replacement = build_attributes(attribute_keys, attribute_values);
        // `String` has `drop`, so `VecMap<String, String>` does too and the
        // superseded map is released here rather than needing a drain loop.
        nft.attributes = replacement;
    }

    /// True iff `key` is present.
    public fun has_attribute(nft: &Nft, key: &String): bool {
        has_attribute_in(&nft.attributes, key)
    }

    /// The per-token attribute bound this module enforces.
    ///
    /// Exposed so an issuer holding its own attribute map — rather than an
    /// `Nft` — can clamp to the same limit instead of inventing a second,
    /// possibly looser, one.
    public fun max_attributes(): u64 {
        MAX_ATTRIBUTES
    }

    /// Build a validated `VecMap` from parallel key and value vectors.
    ///
    /// Shared by `create_with_attributes` and exposed for the same reason
    /// `max_attributes` is: the length check is the whole point of storing
    /// attributes as a map, and every issuer should get it rather than
    /// reimplementing (or forgetting) the guard.
    public fun build_attributes(
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
    ): VecMap<String, String> {
        assert!(
            vector::length(&attribute_keys) == vector::length(&attribute_values),
            E_ATTRIBUTE_LENGTH_MISMATCH
        );
        assert!(
            vector::length(&attribute_keys) <= MAX_ATTRIBUTES,
            E_TOO_MANY_ATTRIBUTES
        );
        vec_map::from_keys_values(attribute_keys, attribute_values)
    }

    // ==================== Attribute map primitives ====================
    //
    // These operate on a bare `VecMap<String, String>` rather than an `Nft`, so
    // an issuer that keeps attributes on its own token type gets the identical
    // bounds and upsert semantics instead of reimplementing them — and getting
    // the limit subtly wrong. `Nft`'s own methods below are thin wrappers over
    // these, so there is exactly one implementation of each rule.

    /// The value under `key` in `attributes`, or `none`.
    public fun attribute_of(
        attributes: &VecMap<String, String>,
        key: &String,
    ): Option<String> {
        vec_map::try_get(attributes, key)
    }

    /// True iff `key` is present in `attributes`.
    public fun has_attribute_in(
        attributes: &VecMap<String, String>,
        key: &String,
    ): bool {
        vec_map::contains(attributes, key)
    }

    /// How many entries `attributes` holds.
    public fun attribute_count_of(attributes: &VecMap<String, String>): u64 {
        vec_map::size(attributes)
    }

    /// Keys of `attributes` in insertion order.
    public fun attribute_keys_of(attributes: &VecMap<String, String>): vector<String> {
        vec_map::keys(attributes)
    }

    /// Every `(key, value)` pair in insertion order, index-aligned.
    public fun all_attributes_of(
        attributes: &VecMap<String, String>,
    ): (vector<String>, vector<String>) {
        let keys = vec_map::keys(attributes);
        let values = vector::empty<String>();
        let i = 0;
        let n = vector::length(&keys);
        while (i < n) {
            let key = *vector::borrow(&keys, i);
            vector::push_back(&mut values, *vec_map::get(attributes, &key));
            i = i + 1;
        };
        (keys, values)
    }

    /// Upsert `key` in `attributes`, enforcing `MAX_ATTRIBUTES` for new keys.
    public fun set_attribute_on(
        attributes: &mut VecMap<String, String>,
        key: String,
        value: String,
    ) {
        if (vec_map::contains(attributes, &key)) {
            let (_removed_key, _removed_value) = vec_map::remove(attributes, &key);
        } else {
            assert!(
                vec_map::size(attributes) < MAX_ATTRIBUTES,
                E_TOO_MANY_ATTRIBUTES
            );
        };
        vec_map::insert(attributes, key, value);
    }

    /// Remove `key` from `attributes`, returning its value. Aborts if absent.
    public fun remove_attribute_from(
        attributes: &mut VecMap<String, String>,
        key: &String,
    ): String {
        let (_removed_key, value) = vec_map::remove(attributes, key);
        value
    }

    /// How many attributes the token carries.
    public fun attribute_count(nft: &Nft): u64 {
        attribute_count_of(&nft.attributes)
    }

    /// Attribute keys in insertion order. Not sorted — do not rely on ordering.
    public fun attribute_keys(nft: &Nft): vector<String> {
        attribute_keys_of(&nft.attributes)
    }

    /// Every `(key, value)` pair in insertion order, for a renderer that wants
    /// the whole set in one call. Keys and values are index-aligned, so element
    /// `i` of each is one attribute.
    public fun all_attributes(nft: &Nft): (vector<String>, vector<String>) {
        all_attributes_of(&nft.attributes)
    }

    #[test]
    fun test_nft_lifecycle() {
        let ctx = tx_context::dummy();
        let nft =
            create(
                string_alias::utf8(b"One"),
                option_alias::some(string_alias::utf8(b"First")),
                option_alias::none<Url>(),
                &mut ctx
            );

        // The object id is the token id; there is no separate counter.
        assert!(id(&nft) != id_from(@0x9), 0);
        assert!(name(&nft) == string_alias::utf8(b"One"), 1);
        assert!(option_alias::is_some(&description(&nft)), 2);
        assert!(option_alias::is_none(&image_url(&nft)), 3);

        update(
            &mut nft,
            string_alias::utf8(b"Two"),
            option_alias::none<String>(),
            option_alias::none<Url>()
        );
        assert!(name(&nft) == string_alias::utf8(b"Two"), 4);
        assert!(option_alias::is_none(&description(&nft)), 5);

        destroy_for_test(nft);
    }

    #[test]
    /// The whole point of storing attributes as a map: a key always means the
    /// same value, no matter how many traits there are.
    fun test_attributes_are_keyed_not_positional() {
        let ctx = tx_context::dummy();
        let keys = vector::empty<String>();
        let values = vector::empty<String>();
        vector::push_back(&mut keys, string_alias::utf8(b"Level"));
        vector::push_back(&mut values, string_alias::utf8(b"10"));
        vector::push_back(&mut keys, string_alias::utf8(b"Attack"));
        vector::push_back(&mut values, string_alias::utf8(b"150"));
        vector::push_back(&mut keys, string_alias::utf8(b"Rarity"));
        vector::push_back(&mut values, string_alias::utf8(b"Rare"));

        let nft =
            create_with_attributes(
                string_alias::utf8(b"Kid"),
                option_alias::none<String>(),
                option_alias::none<Url>(),
                keys,
                values,
                &mut ctx
            );

        assert!(attribute_count(&nft) == 3, 0);
        assert!(has_attribute(&nft, &string_alias::utf8(b"Attack")), 1);
        // Each key resolves independently — the property a parallel vector pair
        // cannot offer.
        assert!(
            option_alias::is_some(&attribute(&nft, &string_alias::utf8(b"Level"))),
            2
        );
        assert!(
            option_alias::is_some(&attribute(&nft, &string_alias::utf8(b"Rarity"))),
            3
        );
        // An absent key is `none`, not a shifted neighbour.
        assert!(
            option_alias::is_none(&attribute(&nft, &string_alias::utf8(b"Speed"))),
            4
        );

        let (all_keys, all_values) = all_attributes(&nft);
        assert!(vector::length(&all_keys) == 3, 5);
        assert!(vector::length(&all_values) == 3, 6);
        // Insertion order is preserved, and the two vectors stay aligned.
        assert!(*vector::borrow(&all_keys, 0) == string_alias::utf8(b"Level"), 7);
        assert!(*vector::borrow(&all_values, 0) == string_alias::utf8(b"10"), 8);
        assert!(*vector::borrow(&all_keys, 2) == string_alias::utf8(b"Rarity"), 9);

        destroy_for_test(nft);
    }

    #[test]
    /// `set_attribute` overwrites rather than aborting, so a trait whose value
    /// changed does not need a remove-then-insert dance.
    fun test_set_attribute_upserts() {
        let ctx = tx_context::dummy();
        let nft =
            create(
                string_alias::utf8(b"Kid"),
                option_alias::none<String>(),
                option_alias::none<Url>(),
                &mut ctx
            );
        assert!(attribute_count(&nft) == 0, 0);

        set_attribute(&mut nft, string_alias::utf8(b"Level"), string_alias::utf8(b"1"));
        set_attribute(&mut nft, string_alias::utf8(b"Level"), string_alias::utf8(b"2"));
        // Same key twice: still one attribute, now holding the newer value.
        assert!(attribute_count(&nft) == 1, 1);
        assert!(
            option_alias::is_some(&attribute(&nft, &string_alias::utf8(b"Level"))),
            2
        );

        let removed = remove_attribute(&mut nft, &string_alias::utf8(b"Level"));
        assert!(removed == string_alias::utf8(b"2"), 3);
        assert!(attribute_count(&nft) == 0, 4);

        destroy_for_test(nft);
    }

    #[test]
    /// Renaming a token must not silently drop its traits, which is why
    /// `update` and `set_attributes` are separate operations.
    fun test_update_does_not_touch_attributes() {
        let ctx = tx_context::dummy();
        let keys = vector::empty<String>();
        let values = vector::empty<String>();
        vector::push_back(&mut keys, string_alias::utf8(b"Level"));
        vector::push_back(&mut values, string_alias::utf8(b"7"));
        let nft =
            create_with_attributes(
                string_alias::utf8(b"Before"),
                option_alias::none<String>(),
                option_alias::none<Url>(),
                keys,
                values,
                &mut ctx
            );

        update(
            &mut nft,
            string_alias::utf8(b"After"),
            option_alias::none<String>(),
            option_alias::none<Url>()
        );

        assert!(name(&nft) == string_alias::utf8(b"After"), 0);
        // The trait survived the rename.
        assert!(attribute_count(&nft) == 1, 1);
        assert!(
            option_alias::is_some(&attribute(&nft, &string_alias::utf8(b"Level"))),
            2
        );

        destroy_for_test(nft);
    }

    #[test]
    /// Wholesale replacement is the legacy `attribute_keys` / `attribute_values`
    /// update path, and it swaps the whole set rather than merging.
    fun test_set_attributes_replaces_the_whole_set() {
        let ctx = tx_context::dummy();
        let nft =
            create(
                string_alias::utf8(b"Kid"),
                option_alias::none<String>(),
                option_alias::none<Url>(),
                &mut ctx
            );
        set_attribute(&mut nft, string_alias::utf8(b"Old"), string_alias::utf8(b"gone"));

        let keys = vector::empty<String>();
        let values = vector::empty<String>();
        vector::push_back(&mut keys, string_alias::utf8(b"New"));
        vector::push_back(&mut values, string_alias::utf8(b"here"));
        set_attributes(&mut nft, keys, values);

        assert!(attribute_count(&nft) == 1, 0);
        assert!(!has_attribute(&nft, &string_alias::utf8(b"Old")), 1);
        assert!(has_attribute(&nft, &string_alias::utf8(b"New")), 2);

        destroy_for_test(nft);
    }

    #[test]
    #[expected_failure(abort_code = E_ATTRIBUTE_LENGTH_MISMATCH)]
    /// The negative case the map storage form exists to prevent. A parallel
    /// vector pair accepts this happily and leaves every later read guessing.
    fun test_mismatched_attribute_lengths_abort() {
        let ctx = tx_context::dummy();
        let keys = vector::empty<String>();
        let values = vector::empty<String>();
        vector::push_back(&mut keys, string_alias::utf8(b"Level"));
        vector::push_back(&mut keys, string_alias::utf8(b"Attack"));
        // One key, no values.
        vector::push_back(&mut values, string_alias::utf8(b"10"));
        let nft =
            create_with_attributes(
                string_alias::utf8(b"Kid"),
                option_alias::none<String>(),
                option_alias::none<Url>(),
                keys,
                values,
                &mut ctx
            );
        destroy_for_test(nft);
    }

    #[test]
    #[expected_failure(abort_code = E_TOO_MANY_ATTRIBUTES)]
    fun test_attribute_count_is_bounded() {
        let ctx = tx_context::dummy();
        let nft =
            create(
                string_alias::utf8(b"Kid"),
                option_alias::none<String>(),
                option_alias::none<Url>(),
                &mut ctx
            );
        let i = 0;
        while (i <= MAX_ATTRIBUTES) {
            // `MAX_ATTRIBUTES + 1` distinct keys trips the bound.
            set_attribute(&mut nft, key_n(i), string_alias::utf8(b"v"));
            i = i + 1;
        };
        destroy_for_test(nft);
    }

    #[test]
    /// Filling to exactly the limit must work, and then updating an existing
    /// key at that limit must still work. Both would fail if the bound were
    /// checked before knowing whether the key already existed.
    fun test_updating_at_the_limit_is_allowed() {
        let ctx = tx_context::dummy();
        let nft =
            create(
                string_alias::utf8(b"Kid"),
                option_alias::none<String>(),
                option_alias::none<Url>(),
                &mut ctx
            );
        let i = 0;
        while (i < MAX_ATTRIBUTES) {
            set_attribute(&mut nft, key_n(i), string_alias::utf8(b"v"));
            i = i + 1;
        };
        // Exactly at the bound, not over it.
        assert!(attribute_count(&nft) == MAX_ATTRIBUTES, 0);

        // Overwriting a key that already exists consumes no new slot, so this
        // must not trip the limit.
        set_attribute(&mut nft, key_n(0), string_alias::utf8(b"updated"));
        assert!(attribute_count(&nft) == MAX_ATTRIBUTES, 1);

        // Freeing a slot then adding a genuinely new key works.
        remove_attribute(&mut nft, &key_n(1));
        set_attribute(&mut nft, key_n(999), string_alias::utf8(b"new"));
        assert!(attribute_count(&nft) == MAX_ATTRIBUTES, 2);

        destroy_for_test(nft);
    }

    #[test_only]
    /// A distinct attribute key per index, so a loop cannot collapse into one
    /// upsert and slip past the count check. Two hex nibbles cover 0..255, which
    /// is well past `MAX_ATTRIBUTES`.
    fun key_n(i: u64): String {
        let bytes = vector::empty<u8>();
        // The whole cast is parenthesized: this edition rejects a bare
        // `expr as u8` and only accepts the wrapped form, the same shape
        // `royalty_rule::encode_config` uses.
        let hi = (((i >> 4) & 0xF) as u8);
        let lo = ((i & 0xF) as u8);
        vector::push_back(&mut bytes, nibble_ascii(hi));
        vector::push_back(&mut bytes, nibble_ascii(lo));
        string_alias::utf8(bytes)
    }

    #[test_only]
    /// ASCII for one hex nibble, matching `address.move`'s own encoding.
    fun nibble_ascii(v: u8): u8 {
        if (v < 10) 48 + v else 87 + (v - 10)
    }

    #[test_only]
    fun id_from(addr: address): ID {
        object::id_from_address(addr)
    }
}
