module james::james {

    use kanari_system::coin;
    use kanari_system::coin::{Coin, TreasuryCap};
    use kanari_system::tx_context::{Self, TxContext};
    use std::string;
    use std::ascii;
    use std::option;
    use kanari_system::transfer;

    /// Name of the coin
    struct JAMES has drop {}

    /// Module initializer - runs once when module is published
    /// Creates currency and transfers TreasuryCap to publisher
    fun init(witness: JAMES, ctx: &mut TxContext) {
        let (treasury_cap, metadata) = coin::create_currency(
            witness,
            9,  // decimals
            ascii::string(b"JAMES"),
            string::utf8(b"JAMES Network Coin"),
            string::utf8(b"The JAMES token for the Kanari network"),
            option::none(),
            ctx
        );
        
        // Freeze metadata so it can't be changed
        transfer::public_freeze_object(metadata);
        
        // Transfer TreasuryCap to publisher
        // Publisher can use this to mint tokens later via CLI
        transfer::public_transfer(treasury_cap, tx_context::sender(ctx));
    }

    /// Setup function that can be called from CLI
    /// This creates the currency without requiring witness/TxContext parameters  
    public entry fun setup(ctx: &mut TxContext) {
        init(JAMES {}, ctx);
    }

    /// Mint new JAMES tokens
    /// Only the holder of TreasuryCap can call this
    /// Usage: kanari move call --function mint --args <amount> <recipient>
    public entry fun mint(
        treasury_cap: &mut TreasuryCap<JAMES>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        let coin = coin::mint(treasury_cap, amount, ctx);
        transfer::public_transfer(coin, recipient);
    }

    /// Transfer JAMES tokens from sender to recipient
    /// This requires the sender to have a Coin<JAMES> object
    public entry fun transfer(c: coin::Coin<JAMES>, recipient: address) {
        transfer::public_transfer(c, recipient)
    }

    /// Burns JAMES tokens, decreasing total supply
    public entry fun burn(treasury_cap: &mut TreasuryCap<JAMES>, coin: Coin<JAMES>) {
        coin::burn(treasury_cap, coin);
    }
}
