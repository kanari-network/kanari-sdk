// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// KariKid example using the framework KRC-721 NFT type and lifecycle.
module james::nft {
    use std::option;
    use std::string::{Self, String};
    use kanari_system::collection;
    use kanari_system::deny_list::DenyList;
    use kanari_system::krc_721::{Self, Nft};
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    use kanari_system::url;
    #[test_only]
    use std::vector;

    /// Structure for defining ownership (One-Time Witness).
    struct NFT has drop {}

    const MAX_SUPPLY: u64 = 2000;

    /// Abort code when the recipient is denied or KariKid transfers pause.
    const EDENIED: u64 = 1;

    /// Create the collection during publication. The publisher receives the
    /// collection and its mint cap.
    public fun init(_otw: NFT, ctx: &mut TxContext) {
        let name = b"KariKid Collection";
        let description = b"The first official standardized NFT on Kanari Network.";
        let banner = b"https://avatars.githubusercontent.com/u/127471673?s=800&v=4";
        let website = b"https://james-project.com";
        let sender = kanari_system::tx_context::sender(ctx);
        let (coll, issuer) =
            collection::create_collection(
                ctx,
                name,
                description,
                banner,
                website,
                MAX_SUPPLY
            );
        transfer::public_transfer(issuer, sender);
        transfer::public_transfer(coll, sender);
    }

    /// Replaces collection artwork without recreating the collection.
    public entry fun update_collection_metadata(
        coll: &mut collection::Collection,
        banner: vector<u8>,
        website: vector<u8>,
        ctx: &mut TxContext
    ) {
        collection::update_metadata(coll, banner, website, ctx);
    }

    /// Mint a KRC-721 KariKid. The collection assigns the historical mint
    /// ordinal; callers cannot choose or reuse a token number.
    public entry fun mint(
        cap: &mut collection::NftCap,
        coll: &collection::Collection,
        name: vector<u8>,
        description: vector<u8>,
        image_url: vector<u8>,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
        ctx: &mut TxContext
    ) {
        let nft =
            krc_721::mint(
                cap,
                coll,
                string::utf8(name),
                option::some(string::utf8(description)),
                option::some(url::new_unsafe_from_bytes(image_url)),
                attribute_keys,
                attribute_values,
                ctx
            );
        krc_721::transfer(nft, kanari_system::tx_context::sender(ctx));
    }

    /// Burn a token through the framework so its collection and supply cap are
    /// validated together.
    public entry fun burn(
        cap: &mut collection::NftCap, coll: &collection::Collection, nft: Nft
    ) {
        krc_721::burn(cap, coll, nft);
    }

    /// Regulated transfer through the shared framework deny list.
    public entry fun transfer(
        list: &DenyList, nft: Nft, recipient: address, ctx: &mut TxContext
    ) {
        transfer_with_list(nft, recipient, list, ctx);
    }

    /// Same transfer against an explicitly passed list for unit tests.
    public fun transfer_with_list(
        nft: Nft, recipient: address, deny_list: &DenyList, _ctx: &mut TxContext
    ) {
        let collection_id = krc_721::collection_of(&nft);
        assert!(
            !krc_721::is_denied(deny_list, collection_id, recipient),
            EDENIED
        );
        assert!(!krc_721::is_paused(deny_list, collection_id), EDENIED);
        krc_721::transfer(nft, recipient);
    }

    /// Replace token attributes using the framework's validation and limits.
    public entry fun update_attributes(
        nft: &mut Nft,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
        _ctx: &mut TxContext
    ) {
        krc_721::set_attributes(nft, attribute_keys, attribute_values);
    }

    /// Add an address to this collection's KRC-721 deny list.
    public entry fun deny_add(
        deny_list: &mut DenyList,
        cap: &collection::NftCap,
        addr: address,
        ctx: &mut TxContext
    ) {
        krc_721::deny_add(deny_list, cap, addr, ctx);
    }

    /// Remove an address from this collection's KRC-721 deny list.
    public entry fun deny_remove(
        deny_list: &mut DenyList,
        cap: &collection::NftCap,
        addr: address,
        _ctx: &mut TxContext
    ) {
        krc_721::deny_remove(deny_list, cap, addr);
    }

    /// Pause transfers for this collection.
    public entry fun deny_pause(
        deny_list: &mut DenyList, cap: &collection::NftCap, ctx: &mut TxContext
    ) {
        krc_721::deny_pause(deny_list, cap, ctx);
    }

    /// Resume transfers for this collection.
    public entry fun deny_unpause(
        deny_list: &mut DenyList, cap: &collection::NftCap, _ctx: &mut TxContext
    ) {
        krc_721::deny_unpause(deny_list, cap);
    }

    /// Return whether an address is denied for a collection.
    public fun is_denied(
        deny_list: &DenyList, collection_id: address, addr: address
    ): bool {
        krc_721::is_denied(deny_list, collection_id, addr)
    }

    /// Return whether transfers are paused for a collection.
    public fun is_paused(deny_list: &DenyList, collection_id: address): bool {
        krc_721::is_paused(deny_list, collection_id)
    }

    #[test_only]
    /// Mint a KRC-721 token directly for tests that need to hold the value.
    public fun mint_for_test(
        cap: &mut collection::NftCap,
        coll: &collection::Collection,
        ctx: &mut TxContext
    ): Nft {
        krc_721::mint(
            cap,
            coll,
            string::utf8(b"Test Kid"),
            option::some(string::utf8(b"test nft")),
            option::some(
                url::new_unsafe_from_bytes(b"https://james-project.com/test.png")
            ),
            vector::empty(),
            vector::empty(),
            ctx
        )
    }
}

