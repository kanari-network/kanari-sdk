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
/// Every mutation takes an *unbound* `&Publisher`, matching Sui. Binding it to
/// `&Publisher` would be tighter in principle, but the publisher is keyed on
/// the one-time witness type rather than on `T` (see `kanari_system::package`),
/// so there is no `Publisher` to bind to. Holding a publisher proves you own
/// a type in your own package.
///
/// Sui additionally enforces one Display per type through a host-side registry,
/// so a second Display for the same `T` is rejected at publish time. This
/// runtime now does the same: `new_display` claims `T` in the host's durable
/// type registry, so the first Display created for a type is the only one that
/// can ever exist. Combined with the unbound `&Publisher<W>` argument, that
/// closes the gap — a publisher for *some* type can no longer mint a
/// `Display<T>` once the real owner has already created one.
module kanari_system::display {
    use std::vector;
    use std::option::{Self, Option};
    use std::string::{String, utf8};
    use kanari_system::object::{Self, ID, UID};
    use kanari_system::package::Publisher;
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    use kanari_system::types;
    use kanari_system::vec_map::{Self, VecMap};

    #[test_only]
    use kanari_system::package;

    #[test_only]
    use kanari_system::tx_context;

    /// The key/value seed vectors had different lengths.
    const E_FIELD_COUNT_MISMATCH: u64 = 0;

    /// `T`'s own type-name entry cannot be removed.
    const E_PROTECTED_FIELD: u64 = 1;

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
    fun new_display<T: key, W: drop>(
        _pub: &Publisher<W>,
        keys: vector<vector<u8>>,
        values: vector<vector<u8>>,
        ctx: &mut TxContext,
    ): Display<T> {
        // One Display per type, enforced by the host and persisted across
        // transactions.
        types::claim_display<T>();
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
        _pub: &Publisher<W>,
        name: vector<u8>,
        value: vector<u8>,
    ) {
        vec_map::insert(&mut display.fields, utf8(name), utf8(value));
    }

    /// Remove one field. `T`'s own type-name entry is protected: indexers key
    /// off it, and losing it would make the display unattributable.
    public fun remove<T: key, W: drop>(
        display: &mut Display<T>,
        _pub: &Publisher<W>,
        name: vector<u8>,
    ) {
        let key = utf8(name);
        assert!(key != utf8(type_name_bytes<T>()), E_PROTECTED_FIELD);
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
    public fun destroy<T: key, W: drop>(display: Display<T>, _pub: &Publisher<W>) {
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

        assert!(*vec_map::get(&d.fields, &utf8(b"name")) == utf8(b"My Collection"), 1);
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
        assert!(option::is_none(&get<DISPLAY>(&d, b"nope")), 6);
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
}
