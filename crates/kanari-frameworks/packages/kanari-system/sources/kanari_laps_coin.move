module kanari_system::kanari_laps_coin {
    use std::ascii;
    use std::option;
    use std::string;
    use kanari_system::coin;
    use kanari_system::coin::{TreasuryCap, Coin};
    use kanari_system::deny_list::{DenyList, DenyCap};
    use kanari_system::object::UID;
    use kanari_system::transfer;
    use kanari_system::tx_context;
    use kanari_system::tx_context::TxContext;
    use kanari_system::url;

    // Struct representing the COIN token
    struct COIN has drop {}

    // Struct representing a borrower of COIN tokens
    struct Borrower has copy, drop {
        amount: u64, // Amount of COIN tokens borrowed
        sender: address, // Address of the borrower
    }

    // Struct representing a minting event of COIN tokens
    struct Mint has copy, drop {
        amount: u64, // Amount of COIN tokens minted
        sender: address, // Address of the minter
    }

    // Function to initialize the COIN governance token
    public fun init(witness: COIN, ctx: &mut TxContext) {
        // Create the COIN governance token with 9 decimals
        let (treasury, denycap, metadata) = coin::create_regulated_currency<COIN>(
            witness,
            9,
            b"KARI",
            b"Kanari Token",
            b"The governance token of Kanari Network",
            option::some(url::new_unsafe_from_bytes(b"https://magenta-able-pheasant-388.mypinata.cloud/ipfs/QmNVQ3LQSbLC8bJDnXrbuftf2dC7LWJp4oXVkXxVRrDRfk")),
            ctx
        );
        // Get the sender of the transaction
        let sender = tx_context::sender(ctx);

        // Transfer the treasury and denycap objects to the sender
        transfer::public_transfer(treasury, sender);
        transfer::public_transfer(denycap, sender);


        // Freeze the metadata object
        transfer::public_freeze_object(metadata);
    }

    // Function to get the total supply of COIN tokens (reads from TreasuryCap)
    public fun total_supply(cap: &TreasuryCap<COIN>) : u64 {
        coin::total_supply<COIN>(cap)
    }

    // Function to borrow COIN tokens
    public entry fun borrow(
        cap: &mut TreasuryCap<COIN>,
        amount: u64,
        sender: address,
        ctx: &mut TxContext,
    ) {
        let borrow = Borrower {
            amount,
            sender,
        };
        // Mint and transfer the borrowed COIN tokens to the borrower
        coin::mint_and_transfer<COIN>(cap, borrow.amount, borrow.sender, ctx);
    }

    // Function to mint COIN tokens
    public entry fun mint(
        cap: &mut TreasuryCap<COIN>,
        amount: u64,
        sender: address,
        ctx: &mut TxContext,
    ) {
        let mint = Mint {
            amount,
            sender,
        };
        // Mint and transfer the minted COIN tokens to the sender
        coin::mint_and_transfer<COIN>(cap, mint.amount, mint.sender, ctx);
    }

    // Function to burn COIN tokens
    public entry fun burn(
        cap: &mut TreasuryCap<COIN>,
        c: Coin<COIN>,
    ) {
        coin::burn<COIN>(cap, c);
    }

    // Function to add an address to the deny list
    public entry fun deny_list_add_admin(
        denylist: &mut DenyList,
        denycap: &mut DenyCap<COIN>,
        sender: address,
        ctx: &mut TxContext,
    ) {
        kanari_system::deny_list::deny_list_add<COIN>(denylist, denycap, sender, ctx);
    }

    // Function to remove an address from the deny list
    public entry fun deny_list_remove_admin(
        denylist: &mut DenyList,
        denycap: &mut DenyCap<COIN>,
        sender: address,
        ctx: &mut tx_context::TxContext,
    ) {
        kanari_system::deny_list::deny_list_remove<COIN>(denylist, denycap, sender, ctx);
    }

    // Function to destroy zero COIN tokens
    public  entry fun destroy_zero(
        c: Coin<COIN>
    ) {
        // Only allow destroying coins whose value is zero
        let v = coin::value(&c);
        assert!(v == 0, 1);
        let _ = coin::into_balance(c);
    }

    // Function to transfer COIN tokens to a recipient
    public entry fun transfer(c: coin::Coin<COIN>, recipient: address) {
        transfer::public_transfer(c, recipient);
    }

    // Struct representing the metadata of the COIN token
    struct CoinMetadata has store, key {
        id: UID,
        decimals: u8, // Number of decimal places the coin uses
        name: string::String, // Name for the token
        symbol: ascii::String, // Symbol for the token
        description: string::String, // Description of the token
        icon_url: option::Option<url::Url>, // URL for the token logo
    }

    // Function to update the name of the KARI token
    public entry fun update_name(
        _treasury: &TreasuryCap<COIN>,
        metadata: &mut CoinMetadata,
        name: string::String
    ) {
        metadata.name = name;
    }

    // Function to update the symbol of the COIN token
    public entry fun update_symbol(
        _treasury: &TreasuryCap<COIN>,
        metadata: &mut CoinMetadata,
        symbol: ascii::String
    ) {
        metadata.symbol = symbol;
    }

    // Function to update the description of the COIN token
    public entry fun update_description(
        _treasury: &TreasuryCap<COIN>,
        metadata: &mut CoinMetadata,
        description: string::String
    ) {
        metadata.description = description;
    }

    // Function to update the icon URL of the KARI token
    public entry fun update_icon_url(
        _treasury: &TreasuryCap<COIN>,
        metadata: &mut CoinMetadata,
        url: ascii::String
    ) {
        metadata.icon_url = option::some(url::new_unsafe(url));
    }

    // Function to get the number of decimal places of the KARI token
    public fun get_decimals(metadata: &CoinMetadata): u8 {
        metadata.decimals
    }
}
