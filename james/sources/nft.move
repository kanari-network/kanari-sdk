//Kanari Laps NFT Module
module james::nft {
    /// Import the necessary modules
    use std::string::{String, utf8};
    use kanari_system::tx_context::TxContext;
    use kanari_system::tx_context;
    use kanari_system::transfer;
    use kanari_system::url as url;
    use kanari_system::object;
    use kanari_system::object::UID;
    use kanari_system::collection;
    use kanari_system::event;

    /// The KARIKID struct is an empty struct used for some kind of initialization.
    struct NFT has drop {
    }

    /// The Attributes struct represents the attributes of a KariKid NFT.
    struct Attributes has store, drop {
        level: vector<String>,
        rarity: vector<String>,
        attack: vector<String>,
        defense: vector<String>,
    }

    /// The KariKid struct represents an NFT with various properties.
    struct KariKid has key, store {
        /// The ID of the NFT
        id: UID,
        /// The name of the NFT
        name: String,
        /// The URL of the image of the NFT
        image_url: url::Url,
        /// The URL of the image of the NFT
        description: String,
        /// The number of the NFT
        number: String,
        /// The collection this NFT belongs to
        collection_id: address,
        /// The address of the creator of the NFT
        creator: address,
        /// The attributes of the NFT
        attributes: Attributes
    }

    // The NftCap struct represents the capabilities of the NFT.
    // NftCap is provided by `kanari_system::collection::NftCap`.

    // Admin capability removed: minting no longer requires an AdminCap

    /// The MAX_SUPPLY constant represents the maximum supply of the NFT.
    const MAX_SUPPLY: u64 = 2000;

    /// Error codes

    /// The init function creates and returns a `(Collection, NftCap)`.
    public fun init(_otw: NFT, ctx: &mut TxContext): (collection::Collection, collection::NftCap) {

        let (coll, issuer) = collection::create_collection(ctx, b"default", b"", MAX_SUPPLY);


        // Keys for the properties of the NFT
        let _keys = vector[
            // The name of the NFT
            utf8(b"name"),
            // The link to the NFT
            utf8(b"link"),
            // The URL of the image of the NFT
            utf8(b"image_url"),
            // The description of the NFT
            utf8(b"description"),
            // The URL of the project
            utf8(b"project_url"),
            // The address of the creator of the NFT
            utf8(b"creator"),
        ];

        // Values for the properties of the NFT
        let _values = vector[
            // The name of the NFT
            utf8(b"{names}"),
            // The link to the NFT
            utf8(b"{link}"),
            // The URL of the image of the NFT
            utf8(b"{image_url}"),
            // The description of the NFT
            utf8(b"{description}"),
            // The URL of the project
            utf8(b"https://art.kanari.network"),
            // The address of the creator of the NFT
            utf8(b"{creator}"),
        ];

        // Return the created collection and capability so callers can persist them.
        (coll, issuer)

    }

    public entry fun setup(ctx: &mut TxContext) {
        let witness = NFT {};
        let (coll, issuer) = init(witness, ctx);
        let sender = tx_context::sender(ctx);
        transfer::public_transfer(issuer, sender);
        transfer::public_transfer(coll, sender);
    }

    /// Lightweight mint log for off-chain indexing (copyable)
    struct MintLog has copy, drop {
        object_id: address,
        creator: address,
        collection_id: address,
    }
    
    /// The mint function mints a new KariKid NFT with the given properties.
    public entry fun mint(
        cap: &mut collection::NftCap,
        name: vector<u8>,
        description: vector<u8>,
        number: vector<u8>,
        url_bytes: vector<u8>,
        level: vector<String>,
        rarity: vector<String>,
        attack: vector<String>,
        defense: vector<String>,
        ctx: &mut TxContext
    ) {
        // Use system collection API to consume supply
        collection::consume_for_mint(cap);

        let sender = tx_context::sender(ctx);

        let attributes = Attributes {
            level,
            rarity,
            attack,
            defense,
        };

        let nft = KariKid {
            id: object::new(ctx),
            name: utf8(name),
            description: utf8(description),
            number: utf8(number),
            image_url: url::new_unsafe_from_bytes(url_bytes),
            collection_id: collection::cap_collection_id(cap),
            creator: sender,
            attributes,
        };

        event::emit(MintLog {
            object_id: object::uid_address(&nft.id),
            creator: sender,
            collection_id: collection::cap_collection_id(cap),
        });

        transfer::public_transfer(nft, sender);
    }

    /// Burn NFT
    public entry fun burn(
        cap: &mut collection::NftCap,
        nft: KariKid,
        _: &mut TxContext
    ) {
        // Return supply when an NFT is burned via system API
        collection::return_from_burn(cap);
        let KariKid { id, name: _, description: _, number: _, image_url: _, collection_id: _, creator: _, attributes: _ } = nft;
        // consuming `id` (UID) here drops the resource
        let _ = id;
    }

    /// transfer Nft to Address
    public entry fun transfer(
        nft: KariKid, recipient: address, _: &mut TxContext
    ) {
        transfer::public_transfer(nft, recipient)
    }

    /// Update the `description` of `nft` to `new_description`
    public entry fun update_description(
        nft: &mut KariKid,
        new_description: vector<u8>,
        _: &mut TxContext
    ) {
        nft.description = utf8(new_description)
    }

    /// update the attributes of the NFT
    public entry fun update_attributes(
        nft: &mut KariKid,
        level: vector<String>,
        rarity: vector<String>,
        attack: vector<String>,
        defense: vector<String>,
        _: &mut TxContext
    ) {
        nft.attributes = Attributes {
            level,
            rarity,
            attack,
            defense,
        };
    }
}