module kanari_e2e_tests::move_e2e_nft {
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};
    use std::string::{Self, String};
    use std::vector;

    struct NFT has key, store {
        id: UID,
        name: String,
        description: String,
        uri: String,
        creator: address,
        owner: address,
    }

    struct NFTCollection has key {
        id: UID,
        name: String,
        description: String,
        nfts: vector<address>,
        owner: address,
    }

    public fun create_collection(name: vector<u8>, description: vector<u8>, ctx: &mut TxContext): address {
        let collection = NFTCollection {
            id: object::new(ctx),
            name: string::utf8(name),
            description: string::utf8(description),
            nfts: vector::empty<address>(),
            owner: tx_context::sender(ctx),
        };
        let addr = object::uid_to_address(&collection.id);
        transfer::share_object(collection);
        addr
    }

    public fun mint_nft(
        collection: &mut NFTCollection,
        name: vector<u8>,
        description: vector<u8>,
        uri: vector<u8>,
        ctx: &mut TxContext,
    ): address {
        assert!(collection.owner == tx_context::sender(ctx), 500);
        let nft = NFT {
            id: object::new(ctx),
            name: string::utf8(name),
            description: string::utf8(description),
            uri: string::utf8(uri),
            creator: tx_context::sender(ctx),
            owner: tx_context::sender(ctx),
        };
        let nft_addr = object::uid_to_address(&nft.id);
        vector::push_back(&mut collection.nfts, nft_addr);
        transfer::public_transfer(nft, tx_context::sender(ctx));
        nft_addr
    }

    public fun transfer_nft(nft: NFT, recipient: address, _ctx: &mut TxContext): NFT {
        let new_nft = NFT {
            id: nft.id,
            name: nft.name,
            description: nft.description,
            uri: nft.uri,
            creator: nft.creator,
            owner: recipient,
        };
        transfer::public_transfer(new_nft, recipient);
        nft
    }

    public fun get_nft_name(nft: &NFT): &String {
        &nft.name
    }

    public fun get_nft_owner(nft: &NFT): address {
        nft.owner
    }

    public fun get_collection_size(collection: &NFTCollection): u64 {
        vector::length(&collection.nfts)
    }
}
