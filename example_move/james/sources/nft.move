// Kanari Labs NFT Module - Universal Standard Version
module james::nft {
    use std::ascii;
    use std::string::{String, utf8};
    use std::type_name;
    use kanari_system::deny_list::{Self, DenyList};
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

    /// Deny-list bag index for the KariKid family. Coin entries live under
    /// `deny_list::coin_index()` (0); object families use their own index so
    /// the two never collide.
    const NFT_DENY_INDEX: u64 = 1;

    /// Abort code when the recipient is denied or KariKid transfers pause.
    const EDENIED: u64 = 1;

    /// Canonical deny-list key for KariKid: its `type_name` bytes, mirroring
    /// `coin::deny_type_key` so tooling can reproduce the exact format.
    fun nft_type_key(): vector<u8> {
        let name = type_name::get<KariKid>();
        *ascii::as_bytes(type_name::borrow_string(&name))
    }

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

    /// NFT transfer (regulated): aborts `EDENIED` when `recipient` is on the
    /// KariKid deny list or KariKid transfers are globally paused. The
    /// shared `DenyList` singleton (`@0x403`) is borrowed by the framework,
    /// so callers pass no deny-list argument.
    public entry fun transfer(
        nft: KariKid,
        recipient: address,
        ctx: &mut TxContext,
    ) {
        transfer_with_list(nft, recipient, deny_list::borrow_singleton(), ctx);
    }

    /// Same transfer against an explicitly passed list. Used by unit tests,
    /// which run without a genesis singleton to borrow.
    public fun transfer_with_list(
        nft: KariKid,
        recipient: address,
        deny_list: &DenyList,
        ctx: &mut TxContext,
    ) {
        let _ = ctx;
        assert!(!is_denied(deny_list, recipient), EDENIED);
        assert!(!is_paused(deny_list), EDENIED);
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
        nft.attribute_values = attribute_values;
    }

    // ==========================================
    // Deny-list management (regulated NFT)
    // ==========================================
    /// The shared `deny_list::v1_*` functions are permissionless at the
    /// framework level, so this module gates every write with
    /// `&collection::NftCap`: only the collection issuer can maintain
    /// KariKid entries — the same role separation `coin::DenyCap` gives
    /// regulated coins.

    /// Add `addr` to the KariKid deny list. Only the NftCap holder.
    public entry fun deny_add(
        deny_list: &mut DenyList,
        _cap: &collection::NftCap,
        addr: address,
        ctx: &mut TxContext,
    ) {
        deny_list::v1_add(deny_list, NFT_DENY_INDEX, nft_type_key(), addr, ctx);
    }

    /// Remove `addr` from the KariKid deny list. Aborts when not denied.
    public entry fun deny_remove(
        deny_list: &mut DenyList,
        _cap: &collection::NftCap,
        addr: address,
    ) {
        deny_list::v1_remove(deny_list, NFT_DENY_INDEX, nft_type_key(), addr);
    }

    /// Pause all KariKid transfers (fail-closed kill-switch).
    public entry fun deny_pause(
        deny_list: &mut DenyList,
        _cap: &collection::NftCap,
        ctx: &mut TxContext,
    ) {
        deny_list::v2_enable_global_pause(deny_list, NFT_DENY_INDEX, nft_type_key(), ctx);
    }

    /// Lift the KariKid pause.
    public entry fun deny_unpause(
        deny_list: &mut DenyList,
        _cap: &collection::NftCap,
    ) {
        deny_list::v2_disable_global_pause(deny_list, NFT_DENY_INDEX, nft_type_key());
    }

    /// Open read: true iff `addr` is denied for KariKid.
    public fun is_denied(deny_list: &DenyList, addr: address): bool {
        deny_list::v1_contains(deny_list, NFT_DENY_INDEX, nft_type_key(), addr)
    }

    /// Open read: true iff KariKid transfers are globally paused.
    public fun is_paused(deny_list: &DenyList): bool {
        deny_list::v2_is_global_pause_enabled(deny_list, NFT_DENY_INDEX, nft_type_key())
    }

    #[test_only]
    /// Mint a KariKid straight into the caller's hands for tests (the
    /// `mint` entry transfers to the tx sender, which unit tests cannot
    /// retrieve objects from).
    public fun mint_for_test(
        cap: &mut collection::NftCap,
        ctx: &mut TxContext,
    ): KariKid {
        collection::consume_for_mint(cap);
        KariKid {
            id: object::new(ctx),
            name: utf8(b"Test Kid"),
            image_url: url::new_unsafe_from_bytes(b"https://james-project.com/test.png"),
            description: utf8(b"test nft"),
            attribute_keys: vector[],
            attribute_values: vector[],
            number: utf8(b"0"),
            collection_id: collection::cap_collection_id(cap),
            creator: tx_context::sender(ctx),
        }
    }
}
