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
        /// The address of the creator of the NFT
        crestor: address,
        /// The attributes of the NFT
        attributes: Attributes
    }

    /// The NftCap struct represents the capabilities of the NFT.
    struct NftCap has key, store, drop {
        /// The ID of the NFT
        id: UID,
        /// The supply of the NFT
        supply: u64,
        /// The number of NFTs issued
        issued_counter: u64,
    }

    // Admin capability removed: minting no longer requires an AdminCap

    /// The MAX_SUPPLY constant represents the maximum supply of the NFT.
    const MAX_SUPPLY: u64 = 2000;

    /// Error codes
    const ETooManyNums: u64 = 1;

    /// The init function creates and returns an `NftCap` for a collection.
    /// The returned capability has `supply = MAX_SUPPLY` (per-collection limit).
    public fun init(_otw: NFT, ctx: &mut TxContext): NftCap {

        let issuer = NftCap {
            id: object::new(ctx),
            supply: MAX_SUPPLY,
            issued_counter: 0,
        };


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
            utf8(b"crestor"),
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

        // Return the created capability so callers can persist it.
        issuer

    }

    public entry fun setup(ctx: &mut TxContext) {
        let witness = NFT {};
        let issuer = init(witness, ctx);
        let sender = tx_context::sender(ctx);
        transfer::public_transfer(issuer, sender);
    }

    /// The MintEvent struct represents an event that occurs when an NFT is minted.
    struct MintEvent has store, drop {
        object_id: address,
        name: String,
        number: String,
        crestor: address,
    }
    
    /// The mint function mints a new KariKid NFT with the given properties.
    public fun mint(
        cap: &mut NftCap,
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
        // Ensure there is remaining supply, then consume one and increment counter
        assert!(cap.supply > 0, ETooManyNums);
        let n = cap.issued_counter;
        cap.issued_counter = n + 1;
        cap.supply = cap.supply - 1;

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
            crestor: sender,
            attributes,
        };

        // (Optional) create a MintEvent object to allow off-chain indexing. We don't have
        // a global event system here, so we simply create and drop it.
        let _evt = MintEvent {
            object_id: object::uid_address(&nft.id),
            name: nft.name,
            number: nft.number,
            crestor: sender,
        };

        transfer::public_transfer(nft, sender);
    }

    /// Burn NFT
    public entry fun burn(
        cap: &mut NftCap,
        nft: KariKid,
        _: &mut TxContext
    ) {
        // Return supply when an NFT is burned
        cap.supply = cap.supply + 1;
        let KariKid { id, name: _, description: _, number: _, image_url: _, crestor: _, attributes: _ } = nft;
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