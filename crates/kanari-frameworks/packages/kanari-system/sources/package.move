// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Type ownership, mirroring `sui::package`.
///
/// `Publisher<W>` is authority over the one-time witness type `W`, which the
/// defining module alone can construct. Sui keys the publisher on the witness
/// rather than on the object type, and so does this: it is the only arrangement
/// that keeps the token type free of `drop`.
///
/// The alternative — `Publisher<TokenType>` bound from the token itself — looks
/// tighter but is unusable. The witness has to *be* the type it claims, so the
/// token would need `drop`, and a droppable token can be silently discarded by
/// whoever holds it. That bypasses supply accounting entirely: a burned token
/// never returns its unit to the cap, and the destroy leaves no trace. Keeping
/// `drop` off the token is worth more than the extra binding.
///
/// The consequence, shared with Sui, is that consumers take an *unbound*
/// `&Publisher`: holding a publisher proves you own *some* type in your own
/// package, not that you own the specific `T` being configured. Sui closes the
/// remaining gap with a host-side one-Display-per-type registry; this runtime
/// has no such registry. What this module CAN close it with is package-level
/// binding: type names carry the defining package's address, so
/// `assert_governs_type` enforces at runtime that a publisher's witness was
/// defined in the same package as the type being configured. A publisher from
/// a foreign package cannot mint `Display<T>` or `TransferPolicy<T>` for your
/// `T`.
module kanari_system::package {
    use std::vector;
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context;
    use kanari_system::types;

    /// Type T is aborted when the witness does not follow the one-time-witness
    /// convention.
    const E_NOT_ONE_TIME_WITNESS: u64 = 0;

    /// A publisher was used to configure a type defined in a different
    /// package than the publisher's witness.
    const E_WRONG_PACKAGE: u64 = 1;

    /// Owns type `T`. The `phantom` marker means the value carries no data of
    /// `T`; the type parameter exists purely to scope the authority.
    struct Publisher<phantom T> has key, store {
        id: UID,
        /// Witness type name, retained for off-chain discovery of which types
        /// have a publisher published.
        type_name: vector<u8>,
    }

    /// Claim authority over `T`. `witness` must be the one-time witness `T`
    /// itself; only the defining module can construct it.
    public fun new<T: drop>(witness: T, ctx: &mut tx_context::TxContext): Publisher<T> {
        assert!(types::is_one_time_witness<T>(&witness), E_NOT_ONE_TIME_WITNESS);
        Publisher<T> {
            id: object::new(ctx),
            type_name: type_name_bytes<T>(),
        }
    }

    /// The publisher's object id.
    public fun id<T>(pub: &Publisher<T>): object::ID {
        object::uid_to_inner(&pub.id)
    }

    /// The publisher's object address.
    public fun publisher_address<T>(pub: &Publisher<T>): address {
        object::uid_address(&pub.id)
    }

    /// The witnessed type's name bytes, for indexers.
    public fun type_name<T>(pub: &Publisher<T>): vector<u8> {
        pub.type_name
    }

    /// Transfer authority over `T` to another address.
    public fun transfer<T>(pub: Publisher<T>, recipient: address) {
        transfer::public_transfer(pub, recipient);
    }

    /// Freeze the publisher permanently, so `T` can never gain new display or
    /// policy authority.
    public fun revoke_forever<T>(pub: Publisher<T>) {
        transfer::public_freeze_object(pub);
    }

    /// Canonical type-name bytes for `T`, matching `type_name::get<T>`.
    fun type_name_bytes<T>(): vector<u8> {
        let name = std::type_name::get<T>();
        *std::ascii::as_bytes(std::type_name::borrow_string(&name))
    }

    /// Abort unless `pub` was claimed for a type defined in the same package
    /// that defines `T`.
    ///
    /// Consumers of a publisher — `display`, `transfer_policy` — call this:
    /// without it, a package could claim a publisher for its own coin and then
    /// mint a `Display<Frog>` or `TransferPolicy<Frog>` for a token type it
    /// does not own. The check compares package addresses parsed from both
    /// type names (`0x43` in `0x43::pet::KanariPet`); it cannot detect
    /// same-package cross-module use, which is accepted, matching the
    /// statement in the module header.
    public fun assert_governs_type<T, W>(pub: &Publisher<W>) {
        assert!(same_package_as<T, W>(pub), E_WRONG_PACKAGE);
    }

    /// True when the witness that produced `pub` was defined in the same
    /// package that defines `T`. See `assert_governs_type`.
    public fun same_package_as<T, W>(pub: &Publisher<W>): bool {
        same_package_type_name(&pub.type_name, &type_name_bytes<T>())
    }

    /// True when two type names name types from the same package: their
    /// address sections, everything up to the first `::`, are byte-identical.
    fun same_package_type_name(a: &vector<u8>, b: &vector<u8>): bool {
        let al = package_prefix_len(a);
        let bl = package_prefix_len(b);
        if (al != bl) {
            return false
        };
        let i = 0;
        let same = true;
        while (same && i < al) {
            same = same && (*vector::borrow(a, i) == *vector::borrow(b, i));
            i = i + 1;
        };
        same
    }

    /// Length of the address section of a type name: `3` for `0x43` in
    /// `0x43::pet::KanariPet`. A name without a separator spans everything.
    fun package_prefix_len(name: &vector<u8>): u64 {
        let n = vector::length(name);
        let i = 0;
        while (i + 1 < n) {
            if (*vector::borrow(name, i) == 58 && *vector::borrow(name, i + 1) == 58) {
                break
            };
            i = i + 1;
        };
        if (i + 1 < n) { i } else { n }
    }

    #[test_only]
    /// One-time witness named after this module, so
    /// `types::is_one_time_witness` accepts it.
    struct PACKAGE has drop {}

    #[test_only]
    /// Deliberately not named after its module, so it is not a valid witness.
    struct NOT_A_WITNESS has drop {}

    #[test_only]
    /// A publisher with a hand-made `type_name`, standing in for a publisher
    /// claimed in a different package. `type_name` is only ever derived from
    /// the compiler-instance type in `new`, which is why the runtime check
    /// cannot be fooled; tests need a fake to simulate the foreign case.
    public fun forged_publisher_for_test<T>(
        name: vector<u8>,
        ctx: &mut tx_context::TxContext,
    ): Publisher<T> {
        Publisher<T> { id: object::new(ctx), type_name: name }
    }

    #[test]
    fun test_publisher_requires_witness_convention() {
        let ctx = tx_context::dummy();
        let good = new<PACKAGE>(PACKAGE {}, &mut ctx);
        let id = id<PACKAGE>(&good);
        let addr = publisher_address<PACKAGE>(&good);
        assert!(type_name<PACKAGE>(&good) == type_name_bytes<PACKAGE>(), 0);
        assert!(object::id_to_address(&id) == addr, 1);
        // A non-conforming struct is not a witness.
        assert!(
            !types::is_one_time_witness<NOT_A_WITNESS>(&NOT_A_WITNESS {}),
            2
        );
        revoke_forever(good);
    }

    #[test]
    #[expected_failure(abort_code = E_NOT_ONE_TIME_WITNESS)]
    fun test_publisher_rejects_non_witness() {
        let ctx = tx_context::dummy();
        // `NOT_A_WITNESS` is not named after its module, so it is not a valid
        // one-time witness and must not be able to claim a type.
        let bad = new<NOT_A_WITNESS>(NOT_A_WITNESS {}, &mut ctx);
        revoke_forever(bad);
    }

    #[test]
    /// A publisher claimed in this package governs types defined in this
    /// package — the byte that authorises `display` and `transfer_policy`.
    fun test_publisher_governs_same_package_types() {
        let ctx = tx_context::dummy();
        let pub = new<PACKAGE>(PACKAGE {}, &mut ctx);
        assert!(same_package_as<PACKAGE, PACKAGE>(&pub), 0);
        assert!(same_package_as<NOT_A_WITNESS, PACKAGE>(&pub), 1);
        assert_governs_type<PACKAGE, PACKAGE>(&pub);
        revoke_forever(pub);
    }

    #[test]
    #[expected_failure(abort_code = E_WRONG_PACKAGE)]
    /// The cross-package spoof `display` and `transfer_policy` guard against.
    /// Every struct here is in one package, so simulate the foreign case by
    /// handing the consumer a publisher whose `type_name` points elsewhere;
    /// that is exactly the value a foreign claim produces.
    fun test_publisher_does_not_govern_foreign_package_types() {
        let ctx = tx_context::dummy();
        let fake = forged_publisher_for_test<PACKAGE>(
            b"0x44::other::OTHER", &mut ctx,
        );
        assert_governs_type<PACKAGE, PACKAGE>(&fake);
        // Unreachable when the abort above fires, but the compiler requires
        // every path to consume the value.
        revoke_forever(fake);
    }
}
