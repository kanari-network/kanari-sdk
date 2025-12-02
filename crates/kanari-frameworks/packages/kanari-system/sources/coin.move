module kanari_system::coin {
    use std::option;
    use std::string;
    use std::ascii;
    use kanari_system::url;
    use kanari_system::object;
    use kanari_system::balance::{Self, Balance, Supply};
    use kanari_system::tx_context::TxContext;
    use kanari_system::transfer;
    
    // --- Data Structures ---

    /// Coin resource wrapper with balance
    struct Coin<phantom T> has store, drop {
        balance: Balance<T>,
    }

    /// Capability allowing the bearer to mint and burn coins
    struct TreasuryCap<phantom T> has store, drop {
        id: object::UID,
        total_supply: u64, // Tracking total supply directly in the cap
    }

    /// Treasury: holds authority to mint into a Supply (deprecated, use TreasuryCap)
    struct Treasury<phantom T> has store, drop {
    }

    /// Metadata resource for a currency (stored as an object with UID)
    struct CoinMetadata<phantom T> has key, store, drop {
        id: object::UID,
        decimals: u8,
        name: string::String,
        symbol: ascii::String,
        description: string::String,
        icon_url: option::Option<url::Url>,
    }

    // --- Error Codes ---
    const EZERO_AMOUNT: u64 = 1;
    const EOVERFLOW: u64 = 2;
    const EUNDERFLOW: u64 = 3;
    const EINVALID_DECIMALS: u64 = 5;

    // --- Public Functions ---

    /// Create a new currency with TreasuryCap for minting control and return the
    /// TreasuryCap and the Metadata object. Callers may transfer/freeze the
    /// returned objects as appropriate for their use-case.
    public fun create_currency<T: drop>(
        witness: T,
        decimals: u8,
        symbol: ascii::String,
        name: string::String,
        description: string::String,
        icon_url: option::Option<url::Url>,
        ctx: &mut TxContext,
    ): (TreasuryCap<T>, CoinMetadata<T>) {
        // 1. Consume the witness type
        let _ = witness;
        
        // Basic safety checks for decimals (Move's u8 can hold max 255, but typically < 27 for real-world)
        // We'll set a soft limit based on common standards.
        assert!(decimals <= 27, EINVALID_DECIMALS); 
        
        // 2. Create the Capability and Metadata, explicitly specifying the generic type T
        let treasury_cap = TreasuryCap<T> { id: object::new(ctx), total_supply: 0 };
        let metadata = CoinMetadata<T> { 
            id: object::new(ctx), 
            decimals, 
            name, 
            symbol, 
            description, 
            icon_url 
        };

        // Return the newly-created capability and metadata. Callers decide how
        // to distribute or freeze them (e.g., transfer the cap to an address
        // or freeze the metadata for public visibility).
        (treasury_cap, metadata)
    }

    /// Mint new coins using TreasuryCap
    /// Returns the newly minted Coin<T>.
    public fun mint<T>(
        cap: &mut TreasuryCap<T>,
        amount: u64,
        _ctx: &mut TxContext,
    ): Coin<T> {
        assert!(amount > 0, EZERO_AMOUNT);
        let new_total = cap.total_supply + amount;
        assert!(new_total >= cap.total_supply, EOVERFLOW); // Check for overflow
        
        cap.total_supply = new_total;
        
        Coin {
            balance: balance::create(amount),
        }
    }

    /// Mint and transfer to recipient
    public fun mint_and_transfer<T>(
        cap: &mut TreasuryCap<T>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext,
    ) {
        let coin = mint(cap, amount, ctx);
        transfer::public_transfer(coin, recipient);
    }

    /// Burn coins, decreasing total supply
    public fun burn<T>(cap: &mut TreasuryCap<T>, coin: Coin<T>): u64 {
        let Coin { balance } = coin;
        let value = balance::destroy(balance);
        
        assert!(cap.total_supply >= value, EUNDERFLOW); // Check for underflow
        
        cap.total_supply = cap.total_supply - value;
        value
    }

    /// Convert a `Coin<T>` into its inner `Balance<T>`.
    public fun into_balance<T>(coin: Coin<T>): Balance<T> {
        let Coin { balance } = coin;
        balance
    }

    /// Construct a `Coin<T>` from a `Balance<T>`.
    /// This helper allows other modules to wrap balances into Coin objects
    /// when they take custody of raw balances (e.g., DEX pools).
    public fun from_balance<T>(balance: Balance<T>): Coin<T> {
        Coin { balance }
    }

    /// Get total supply from TreasuryCap
    public fun total_supply<T>(cap: &TreasuryCap<T>): u64 {
        cap.total_supply
    }

    /// Get coin value
    public fun value<T>(coin: &Coin<T>): u64 {
        balance::value(&coin.balance)
    }

    /// Split a coin into two. Returns the new coin with the specified amount.
    public fun split<T>(coin: &mut Coin<T>, amount: u64, ctx: &mut TxContext): Coin<T> {
        // Assert for sufficient balance is implicitly handled by balance::split
        let _ = ctx;
        Coin {
            balance: balance::split(&mut coin.balance, amount),
        }
    }

    /// Join two coins together (adds the balance of 'other' into 'coin').
    public fun join<T>(coin: &mut Coin<T>, other: Coin<T>) {
        let Coin { balance } = other;
        balance::merge(&mut coin.balance, balance);
    }
    
    // --- Deprecated/Legacy functions ---

    /// Deprecated: Convert a treasury (or treasury cap) into a supply handle.
    /// In modern Kanari/Move systems, the total supply is tracked either in the TreasuryCap
    /// or in a separate Supply object that is shared upon creation.
    public fun treasury_into_supply<T>(cap: &mut TreasuryCap<T>): Supply<T> {
        let _ = cap;
        balance::new_supply<T>() // Assumes new_supply is still needed for compatibility
    }

}