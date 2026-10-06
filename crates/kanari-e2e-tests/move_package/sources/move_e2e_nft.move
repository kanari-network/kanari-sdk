module kanari_e2e_tests::move_e2e_nft {
    use std::string::{Self, String};
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    const E_NOT_OWNER: u64 = 500;
    const E_BAD_SIZE: u64 = 501;

    /// Ownable NFT with metadata fields.
    public struct NFT has key, store {
        id: UID,
        name: String,
        description: String,
        uri: String,
        creator: address
    }

    /// Shared collection tracking minted NFT addresses.
    public struct NFTCollection has key, store {
        id: UID,
        name: String,
        description: String,
        nfts: vector<address>,
        owner: address
    }

    /// Create a shared collection and return its address.
    public fun create_collection(
        name: vector<u8>, description: vector<u8>, ctx: &mut TxContext
    ): address {
        let collection = e2e_new_collection(name, description, ctx);
        let addr = object::uid_address(&collection.id);
        transfer::share_object(collection);
        addr
    }

    /// Owner-only mint: creates the NFT, registers it, and transfers it to the sender.
    public fun mint_nft(
        collection: &mut NFTCollection,
        name: vector<u8>,
        description: vector<u8>,
        uri: vector<u8>,
        ctx: &mut TxContext
    ): address {
        let sender = tx_context::sender(ctx);
        assert!(collection.owner == sender, E_NOT_OWNER);
        let nft = NFT {
            id: object::new(ctx),
            name: string::utf8(name),
            description: string::utf8(description),
            uri: string::utf8(uri),
            creator: sender
        };
        let nft_addr = object::uid_address(&nft.id);
        vector::push_back(&mut collection.nfts, nft_addr);
        transfer::public_transfer(nft, sender);
        nft_addr
    }

    /// Transfer an owned NFT to a new recipient.
    public entry fun transfer_nft(nft: NFT, recipient: address) {
        transfer::public_transfer(nft, recipient);
    }

    /// Burn an owned NFT and return its former address.
    public fun burn_nft(nft: NFT): address {
        let addr = object::uid_address(&nft.id);
        let NFT { id, name: _, description: _, uri: _, creator: _ } = nft;
        object::delete(id);
        addr
    }

    public fun nft_name(nft: &NFT): &String {
        &nft.name
    }

    public fun nft_uri(nft: &NFT): &String {
        &nft.uri
    }

    public fun nft_creator(nft: &NFT): address {
        nft.creator
    }

    public fun collection_size(collection: &NFTCollection): u64 {
        vector::length(&collection.nfts)
    }

    public fun collection_name(collection: &NFTCollection): &String {
        &collection.name
    }

    /// Builds an unshared collection owned by the sender.
    fun e2e_new_collection(
        name: vector<u8>, description: vector<u8>, ctx: &mut TxContext
    ): NFTCollection {
        NFTCollection {
            id: object::new(ctx),
            name: string::utf8(name),
            description: string::utf8(description),
            nfts: vector::empty<address>(),
            owner: tx_context::sender(ctx)
        }
    }

    /// Entry wrapper for `create_collection`.
    public entry fun e2e_create_collection(
        name: vector<u8>, description: vector<u8>, ctx: &mut TxContext
    ) {
        create_collection(name, description, ctx);
    }

    /// Entry wrapper for `mint_nft`.
    public entry fun e2e_mint_nft(
        collection: &mut NFTCollection,
        name: vector<u8>,
        description: vector<u8>,
        uri: vector<u8>,
        ctx: &mut TxContext
    ) {
        mint_nft(collection, name, description, uri, ctx);
    }

    /// Entry wrapper for `burn_nft`.
    public entry fun e2e_burn_nft(nft: NFT) {
        burn_nft(nft);
    }

    /// Entry wrapper asserting the collection size.
    public entry fun e2e_check_collection_size(
        collection: &NFTCollection, expected: u64
    ) {
        assert!(collection_size(collection) == expected, E_BAD_SIZE);
    }

    /// Entry wrapper asserting the NFT creator.
    public entry fun e2e_check_nft_creator(nft: &NFT, expected: address) {
        assert!(nft_creator(nft) == expected, E_BAD_SIZE);
    }

    /// Entry wrapper asserting the collection name.
    public entry fun e2e_check_collection_name(
        collection: &NFTCollection, expected: vector<u8>
    ) {
        assert!(*collection_name(collection) == string::utf8(expected), E_BAD_SIZE);
    }

    /// Mints two NFTs into a shared collection, asserted in-transaction.
    public entry fun e2e_nft_roundtrip(ctx: &mut TxContext) {
        let mut collection = e2e_new_collection(
            b"E2E Collection", b"collection for e2e tests", ctx
        );
        assert!(collection_size(&collection) == 0, E_BAD_SIZE);

        mint_nft(
            &mut collection,
            b"Genesis",
            b"first nft",
            b"https://kanari.example/1.png",
            ctx
        );
        assert!(collection_size(&collection) == 1, E_BAD_SIZE);

        mint_nft(
            &mut collection,
            b"Second",
            b"second nft",
            b"https://kanari.example/2.png",
            ctx
        );
        assert!(collection_size(&collection) == 2, E_BAD_SIZE);

        transfer::share_object(collection);
    }

    /// Creates a shared collection owned by the sender so foreign mints abort.
    public entry fun e2e_create_owned_collection(
        name: vector<u8>, description: vector<u8>, ctx: &mut TxContext
    ) {
        let collection = e2e_new_collection(name, description, ctx);
        transfer::share_object(collection);
    }
}