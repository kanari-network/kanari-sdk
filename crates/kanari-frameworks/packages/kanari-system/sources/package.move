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
/// has no such registry, so `display.move` documents the caveat.
module kanari_system::package {
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context;
    use kanari_system::types;

    /// Type T is aborted when the witness does not follow the one-time-witness
    /// convention.
    const E_NOT_ONE_TIME_WITNESS: u64 = 0;

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

    #[test_only]
    /// One-time witness named after this module, so
    /// `types::is_one_time_witness` accepts it.
    struct PACKAGE has drop {}

    #[test_only]
    /// Deliberately not named after its module, so it is not a valid witness.
    struct NOT_A_WITNESS has drop {}

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
}
