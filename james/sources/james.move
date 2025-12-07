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
    public entry fun init(witness: JAMES, ctx: &mut TxContext) {
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

    /// Test helper: Create a mock TreasuryCap for testing
    /// In production, use init() function instead
    public entry fun create_test_treasury(ctx: &mut TxContext) {
        // Create a witness - this works because JAMES has `drop` ability
        // Note: In Sui/Aptos, this would be prevented, but our VM allows it
        let witness = JAMES {};
        init(witness, ctx);
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


    /// Transfer a specific `amount` of JAMES from a mutable Coin held by the caller
    /// Usage: provide the caller's coin, the amount to send, and the recipient
    public entry fun transfer_amount(
        c: &mut coin::Coin<JAMES>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        let split_coin = coin::split(c, amount, ctx);
        transfer::public_transfer(split_coin, recipient);
    }


    /// Burn a specific `amount` of JAMES from a mutable Coin held by the caller
    /// Usage: provide the TreasuryCap, a mutable coin owned by caller, amount to burn, and tx context
    public entry fun burn_amount(
        treasury_cap: &mut TreasuryCap<JAMES>,
        c: &mut Coin<JAMES>,
        amount: u64,
        ctx: &mut TxContext
    ) {
        let to_burn = coin::split(c, amount, ctx);
        let _burned = coin::burn(treasury_cap, to_burn);
    }
}
