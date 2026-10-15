// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0
module kanari_system::collection {
    use std::string::{String, utf8};
    use kanari_system::tx_context::{TxContext};
    use kanari_system::tx_context;
    use kanari_system::event;
    use kanari_system::object;
    use kanari_system::object::UID;
    use kanari_system::transfer;
    use kanari_system::url::Url;

    /// A reusable Collection resource for NFTs and similar objects.
    struct Collection has key, store {
        id: UID,
        name: String,
        description: String,
        banner_url: Url,
        website_url: Url,
        creator: address,
        max_supply: u64,
    }

    /// A capability resource that governs minting within a Collection.
    struct NftCap has key, store, drop {
        id: UID,
        remaining: u64,
        issued_counter: u64,
        collection_id: address,
    }

    /// Event emitted when a collection is created (for off-chain indexing)
    struct CollectionCreated has copy, drop {
        collection_id: address,
        creator: address,
        max_supply: u64,
    }

    const E_NO_SUPPLY: u64 = 1;
    const E_NOT_COLLECTION_CREATOR: u64 = 2;

    /// Cap does not belong to the given collection.
    const E_CAP_COLLECTION_MISMATCH: u64 = 3;

    /// Create a collection and its corresponding `NftCap`.
    /// Returns `(Collection, NftCap)` so callers can persist one or both.
    public fun create_collection(
        ctx: &mut TxContext,
        name: vector<u8>,
        description: vector<u8>,
        banner_url: vector<u8>,
        website_url: vector<u8>,
        max_supply: u64,
    ): (Collection, NftCap) {
        let id = object::new(ctx);
        let sender = tx_context::sender(ctx);

        let collection_addr = object::uid_address(&id);

        let coll = Collection {
            id,
            name: utf8(name),
            description: utf8(description),
            banner_url: kanari_system::url::new_unsafe_from_bytes(banner_url),
            website_url: kanari_system::url::new_unsafe_from_bytes(website_url),
            creator: sender,
            max_supply
        };

        let cap = NftCap {
            id: object::new(ctx),
            remaining: max_supply,
            issued_counter: 0,
            collection_id: collection_addr // use the fetched address
        };

        event::emit(
            CollectionCreated {
                collection_id: collection_addr,
                creator: sender,
                max_supply
            }
        );

        (coll, cap)
    }

    /// Returns the address (UID) of a `Collection`.
    public fun collection_id(_c: &Collection): address {
        // Consumers can call `object::uid_address(&c.id)` directly; keep API minimal.
        object::uid_address(&_c.id)
    }

    /// Returns the collection id stored in an `NftCap`.
    public fun cap_collection_id(cap: &NftCap): address {
        cap.collection_id
    }

    public fun collection_creator(c: &Collection): address {
        c.creator
    }

    public fun max_supply(c: &Collection): u64 {
        c.max_supply
    }

    /// Updates collection display metadata. Only the original collection creator
    /// may perform this mutation; supply and ownership are intentionally unchanged.
    public fun update_metadata(
        c: &mut Collection,
        banner_url: vector<u8>,
        website_url: vector<u8>,
        ctx: &TxContext,
    ) {
        assert!(tx_context::sender(ctx) == c.creator, E_NOT_COLLECTION_CREATOR);
        c.banner_url = kanari_system::url::new_unsafe_from_bytes(banner_url);
        c.website_url = kanari_system::url::new_unsafe_from_bytes(website_url);
        object::save_object(c);
    }

    public fun remaining(cap: &NftCap): u64 {
        cap.remaining
    }

    public fun issued(cap: &NftCap): u64 {
        cap.issued_counter
    }

    /// Consume one supply unit from the cap for minting.
    public fun consume_for_mint(cap: &mut NftCap) {
        assert!(cap.remaining > 0, E_NO_SUPPLY);
        cap.issued_counter = cap.issued_counter + 1;
        cap.remaining = cap.remaining - 1;
        object::save_object(cap);
    }

    /// Return one supply unit to cap (used on burn). Bound to the cap's own
    /// collection: the cap must belong to `collection` and `remaining` must
    /// stay within `max_supply`, otherwise a transferred cap could inflate
    /// mint allowance past the collection cap.
    public fun return_from_burn(cap: &mut NftCap, collection: &Collection) {
        assert!(
            cap.collection_id == object::uid_address(&collection.id),
            E_CAP_COLLECTION_MISMATCH
        );
        assert!(cap.remaining < collection.max_supply, E_NO_SUPPLY);
        cap.remaining = cap.remaining + 1;
        // Note: issued_counter is intentionally not decremented; it records how many
        // items have been minted historically.
        object::save_object(cap);
    }

    /// Get the creator of a collection.
    /// Transfer helpers using `transfer::public_transfer`.
    public fun transfer_collection(
        c: Collection,
        recipient: address,
        _ctx: &mut TxContext,
    ) {
        transfer::public_transfer(c, recipient)
    }

    public fun transfer_cap(
        cap: NftCap,
        recipient: address,
        _ctx: &mut TxContext,
    ) {
        transfer::public_transfer(cap, recipient)
    }

    #[test]
    fun test_collection_lifecycle() {
        let ctx = tx_context::dummy();

        // create collection with small supply
        let (coll, cap) =
            create_collection(
                &mut ctx,
                b"Test Name", // name
                b"Test Desc", // description
                b"https://banner", // banner_url
                b"https://web", // website_url
                2 // max_supply
            );

        // initial checks
        assert!(remaining(&cap) == 2, 0);
        assert!(collection_creator(&coll) == tx_context::sender(&ctx), 1);

        // mint one
        consume_for_mint(&mut cap);
        assert!(remaining(&cap) == 1, 2);
        assert!(issued(&cap) == 1, 3);

        // burn and return supply
        return_from_burn(&mut cap, &coll);
        assert!(remaining(&cap) == 2, 4);

        // transfer cap and collection to sender (moves objects)
        transfer_cap(cap, tx_context::sender(&ctx), &mut ctx);
        transfer_collection(coll, tx_context::sender(&ctx), &mut ctx);
    }

    #[test]
    #[expected_failure(abort_code = E_NO_SUPPLY)]
    fun test_return_from_burn_past_max_supply_aborts() {
        let ctx = tx_context::dummy();
        let (coll, cap) =
            create_collection(
                &mut ctx,
                b"N",
                b"D",
                b"https://banner",
                b"https://web",
                1
            );
        consume_for_mint(&mut cap);
        return_from_burn(&mut cap, &coll);
        // remaining == max_supply already: one more return would inflate
        // mint allowance past the collection cap.
        return_from_burn(&mut cap, &coll);
        transfer_cap(cap, tx_context::sender(&ctx), &mut ctx);
        transfer_collection(coll, tx_context::sender(&ctx), &mut ctx);
    }

    #[test]
    #[expected_failure(abort_code = E_CAP_COLLECTION_MISMATCH)]
    fun test_return_from_burn_wrong_collection_aborts() {
        let ctx = tx_context::dummy();
        let (coll_a, cap_a) =
            create_collection(
                &mut ctx,
                b"A",
                b"D",
                b"https://banner",
                b"https://web",
                10
            );
        let (coll_b, cap_b) =
            create_collection(
                &mut ctx,
                b"B",
                b"D",
                b"https://banner",
                b"https://web",
                10
            );
        consume_for_mint(&mut cap_a);
        // cap_a belongs to coll_a, not coll_b.
        return_from_burn(&mut cap_a, &coll_b);
        transfer_cap(cap_a, tx_context::sender(&ctx), &mut ctx);
        transfer_cap(cap_b, tx_context::sender(&ctx), &mut ctx);
        transfer_collection(coll_a, tx_context::sender(&ctx), &mut ctx);
        transfer_collection(coll_b, tx_context::sender(&ctx), &mut ctx);
    }

    // Test functions that require constructing a `TxContext` are executed
    // in the framework's higher-level test suites. Keep this package focused
    // on the Collection API surface.
}
