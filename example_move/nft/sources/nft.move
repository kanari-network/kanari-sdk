module nft::nft {
    /// Import the necessary modules
    use std::string;
    /// Import the necessary modules
    use kanari_framework::url::{Self, Url};
    /// Import the necessary modules
    use kanari_framework::object::{Self, UID, ID};
    /// Import the necessary modules
    use kanari_framework::tx_context::{sender,  TxContext};
    /// Import the necessary modules
    use std::string::{String, utf8};
    /// Import the necessary modules
    use kanari_framework::transfer;
    /// Import the necessary modules
    use kanari_framework::package;
    /// Import the necessary modules
    use kanari_framework::display;
    /// Import the necessary modules
    use kanari_framework::event;
    /// Import the necessary modules
    use kanari_framework::tx_context;

    /// The AdminCap struct represents the admin capabilities of the contract.
    struct AdminCap has key, store {
        id: UID
    }

    /// The SharedItem struct represents a shared item.
    struct SharedItem has key {
        id: UID
    }

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
        image_url: Url,
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
    struct NftCap has key, store {
        /// The ID of the NFT
        id: UID,
        /// The supply of the NFT
        supply: u64,
        /// The number of NFTs issued
        issued_counter: u64,
    }

    /// The MAX_SUPPLY constant represents the maximum supply of the NFT.
    const MAX_SUPPLY: u64 = 2000;

    /// The ETooManyNums constant represents the error code for too many NFTs.
    const ETooManyNums: u64 = 0;

    /// The init function initializes a KariKid NFT with certain properties.
    fun init(otw: NFT, ctx: &mut TxContext ) {

        let issuer = NftCap {
            id: object::new(ctx),
            supply: 0,
            issued_counter: 0,
        };

        // Keys for the properties of th`   e NFT
        let keys = vector[
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
        let values = vector[
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
            utf8(b"{crestor}"),
        ];

        // Claim the package
        let publisher = package::claim(otw, ctx);

        // Create a new display with the given fields
        let display = display::new_with_fields<KariKid>(
            &publisher, keys, values, ctx
        );

        // Transfer the display to the publisher
        transfer::share_object(SharedItem { id: object::new(ctx) });

        // Update the version of the display
        display::update_version(&mut display);

        transfer::transfer(issuer , sender(ctx));

        // Transfer the display to the publisher
        transfer::public_transfer(AdminCap { id: object::new(ctx) }, sender(ctx));

        // Transfer the display to the publisher
        transfer::public_transfer(publisher, sender(ctx));
        transfer::public_transfer(display, sender(ctx));

    }

    /// The MintEven struct represents an event that occurs when an NFT is minted.
    struct MintEven has copy, drop {
        // The ID of the NFT
        object_id: ID,
        // The name of the NFT
        name: String,
        // The number of the NFT
        number: String,
        // The address of the creator of the NFT
        crestor: address,
    }

    #[allow(lint(self_transfer))]
    /// The mint function mints a new KariKid NFT with the given properties.
    public fun mint(
        cap: &mut NftCap,
        _: &mut AdminCap,
        name: vector<u8>,
        description: vector<u8>,
        number: vector<u8>,
        url: vector<u8>,
        level: vector<String>,
        rarity: vector<String>,
        attack: vector<String>,
        defense: vector<String>,
        ctx: &mut TxContext
    ) {
        // The number of NFTs issued
        let n = cap.issued_counter;

        cap.issued_counter = n + 1;

        cap.supply = cap.supply + 1;

        assert!(cap.supply <= MAX_SUPPLY, ETooManyNums);


        // The sender of the transaction
        let sender = tx_context::sender(ctx);

        // The attributes of the NFT
        let attributes = Attributes {
            level,
            rarity,
            attack,
            defense,
        };

        // Create a new KariKid NFT
        let nft = KariKid {
            // The ID of the NFT
            id: object::new(ctx),
            // The name of the NFT
            name: utf8(name),
            // The description of the NFT
            description: utf8(description),
            // The number of the NFT
            number: utf8(number),
            // The URL of the image of the NFT
            image_url: url::new_unsafe_from_bytes(url),
            // The address of the creator of the NFT
            crestor: sender(ctx),
            // The attributes of the NFT
            attributes,
        };

        // Emit a MintEvent
        event::emit(MintEven{
            // The ID of the NFT
            object_id: object::id(&nft),
            // The name of the NFT
            crestor: sender,
            // The number of the NFT
            name: nft.name,
            // The address of the creator of the NFT
            number: nft.number,
        });

        // Transfer the NFT to the sender
        transfer::public_transfer(nft ,sender);
    }

    /// Burn NFT
    public entry fun burn(
        cap: &mut NftCap,
        nft: KariKid,
        _: &mut TxContext
    ) {
        cap.supply = cap.supply - 1;
        // The ID of the NFT
        let KariKid {
            id,
            // The name of the NFT
            name: _,
            // The description of the NFT
            description: _,
            // The number of the NFT
            number: _,
            // The URL of the image of the NFT
            image_url: _,
            // The address of the creator of the NFT
            crestor: _ ,
            // The attributes of the NFT
            attributes: _,
        } = nft;

        object::delete(id);
    }

    /// transfer Nft to Address
    public entry fun transfer(
        // The NFT to transfer
        nft: KariKid, recipient: address, _: &mut TxContext
    ) {
        // Transfer the NFT to the recipient
        transfer::public_transfer(nft, recipient)
    }

    /// Update the `description` of `nft` to `new_description`
    public entry fun update_description(
        // The NFT to update
        nft: &mut KariKid,
        // The new description
        new_description: vector<u8>,
        // The transaction context
        _: &mut TxContext
    ) {
        // Update the description of the NFT
        nft.description = string::utf8(new_description)
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

    /// The get_nft function returns the KariKid NFT with the given ID.
    public fun process_shared_item(
        _shared_item: &mut SharedItem,
    ) {
    // do nothing, we just want to test that we can execute
    // a transaction block containing a shared item
    }

}