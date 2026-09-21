// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0
module kanari_system::coin {
    use std::option;
    use std::string;
    use std::ascii;
    use std::vector;
    use kanari_system::url;
    use kanari_system::object;
    use kanari_system::balance::Balance;
    use kanari_system::tx_context::TxContext;
    use kanari_system::transfer;

    // --- Error Codes ---
    /// Invalid arguments are passed to a function.
    const EInvalidArg: u64 = 1;
    /// Trying to split a coin more times than its balance allows.
    const ENotEnough: u64 = 2;
    /// Amount must be greater than zero.
    const EZERO_AMOUNT: u64 = 3;
    /// Arithmetic overflow.
    const EOVERFLOW: u64 = 4;
    /// Arithmetic underflow.
    const EUNDERFLOW: u64 = 5;
    /// Invalid decimal count.
    const EINVALID_DECIMALS: u64 = 6;
    /// Split fan-out exceeds the per-call object cap.
    const ETOO_MANY_PARTS: u64 = 7;

    /// Maximum coins one `divide_into_n` call may create. Bounds object
    /// creation per call even when transaction gas accounting is permissive;
    /// large airdrops should batch across transactions.
    const MAX_SPLIT_PARTS: u64 = 1024;

    /// The per-call split fan-out cap (see `MAX_SPLIT_PARTS`).
    public fun max_split_parts(): u64 {
        MAX_SPLIT_PARTS
    }

    // --- Data Structures ---

    /// Coin resource wrapper with balance (Removed `drop` for asset safety)
    struct Coin<phantom T> has key, store {
        id: object::UID,
        balance: Balance<T>
    }

    /// Capability allowing the bearer to mint and burn coins (Removed `drop`)
    struct TreasuryCap<phantom T> has key, store {
        id: object::UID,
        total_supply: u64 // Tracking total supply directly in the cap
    }

    /// Metadata resource for a currency (Removed `drop`)
    struct CoinMetadata<phantom T> has key, store {
        id: object::UID,
        decimals: u8,
        name: string::String,
        symbol: ascii::String,
        description: string::String,
        icon_url: option::Option<url::Url>
    }

    // --- Public Functions ---

    /// Create a new currency with TreasuryCap for minting control and return the
    /// TreasuryCap and the Metadata object.
    public fun create_currency<T: drop>(
        witness: T,
        decimals: u8,
        symbol_bytes: vector<u8>,
        name_bytes: vector<u8>,
        description_bytes: vector<u8>,
        icon_url: option::Option<url::Url>,
        ctx: &mut TxContext
    ): (TreasuryCap<T>, CoinMetadata<T>) {
        let _ = witness;

        assert!(decimals <= 9, EINVALID_DECIMALS);

        let symbol = ascii::string(symbol_bytes);
        let name = string::utf8(name_bytes);
        let description = string::utf8(description_bytes);

        let treasury_cap = TreasuryCap<T> { id: object::new(ctx), total_supply: 0 };
        let metadata = CoinMetadata<T> {
            id: object::new(ctx),
            decimals,
            name,
            symbol,
            description,
            icon_url
        };

        (treasury_cap, metadata)
    }

    /// Create a regulated currency (compatibility with kanari)
    public fun create_regulated_currency<T: drop>(
        witness: T,
        decimals: u8,
        symbol_bytes: vector<u8>,
        name_bytes: vector<u8>,
        description_bytes: vector<u8>,
        icon_url: option::Option<url::Url>,
        ctx: &mut TxContext
    ): (
        TreasuryCap<T>, kanari_system::deny_list::DenyCap<T>, CoinMetadata<T>
    ) {
        let _ = witness;
        assert!(decimals <= 9, EINVALID_DECIMALS);

        let symbol = ascii::string(symbol_bytes);
        let name = string::utf8(name_bytes);
        let description = string::utf8(description_bytes);

        let treasury_cap = TreasuryCap<T> { id: object::new(ctx), total_supply: 0 };
        let denycap = kanari_system::deny_list::new_denycap<T>(ctx);
        let metadata = CoinMetadata<T> {
            id: object::new(ctx),
            decimals,
            name,
            symbol,
            description,
            icon_url
        };
        (treasury_cap, denycap, metadata)
    }

    /// Mint new coins using TreasuryCap
    public fun mint<T>(
        cap: &mut TreasuryCap<T>, amount: u64, ctx: &mut TxContext
    ): Coin<T> {
        assert!(amount > 0, EZERO_AMOUNT);
        let new_total = cap.total_supply + amount;
        assert!(new_total >= cap.total_supply, EOVERFLOW);
        cap.total_supply = new_total;
        object::save_object(cap);
        Coin {
            id: object::new(ctx),
            balance: kanari_system::balance::create<T>(amount)
        }
    }

    /// Mint and transfer to recipient
    public fun mint_and_transfer<T>(
        cap: &mut TreasuryCap<T>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        let coin = mint(cap, amount, ctx);
        transfer::public_transfer(coin, recipient);
    }

    /// Burn coins, decreasing total supply
    public fun burn<T>(cap: &mut TreasuryCap<T>, coin: Coin<T>): u64 {
        let Coin { id, balance } = coin;
        let value = kanari_system::balance::destroy<T>(balance);
        assert!(value > 0, EZERO_AMOUNT);
        assert!(cap.total_supply >= value, EUNDERFLOW);
        cap.total_supply = cap.total_supply - value;
        object::save_object(cap);
        object::delete(id);
        value
    }

    /// Convert a `Coin<T>` into its inner `Balance<T>`.
    public fun into_balance<T>(coin: Coin<T>): Balance<T> {
        let Coin { id, balance } = coin;
        object::delete(id);
        balance
    }

    /// Construct a `Coin<T>` from a `Balance<T>`.
    public fun from_balance<T>(balance: Balance<T>, ctx: &mut TxContext): Coin<T> {
        Coin { id: object::new(ctx), balance }
    }

    /// Get total supply from TreasuryCap
    public fun total_supply<T>(cap: &TreasuryCap<T>): u64 {
        cap.total_supply
    }

    /// Get coin value
    public fun value<T>(coin: &Coin<T>): u64 {
        kanari_system::balance::value(&coin.balance)
    }

    /// Create a zero-value Coin<T> (useful for testing and as a starting point)
    public fun zero<T>(ctx: &mut TxContext): Coin<T> {
        Coin {
            id: object::new(ctx),
            balance: kanari_system::balance::create<T>(0)
        }
    }

    /// Split a coin into two. Returns the new coin with the specified amount.
    public fun split<T>(coin: &mut Coin<T>, amount: u64, ctx: &mut TxContext): Coin<T> {
        assert!(amount > 0, EZERO_AMOUNT);
        assert!(value(coin) >= amount, ENotEnough);

        let new_balance = kanari_system::balance::split(&mut coin.balance, amount);
        object::save_object(coin);
        Coin { id: object::new(ctx), balance: new_balance }
    }

    /// Join two coins together (adds the balance of 'other' into 'coin').
    public fun join<T>(coin: &mut Coin<T>, other: Coin<T>) {
        let Coin { id, balance } = other;
        kanari_system::balance::merge(&mut coin.balance, balance);
        object::save_object(coin);
        object::delete(id);
    }

    /// Entry wrapper for joining two coin objects owned by the sender.
    public entry fun join_entry<T>(coin: &mut Coin<T>, other: Coin<T>) {
        join(coin, other);
    }

    /// Destroy a zero-balance coin.
    public fun destroy_zero<T>(coin: Coin<T>) {
        let Coin { id, balance } = coin;
        assert!(kanari_system::balance::value(&balance) == 0, EInvalidArg);
        kanari_system::balance::destroy<T>(balance);
        object::delete(id);
    }

    /// Split coin `self` into `n - 1` coins with equal balances.
    public fun divide_into_n<T>(
        self: &mut Coin<T>, n: u64, ctx: &mut TxContext
    ): vector<Coin<T>> {
        assert!(n > 0, EInvalidArg);
        assert!(n <= MAX_SPLIT_PARTS, ETOO_MANY_PARTS);
        assert!(value(self) >= n, ENotEnough);

        let vec = vector::empty<Coin<T>>();
        let i = 0;
        let split_amount = value(self) / n;
        while (i < n - 1) {
            vector::push_back(&mut vec, split(self, split_amount, ctx));
            i = i + 1;
        };
        vec
    }

    // ==========================================
    // Functions to update CoinMetadata
    // ==========================================
    public fun update_icon_url<T>(
        treasury: &TreasuryCap<T>,
        metadata: &mut CoinMetadata<T>,
        url: option::Option<url::Url>
    ) {
        let _ = treasury;
        metadata.icon_url = url;
    }

    public fun update_name<T>(
        treasury: &TreasuryCap<T>, metadata: &mut CoinMetadata<T>, name: string::String
    ) {
        let _ = treasury;
        metadata.name = name;
    }

    public fun update_symbol<T>(
        treasury: &TreasuryCap<T>, metadata: &mut CoinMetadata<T>, symbol: ascii::String
    ) {
        let _ = treasury;
        metadata.symbol = symbol;
    }

    public fun update_description<T>(
        treasury: &TreasuryCap<T>, metadata: &mut CoinMetadata<T>, description: string::String
    ) {
        let _ = treasury;
        metadata.description = description;
    }
}

