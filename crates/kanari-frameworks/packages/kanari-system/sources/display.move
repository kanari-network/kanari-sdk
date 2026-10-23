// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Type-keyed metadata, mirroring `sui::display`.
///
/// Sui does not put name/image on each token object. One shared `Display<T>`
/// holds a `VecMap<String, String>` keyed by the type's name, so a wallet can
/// render *any* object type without per-object metadata and without the
/// creator writing a custom viewer. Per-token uniqueness comes from the object
/// id, not from the metadata.
///
/// Every mutation takes a `&Publisher<W>` and requires it, through
/// `package::assert_governs_type`, to have been claimed in the same package
/// that defines `T`. Sui types the publisher to the token itself; here the
/// witness carries `drop` while the token must not (see `kanari_system::package`),
/// so type-level binding is unavailable and package-level binding is what the
/// runtime enforces. A foreign package holding its own publisher cannot mint
/// a Display for your type. What remains unenforced is the Sui host-side
/// one-Display-per-type registry: creating two Displays for one type is still
/// possible inside T's own package, and clients must resolve by type name and
/// prefer the most recent. Packages that care should create exactly one, in
/// `init`.
module kanari_system::display {
    use std::vector;
    use std::option::{Self, Option};
    use std::string::{String, utf8};
    use kanari_system::collection::{Self, NftCap};
    use kanari_system::krc_721::Nft;
    use kanari_system::object::{Self, ID, UID};
    use kanari_system::package::{Self, Publisher};
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    use kanari_system::vec_map::{Self, VecMap};

    #[test_only]
    use kanari_system::tx_context;

    /// The key/value seed vectors had different lengths.
    const E_FIELD_COUNT_MISMATCH: u64 = 0;

    /// `T`'s own type-name entry cannot be removed.
    const E_PROTECTED_FIELD: u64 = 1;

    /// The cap does not govern the display's collection.
    const E_COLLECTION_MISMATCH: u64 = 2;

    /// Shared metadata for every object of type `T`. `T` is `phantom`: the
    /// value stores no `T` data, and the parameter exists so that one
    /// `Display<T>` cannot be passed where `Display<U>` is expected. That is
    /// what makes the type the key.
    struct Display<phantom T: key> has key, store {
        id: UID,
        /// `T`'s canonical type name is always the first entry, so indexers
        /// can identify the display from its object alone.
        fields: VecMap<String, String>,
    }

    /// Collection-scoped display for the framework's shared KRC-721 type.
    /// The mint cap, rather than a package publisher, controls its metadata.
    struct Krc721CollectionDisplay has key, store {
        id: UID,
        collection_id: address,
        fields: VecMap<String, String>,
    }

    /// Create collection-specific metadata for `krc_721::Nft`.
    public fun create_krc_721_collection_display(
        cap: &NftCap,
        keys: vector<vector<u8>>,
        values: vector<vector<u8>>,
        ctx: &mut TxContext,
    ): Krc721CollectionDisplay {
        assert!(vector::length(&keys) == vector::length(&values), E_FIELD_COUNT_MISMATCH);
        let type_name = utf8(type_name_bytes<Nft>());
        let fields = seed_fields(keys, values, type_name);
        Krc721CollectionDisplay {
            id: object::new(ctx),
            collection_id: collection::cap_collection_id(cap),
            fields
        }
    }

    /// Share a collection display so indexers and wallets can read it.
    public fun share_krc_721_collection_display(
        display: Krc721CollectionDisplay,
    ) {
        transfer::share_object(display);
    }

    /// Read one collection display field, or `none` when absent.
    public fun get_krc_721_collection_field(
        display: &Krc721CollectionDisplay,
        name: vector<u8>,
    ): Option<String> {
        let key = utf8(name);
        if (vec_map::contains(&display.fields, &key)) {
            option::some(*vec_map::get(&display.fields, &key))
        } else {
            option::none<String>()
        }
    }

    /// Count fields in a collection display.
    public fun krc_721_collection_field_count(
        display: &Krc721CollectionDisplay,
    ): u64 {
        vec_map::size(&display.fields)
    }

    /// Add or replace one field, authorized by the matching collection cap.
    public fun add_krc_721_collection_field(
        display: &mut Krc721CollectionDisplay,
        cap: &NftCap,
        name: vector<u8>,
        value: vector<u8>,
    ) {
        assert!(
            display.collection_id == collection::cap_collection_id(cap),
            E_COLLECTION_MISMATCH
        );
        let key = utf8(name);
        assert!(
            key != utf8(type_name_bytes<Nft>()),
            E_PROTECTED_FIELD
        );
        if (vec_map::contains(&display.fields, &key)) {
            let (_old_key, _old_value) = vec_map::remove(&mut display.fields, &key);
        };
        vec_map::insert(&mut display.fields, key, utf8(value));
    }

    /// Remove one field while protecting the canonical KRC-721 type name.
    public fun remove_krc_721_collection_field(
        display: &mut Krc721CollectionDisplay,
        cap: &NftCap,
        name: vector<u8>,
    ) {
        assert!(
            display.collection_id == collection::cap_collection_id(cap),
            E_COLLECTION_MISMATCH
        );
        let key = utf8(name);
        assert!(
            key != utf8(type_name_bytes<Nft>()),
            E_PROTECTED_FIELD
        );
        vec_map::remove(&mut display.fields, &key);
    }

    /// The display's object address.
    public fun krc_721_collection_display_address(
        display: &Krc721CollectionDisplay,
    ): address {
        object::uid_address(&display.id)
    }

    /// The collection governed by this display.
    public fun krc_721_collection_id(display: &Krc721CollectionDisplay): address {
        display.collection_id
    }

    /// Create a display for `T` with a `name` field already set.
    public fun create_with_name<T: key, W: drop>(
        pub: &Publisher<W>,
        name: vector<u8>,
        ctx: &mut TxContext,
    ): Display<T> {
        let keys = vector::empty<vector<u8>>();
        let values = vector::empty<vector<u8>>();
        vector::push_back(&mut keys, b"name");
        vector::push_back(&mut values, name);
        create_with_fields<T, W>(pub, keys, values, ctx)
    }

    /// Create an empty display for `T`.
    public fun create_empty<T: key, W: drop>(
        pub: &Publisher<W>,
        ctx: &mut TxContext,
    ): Display<T> {
        let keys = vector::empty<vector<u8>>();
        let values = vector::empty<vector<u8>>();
        new_display<T, W>(pub, keys, values, ctx)
    }

    /// Create a display seeded with `keys` / `values`. Both vectors must be the
    /// same length; a mismatched pair would leave a half-written field.
    public fun create_with_fields<T: key, W: drop>(
        pub: &Publisher<W>,
        keys: vector<vector<u8>>,
        values: vector<vector<u8>>,
        ctx: &mut TxContext,
    ): Display<T> {
        assert!(vector::length(&keys) == vector::length(&values), E_FIELD_COUNT_MISMATCH);
        new_display<T, W>(pub, keys, values, ctx)
    }

    /// Seed fields into a caller-built `VecMap`. Split out because this legacy
    /// edition has no `let mut` bindings, so the loop body needs a named local.
    /// The package guard lives here because both seed constructors funnel
    /// through it.
    fun new_display<T: key, W: drop>(
        pub: &Publisher<W>,
        keys: vector<vector<u8>>,
        values: vector<vector<u8>>,
        ctx: &mut TxContext,
    ): Display<T> {
        package::assert_governs_type<T, W>(pub);
        let type_name = utf8(type_name_bytes<T>());
        let fields = seed_fields(keys, values, type_name);
        Display<T> { id: object::new(ctx), fields }
    }

    /// Build the initial field map: the type name first, then every seed pair.
    fun seed_fields(
        keys: vector<vector<u8>>,
        values: vector<vector<u8>>,
        type_name: String,
    ): VecMap<String, String> {
        let fields = vec_map::empty<String, String>();
        vec_map::insert(&mut fields, type_name, type_name);
        let n = vector::length(&keys);
        let i = 0;
        while (i < n) {
            let k = *vector::borrow(&keys, i);
            let v = *vector::borrow(&values, i);
            vec_map::insert(&mut fields, utf8(k), utf8(v));
            i = i + 1;
        };
        fields
    }

    /// Add or replace one field.
    public fun add<T: key, W: drop>(
        display: &mut Display<T>,
        pub: &Publisher<W>,
        name: vector<u8>,
        value: vector<u8>,
    ) {
        package::assert_governs_type<T, W>(pub);
        vec_map::insert(&mut display.fields, utf8(name), utf8(value));
    }

    /// Remove one field. `T`'s own type-name entry is protected: indexers key
    /// off it, and losing it would make the display unattributable.
    public fun remove<T: key, W: drop>(
        display: &mut Display<T>,
        pub: &Publisher<W>,
        name: vector<u8>,
    ) {
        package::assert_governs_type<T, W>(pub);
        let key = utf8(name);
        assert!(
            key != utf8(type_name_bytes<T>()),
            E_PROTECTED_FIELD
        );
        vec_map::remove(&mut display.fields, &key);
    }

    /// Read one field, or `none` when absent.
    public fun get<T: key>(display: &Display<T>, name: vector<u8>): Option<String> {
        let key = utf8(name);
        if (vec_map::contains(&display.fields, &key)) {
            option::some(*vec_map::get(&display.fields, &key))
        } else {
            option::none<String>()
        }
    }

    /// Read one field, falling back to `default` when absent. Convenient for
    /// callers that always need a string (wallets rendering a list).
    public fun get_with_default<T: key>(
        display: &Display<T>,
        name: vector<u8>,
        default: vector<u8>,
    ): String {
        let key = utf8(name);
        if (vec_map::contains(&display.fields, &key)) {
            *vec_map::get(&display.fields, &key)
        } else {
            utf8(default)
        }
    }

    /// Number of fields currently set.
    public fun field_count<T: key>(display: &Display<T>): u64 {
        vec_map::size(&display.fields)
    }

    /// The display's object id.
    public fun id<T: key>(display: &Display<T>): ID {
        object::uid_to_inner(&display.id)
    }

    /// The display's object address.
    public fun display_address<T: key>(display: &Display<T>): address {
        object::uid_address(&display.id)
    }

    /// `T`'s canonical type-name bytes.
    public fun type_name_of<T: key>(): vector<u8> {
        type_name_bytes<T>()
    }

    /// Share the display so anyone can read it. There is nothing sensitive in
    /// the fields, and wallets need it shared to discover it.
    public fun share<T: key>(display: Display<T>) {
        transfer::share_object(display);
    }

    /// Transfer the display to a specific address instead of sharing it.
    public fun transfer<T: key>(display: Display<T>, recipient: address) {
        transfer::public_transfer(display, recipient);
    }

    /// Destroy the display. `T`'s metadata becomes unresolvable off-chain; the
    /// type itself and its objects are unaffected.
    ///
    /// `VecMap` carries `drop`, so the field map goes away with the value; only
    /// the `UID` needs explicit deletion.
    public fun destroy<T: key, W: drop>(
        display: Display<T>,
        pub: &Publisher<W>,
    ) {
        package::assert_governs_type<T, W>(pub);
        let Display { id, fields } = display;
        let _ = fields;
        object::delete(id);
    }

    /// Canonical type-name bytes for `T`.
    fun type_name_bytes<T: key>(): vector<u8> {
        let name = std::type_name::get<T>();
        *std::ascii::as_bytes(std::type_name::borrow_string(&name))
    }

    #[test_only]
    /// One-time witness for the `Display<DISPLAY>` test type. Named after this
    /// module in caps so `types::is_one_time_witness` accepts it, which is
    /// what lets `package::new` claim authority in these tests.
    struct DISPLAY has key, drop {}

    #[test]
    fun test_display_roundtrip() {
        let ctx = tx_context::dummy();
        let pub = package::new<DISPLAY>(DISPLAY {}, &mut ctx);
        let d = create_with_name<DISPLAY, DISPLAY>(&pub, b"My Collection", &mut ctx);

        assert!(
            *vec_map::get(&d.fields, &utf8(b"name")) == utf8(b"My Collection"),
            1
        );
        // The type name is always present, and is the first of two entries.
        assert!(field_count<DISPLAY>(&d) == 2, 2);
        assert!(
            *vec_map::get(&d.fields, &utf8(type_name_bytes<DISPLAY>()))
                == utf8(type_name_bytes<DISPLAY>()),
            3
        );

        add<DISPLAY, DISPLAY>(&mut d, &pub, b"description", b"Round trip");
        assert!(field_count<DISPLAY>(&d) == 3, 4);
        assert!(
            *option::borrow(&get<DISPLAY>(&d, b"description")) == utf8(b"Round trip"),
            5
        );
        assert!(
            option::is_none(&get<DISPLAY>(&d, b"nope")),
            6
        );
        assert!(
            get_with_default<DISPLAY>(&d, b"nope", b"fallback") == utf8(b"fallback"),
            7
        );

        remove<DISPLAY, DISPLAY>(&mut d, &pub, b"description");
        assert!(field_count<DISPLAY>(&d) == 2, 8);

        transfer::public_freeze_object(d);
        transfer::public_freeze_object(pub);
    }

    #[test]
    #[expected_failure(abort_code = E_PROTECTED_FIELD)]
    fun test_display_cannot_drop_type_name_entry() {
        let ctx = tx_context::dummy();
        let pub = package::new<DISPLAY>(DISPLAY {}, &mut ctx);
        let d = create_empty<DISPLAY, DISPLAY>(&pub, &mut ctx);
        remove<DISPLAY, DISPLAY>(&mut d, &pub, type_name_bytes<DISPLAY>());
        transfer::public_freeze_object(d);
        transfer::public_freeze_object(pub);
    }

    #[test]
    #[expected_failure(abort_code = E_PROTECTED_FIELD)]
    fun DISPLAY_cannot_drop_type_name_entry() {
        let ctx = tx_context::dummy();
        let pub = package::new<DISPLAY>(DISPLAY {}, &mut ctx);
        let d = create_empty<DISPLAY, DISPLAY>(&pub, &mut ctx);
        remove<DISPLAY, DISPLAY>(&mut d, &pub, type_name_bytes<DISPLAY>());
        transfer::public_freeze_object(d);
        transfer::public_freeze_object(pub);
    }

    #[test]
    #[expected_failure(abort_code = E_FIELD_COUNT_MISMATCH)]
    fun DISPLAY_rejects_mismatched_seed_lengths() {
        let ctx = tx_context::dummy();
        let pub = package::new<DISPLAY>(DISPLAY {}, &mut ctx);
        let keys = vector::empty<vector<u8>>();
        vector::push_back(&mut keys, b"a");
        vector::push_back(&mut keys, b"b");
        let values = vector::empty<vector<u8>>();
        vector::push_back(&mut values, b"only-one");
        let d = create_with_fields<DISPLAY, DISPLAY>(&pub, keys, values, &mut ctx);
        transfer::public_freeze_object(d);
        transfer::public_freeze_object(pub);
    }

    #[test]
    #[expected_failure(abort_code = package::E_WRONG_PACKAGE)]
    /// A publisher claimed in a foreign package must not mint a Display for a
    /// type it does not own. The forged publisher stands in for one claimed by
    /// another package: `type_name` is compiler-derived in `package::new`, so
    /// a real foreign claim produces exactly this foreign name.
    fun create_refuses_foreign_publisher() {
        let ctx = tx_context::dummy();
        let fake =
            package::forged_publisher_for_test<DISPLAY>(b"0x44::other::OTHER", &mut ctx);
        let d = create_empty<DISPLAY, DISPLAY>(&fake, &mut ctx);
        share<DISPLAY>(d);
        transfer::public_freeze_object(fake);
    }

    #[test]
    #[expected_failure(abort_code = package::E_WRONG_PACKAGE)]
    /// Same guard on the mutation path: a foreign publisher cannot edit a
    /// Display it did not create either.
    fun add_refuses_foreign_publisher() {
        let ctx = tx_context::dummy();
        let real = package::new<DISPLAY>(DISPLAY {}, &mut ctx);
        let d = create_empty<DISPLAY, DISPLAY>(&real, &mut ctx);
        let fake =
            package::forged_publisher_for_test<DISPLAY>(b"0x44::other::OTHER", &mut ctx);
        add<DISPLAY, DISPLAY>(&mut d, &fake, b"name", b"spoofed");
        // Unreachable when the abort above fires, but the compiler requires
        // every path to consume the value.
        let Display { id, fields } = d;
        let _ = fields;
        object::delete(id);
        transfer::public_freeze_object(real);

        package::revoke_forever<DISPLAY>(fake);
    }
}
