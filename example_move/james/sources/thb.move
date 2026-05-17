module james::thb {

    use kanari_system::coin;
    use kanari_system::coin::{Coin, TreasuryCap};
    use kanari_system::tx_context::{TxContext};

    use std::option;
    use kanari_system::transfer;
    use kanari_system::url;

    use std::string;
    use std::ascii;
    
    /// Name of the coin
    struct THB has drop {}

    // ==========================================
    // 🟢 DAO Configuration
    // ==========================================
    
    /// DAO wallet address for collecting transfer fees (0.1%)
    const DAO_ADDRESS: address = @0x3141a487d7a5382bb435c0ad39a6060067765e60e45b50953a0050bcf24b03a3;
    
    /// Transfer fee rate: 0.1% = 1/1000 (in basis points: 10 out of 10000)
    const FEE_RATE_NUMERATOR: u64 = 1;
    const FEE_RATE_DENOMINATOR: u64 = 1000;

    /// Initialize and register the THB currency.
    /// Returns the `TreasuryCap<THB>` which can be used to mint tokens.
    /// This should be invoked once (e.g., during genesis or deployment).
    fun init(witness: THB ,ctx: &mut TxContext): (TreasuryCap<THB>, coin::CoinMetadata<THB>) {
        let (treasury, metadata) = coin::create_currency<THB>(
            witness,
            6,
            b"THB",
            b"THB Token",
            b"",
                        option::none<kanari_system::url::Url>(),
            ctx,
        );
        // Return both TreasuryCap and Metadata so callers can persist them.
        (treasury, metadata)
    }

    /// Public setup entry that creates the required `THB` witness,
    /// invokes `init`, and transfers the created objects to the
    /// transaction sender so they are persisted in the caller's account.
    public entry fun setup(ctx: &mut TxContext) {
        let witness = THB {};
        let (treasury, metadata) = init(witness, ctx);
        let sender = kanari_system::tx_context::sender(ctx);
        transfer::public_transfer(treasury, sender);
        transfer::public_transfer(metadata, sender);
    }


/// Mint new THB tokens
    /// Only the holder of TreasuryCap can call this
    /// Usage: kanari move call --function mint --args <amount> <recipient>
    /// 
    /// This function mints tokens directly to the recipient's address
    /// The runtime will automatically create or update the recipient's Coin object
    public entry fun mint(
    treasury_cap: &mut TreasuryCap<THB>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        // Mint a new Coin with the specified amount
        let coin = coin::mint<THB>(treasury_cap, amount, ctx);
        
        // Transfer the Coin to the recipient
        // The runtime will merge Coins of the same type automatically
        transfer::public_transfer(coin, recipient);
    }


    /// Transfer a specific `amount` of THB from a mutable Coin held by the caller
    /// Usage: provide the caller's coin, the amount to send, and the recipient
    /// A 0.1% fee is deducted and sent to the DAO wallet
    public entry fun transfer_amount(
        c: &mut coin::Coin<THB>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        // 1. Check if sender is the same as recipient, if so, do nothing
        let sender = kanari_system::tx_context::sender(ctx);

       // 2. sender is not the same as recipient, proceed with transfer
        if (sender == recipient) {
            return
        };

        // 3. Calculate the 0.1% fee
        let fee = calculate_fee(amount);
        
        // 4. Ensure there's enough balance for amount + fee
        let total_required = amount + fee;
        assert!(coin::value(c) >= total_required, 0);

        // 5. Split the total amount (including fee) from sender's coin
        let split_coin = coin::split(c, total_required, ctx);
        
        // 6. From the split coin, separate the fee portion
        let fee_coin = coin::split(&mut split_coin, fee, ctx);
        
        // 7. Transfer the fee to DAO wallet
        transfer::public_transfer(fee_coin, DAO_ADDRESS);
        
        // 8. Transfer the remaining amount to the recipient
        transfer::public_transfer(split_coin, recipient);
    }

    /// Calculate 0.1% fee from the given amount
    /// Formula: fee = (amount * 1) / 1000
    fun calculate_fee(amount: u64): u64 {
        if (amount == 0) {
            return 0
        };
        
        // Calculate fee: amount * 1 / 1000
        let fee = (amount * FEE_RATE_NUMERATOR) / FEE_RATE_DENOMINATOR;
        
        // Ensure minimum fee of 1 if amount > 0
        if (fee == 0 && amount > 0) {
            return 1
        };
        
        fee
    }


    /// Burn a specific `amount` of THB from a mutable Coin held by the caller
    /// Usage: provide the TreasuryCap, a mutable coin owned by caller, amount to burn, and tx context
    public entry fun burn_amount(
    treasury_cap: &mut TreasuryCap<THB>,
    c: &mut Coin<THB>,
        amount: u64,
        ctx: &mut TxContext
    ) {
        let to_burn = coin::split(c, amount, ctx);
        let _burned = coin::burn(treasury_cap, to_burn);
    }

    // ==========================================
    // 🟢 Entry wrappers for CLI calling
    // ==========================================

    /// Usage: kanari move call --function update_icon --args <TreasuryCap_ID> <Metadata_ID> "https://..."
    public entry fun update_icon(
        treasury_cap: &TreasuryCap<THB>,
        metadata: &mut coin::CoinMetadata<THB>,
        new_url: vector<u8>,
    ) {
        let new_url_obj = url::new_unsafe_from_bytes(new_url);
        coin::update_icon_url<THB>(treasury_cap, metadata, option::some(new_url_obj));
    }

    /// Usage: kanari move call --function update_name --args <TreasuryCap_ID> <Metadata_ID> "Thai Baht"
    public entry fun update_name(
        treasury_cap: &TreasuryCap<THB>,
        metadata: &mut coin::CoinMetadata<THB>,
        new_name: vector<u8>,
    ) {
        let name_str = string::utf8(new_name);
        coin::update_name<THB>(treasury_cap, metadata, name_str);
    }

    /// Usage: kanari move call --function update_symbol --args <TreasuryCap_ID> <Metadata_ID> "THB"
    public entry fun update_symbol(
        treasury_cap: &TreasuryCap<THB>,
        metadata: &mut coin::CoinMetadata<THB>,
        new_symbol: vector<u8>,
    ) {
        let symbol_str = ascii::string(new_symbol);
        coin::update_symbol<THB>(treasury_cap, metadata, symbol_str);
    }

    /// Usage: kanari move call --function update_description --args <TreasuryCap_ID> <Metadata_ID> "My new THB description"
    public entry fun update_description(
        treasury_cap: &TreasuryCap<THB>,
        metadata: &mut coin::CoinMetadata<THB>,
        new_description: vector<u8>,
    ) {
        let desc_str = string::utf8(new_description);
        coin::update_description<THB>(treasury_cap, metadata, desc_str);
    }
}
