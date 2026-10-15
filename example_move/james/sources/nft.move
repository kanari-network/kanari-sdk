// Kanari Labs NFT Module - Universal Standard Version
module james::nft {
    use std::string::{String, utf8};
    use kanari_system::tx_context::TxContext;
    use kanari_system::tx_context;
    use kanari_system::transfer;
    use kanari_system::url as url;
    use kanari_system::object;
    use kanari_system::object::UID;
    use kanari_system::collection;
    use kanari_system::event;

    /// Structure for defining ownership (One-Time Witness)
    struct NFT has drop {}

    /// A universal, standard NFT structure
    struct KariKid has key, store {
        /// 1. Object ID (UID),
        id: UID,
        /// 2. NFT name,
        name: String,
        /// 3. Image URL,
        image_url: url::Url,
        /// 4. Description,
        description: String,
        /// 5. List of attribute keys (e.g., ["Level", "Attack"]),
        attribute_keys: vector<String>,
        /// 6. List of attribute values ​​(e.g., ["10", "150"]),
        attribute_values: vector<String>,
        // --- Additional fields beyond the standard ---,
        number: String,
        collection_id: address,
        creator: address,
    }

    /// Event สำหรับการ Mint
    struct MintLog has copy, drop {
        object_id: address,
        creator: address,
        collection_id: address,
    }

    const MAX_SUPPLY: u64 = 2000;

    #[allow(unused_function)]
    /// Function to create a collection; executed once by the runtime during publication
    /// (not re-executed during upgrades). The publisher receives the Collection and NftCap objects.
    public fun init(_otw: NFT, ctx: &mut TxContext) {
        let name = b"KariKid Collection";
        let description = b"The first official standardized NFT on Kanari Network.";
        // Use a durable, directly-loadable image URL. A custom HTTPS, ipfs://,
        // or data:image/jpeg;base64,... URI can be set after setup as well.
        let banner = b"https://avatars.githubusercontent.com/u/127471673?s=800&v=4";
        let website = b"https://james-project.com";
        let sender = tx_context::sender(ctx);
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

    /// Replaces collection artwork without recreating the collection. Only the
    /// original collection creator is authorized by collection::update_metadata.
    public entry fun update_collection_metadata(
        coll: &mut collection::Collection,
        banner: vector<u8>,
        website: vector<u8>,
        ctx: &mut TxContext,
    ) {
        collection::update_metadata(coll, banner, website, ctx);
    }

    /// Mint function according to international standards.
    public entry fun mint(
        cap: &mut collection::NftCap,
        name: vector<u8>,
        description: vector<u8>,
        url_bytes: vector<u8>,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
        number: vector<u8>,
        ctx: &mut TxContext,
    ) {
        collection::consume_for_mint(cap);
        let sender = tx_context::sender(ctx);
        let nft = KariKid {
            id: object::new(ctx),
            name: utf8(name),
            image_url: url::new_unsafe_from_bytes(url_bytes),
            description: utf8(description),
            attribute_keys,
            attribute_values,
            number: utf8(number),
            collection_id: collection::cap_collection_id(cap),
            creator: sender
        };
        event::emit(
            MintLog {
                object_id: object::uid_address(&nft.id),
                creator: sender,
                collection_id: collection::cap_collection_id(cap)
            }
        );
        transfer::public_transfer(nft, sender);
    }

    /// NFT Destruction (Burn)
    public entry fun burn(
        cap: &mut collection::NftCap,
        coll: &collection::Collection,
        nft: KariKid,
        _: &mut TxContext,
    ) {
        collection::return_from_burn(cap, coll);
        let KariKid {
            id,
            name: _,
            image_url: _,
            description: _,
            attribute_keys: _,
            attribute_values: _,
            number: _,
            collection_id: _,
            creator: _
        } = nft;
        object::delete(id);
    }

    /// NFT transfer
    public entry fun transfer(
        nft: KariKid,
        recipient: address,
        _: &mut TxContext,
    ) {
        transfer::public_transfer(nft, recipient)
    }

    /// Update Attributes (Attributes)
    public entry fun update_attributes(
        nft: &mut KariKid,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
        _: &mut TxContext,
    ) {
        nft.attribute_keys = attribute_keys;
        nft.attribute_values ​​= attribute_values;
    }
}
