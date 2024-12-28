module token::token {
    use std::option;
    use kanari_framework::coin;
    use kanari_framework::coin::{TreasuryCap, Coin, DenyCap};
    use kanari_framework::deny_list::DenyList;
    // use kanari_framework::object::UID;
    use kanari_framework::transfer;
    use kanari_framework::tx_context;
    use kanari_framework::tx_context::TxContext;
    use kanari_framework::url;

    // Struct representing the COIN token
    struct TOKEN has drop {}

    // Struct representing a minting event of COIN tokens
    struct Mint has copy, drop {
        amount: u64, // Amount of COIN tokens minted
        sender: address, // Address of the minter
    }

    // Function to initialize the COIN governance token
    fun init(witness: TOKEN, ctx: &mut TxContext) {
        // Create the COIN governance token with 9 decimals
        let (treasury, denycap, metadata) = coin::create_regulated_currency<TOKEN>(
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
        transfer::public_transfer(treasury, sender,);
        transfer::public_transfer(denycap, sender);


        // Freeze the metadata object
        transfer::public_freeze_object(metadata);
    }

    // Function to mint COIN tokens
    entry public fun mint(
        cap: &mut TreasuryCap<TOKEN>,
        amount: u64,
        sender: address,
        ctx: &mut TxContext,
    ) {
        let mint = Mint {
            amount,
            sender,
        };
        // Mint and transfer the minted COIN tokens to the sender
        coin::mint_and_transfer(cap, mint.amount,mint.sender, ctx);
    }

    // Function to burn COIN tokens
    public entry fun burn(
        cap: &mut TreasuryCap<TOKEN>,
        coin: Coin<TOKEN>,
    ) {
        coin::burn(cap, coin);
    }

    // Function to add an address to the deny list
    public entry fun deny_list_add_admin(
        denylist: &mut DenyList,
        denycap: &mut DenyCap<TOKEN>,
        sender: address,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_add(denylist, denycap, sender, ctx);
    }
}