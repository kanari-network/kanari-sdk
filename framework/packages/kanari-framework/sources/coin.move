module kanari_framework::coin {
    use std::string;
    use std::ascii;
    use std::option::{Self, Option};
    use kanari_framework::balance::{Self, Balance, Supply};
    use kanari_framework::tx_context::TxContext;
    use kanari_framework::object::{Self, UID, ID};
    use kanari_framework::transfer;
    use kanari_framework::url::{Self, Url};
    use std::vector;
    use kanari_framework::deny_list::{Self, DenyList};
    use std::type_name;


    /// A type passed to create_supply is not a one-time witness.
    const EBadWitness: u64 = 0;
    /// Invalid arguments are passed to a function.
    const EInvalidArg: u64 = 1;
    /// Trying to split a coin more times than its balance allows.
    const ENotEnough: u64 = 2;

    /// A coin of type `T` worth `value`. Transferable and storable
    struct Coin<phantom T> has key, store {
        id: UID,
        balance: Balance<T>
    }

    /// Each Coin type T created through `create_currency` function will have a
    /// unique instance of CoinMetadata<T> that stores the metadata for this coin type.
    struct CoinMetadata<phantom T> has key, store {
        id: UID,
        /// Number of decimal places the coin uses.
        /// A coin with `value ` N and `decimals` D should be shown as N / 10^D
        /// E.g., a coin with `value` 7002 and decimals 3 should be displayed as 7.002
        /// This is metadata for display usage only.
        decimals: u8,
        /// Name for the token
        name: string::String,
        /// Symbol for the token
        symbol: ascii::String,
        /// Description of the token
        description: string::String,
        /// URL for the token logo
        icon_url: Option<Url>
    }

    /// Similar to CoinMetadata, but created only for regulated coins that use the DenyList.
    /// This object is always immutable.
    struct RegulatedCoinMetadata<phantom T> has key {
        id: UID,
        /// The ID of the coin's CoinMetadata object.
        coin_metadata_object: ID,
        /// The ID of the coin's DenyCap object.
        deny_cap_object: ID,
    }

    /// Capability allowing the bearer to mint and burn
    /// coins of type `T`. Transferable
    struct TreasuryCap<phantom T> has key, store {
        id: UID,
        total_supply: Supply<T>
    }

    /// Capability allowing the bearer to freeze addresses, preventing those addresses from
    /// interacting with the coin as an input to a transaction.
    struct DenyCap<phantom T> has key, store {
        id: UID,
    }

    // === Supply <-> TreasuryCap morphing and accessors  ===

    /// Return the total number of `T`'s in circulation.
    public fun total_supply<T>(cap: &TreasuryCap<T>): u64 {
        balance::supply_value(&cap.total_supply)
    }

    /// Unwrap `TreasuryCap` getting the `Supply`.
    ///
    /// Operation is irreversible. Supply cannot be converted into a `TreasuryCap` due
    /// to different security guarantees (TreasuryCap can be created only once for a type)
    public fun treasury_into_supply<T>(treasury: TreasuryCap<T>): Supply<T> {
        let TreasuryCap { id, total_supply } = treasury;
        object::delete(id);
        total_supply
    }

    /// Get immutable reference to the treasury's `Supply`.
    public fun supply_immut<T>(treasury: &TreasuryCap<T>): &Supply<T> {
        &treasury.total_supply
    }

    /// Get mutable reference to the treasury's `Supply`.
    public fun supply_mut<T>(treasury: &mut TreasuryCap<T>): &mut Supply<T> {
        &mut treasury.total_supply
    }

    // === Balance <-> Coin accessors and type morphing ===

    /// Public getter for the coin's value
    public fun value<T>(self: &Coin<T>): u64 {
        balance::value(&self.balance)
    }

    /// Get immutable reference to the balance of a coin.
    public fun balance<T>(coin: &Coin<T>): &Balance<T> {
        &coin.balance
    }

    /// Get a mutable reference to the balance of a coin.
    public fun balance_mut<T>(coin: &mut Coin<T>): &mut Balance<T> {
        &mut coin.balance
    }

    /// Wrap a balance into a Coin to make it transferable.
    public fun from_balance<T>(balance: Balance<T>, ctx: &mut TxContext): Coin<T> {
        Coin { id: object::new(ctx), balance }
    }

    /// Destruct a Coin wrapper and keep the balance.
    public fun into_balance<T>(coin: Coin<T>): Balance<T> {
        let Coin { id, balance } = coin;
        object::delete(id);
        balance
    }

    /// Take a `Coin` worth of `value` from `Balance`.
    /// Aborts if `value > balance.value`
    public fun take<T>(
        balance: &mut Balance<T>, value: u64, ctx: &mut TxContext,
    ): Coin<T> {
        Coin {
            id: object::new(ctx),
            balance: balance::split(balance, value)
        }
    }

    /// Put a `Coin<T>` to the `Balance<T>`.
    public fun put<T>(balance: &mut Balance<T>, coin: Coin<T>) {
        balance::join(balance, into_balance(coin));
    }

    // === Base Coin functionality ===

    /// Consume the coin `c` and add its value to `self`.
    /// Aborts if `c.value + self.value > U64_MAX`
    public entry fun join<T>(self: &mut Coin<T>, c: Coin<T>) {
        let Coin { id, balance } = c;
        object::delete(id);
        balance::join(&mut self.balance, balance);
    }

    /// Split coin `self` to two coins, one with balance `split_amount`,
    /// and the remaining balance is left is `self`.
    public fun split<T>(
        self: &mut Coin<T>, split_amount: u64, ctx: &mut TxContext
    ): Coin<T> {
        take(&mut self.balance, split_amount, ctx)
    }

    /// Split coin `self` into `n - 1` coins with equal balances. The remainder is left in
    /// `self`. Return newly created coins.
    public fun divide_into_n<T>(
        self: &mut Coin<T>, n: u64, ctx: &mut TxContext
    ): vector<Coin<T>> {
        assert!(n > 0, EInvalidArg);
        assert!(n <= value(self), ENotEnough);

        let vec = vector::empty<Coin<T>>();
        let i = 0;
        let split_amount = value(self) / n;
        while (i < n - 1) {
            vector::push_back(&mut vec, split(self, split_amount, ctx));
            i = i + 1;
        };
        vec
    }

    /// Make any Coin with a zero value. Useful for placeholding
    /// bids/payments or preemptively making empty balances.
    public fun zero<T>(ctx: &mut TxContext): Coin<T> {
        Coin { id: object::new(ctx), balance: balance::zero() }
    }

    /// Destroy a coin with value zero
    public fun destroy_zero<T>(c: Coin<T>) {
        let Coin { id, balance } = c;
        object::delete(id);
        balance::destroy_zero(balance)
    }

    // === Registering new coin types and managing the coin supply ===

    /// Create a new currency type `T` as and return the `TreasuryCap` for
    /// `T` to the caller. Can only be called with a `one-time-witness`
    /// type, ensuring that there's only one `TreasuryCap` per `T`.
    public fun create_currency<T: drop>(
        witness: T,
        decimals: u8,
        symbol: vector<u8>,
        name: vector<u8>,
        description: vector<u8>,
        icon_url: Option<Url>,
        ctx: &mut TxContext
    ): (TreasuryCap<T>, CoinMetadata<T>) {
        // Make sure there's only one instance of the type T
        assert!(kanari_framework::types::is_one_time_witness(&witness), EBadWitness);

        (
            TreasuryCap {
                id: object::new(ctx),
                total_supply: balance::create_supply(witness)
            },
            CoinMetadata {
                id: object::new(ctx),
                decimals,
                name: string::utf8(name),
                symbol: ascii::string(symbol),
                description: string::utf8(description),
                icon_url
            }
        )
    }

    /// This creates a new currency, via `create_currency`, but with an extra capability that
    /// allows for specific addresses to have their coins frozen. Those addresses cannot interact
    /// with the coin as input objects.
    public fun create_regulated_currency<T: drop>(
        witness: T,
        decimals: u8,
        symbol: vector<u8>,
        name: vector<u8>,
        description: vector<u8>,
        icon_url: Option<Url>,
        ctx: &mut TxContext
    ): (TreasuryCap<T>, DenyCap<T>, CoinMetadata<T>) {
        let (treasury_cap, metadata) = create_currency(
            witness,
            decimals,
            symbol,
            name,
            description,
            icon_url,
            ctx
        );
        let deny_cap = DenyCap {
            id: object::new(ctx),
        };
        transfer::freeze_object(RegulatedCoinMetadata<T> {
            id: object::new(ctx),
            coin_metadata_object: object::id(&metadata),
            deny_cap_object: object::id(&deny_cap),
        });
        (treasury_cap, deny_cap, metadata)
    }

    /// Create a coin worth `value` and increase the total supply
    /// in `cap` accordingly.
    public fun mint<T>(
        cap: &mut TreasuryCap<T>, value: u64, ctx: &mut TxContext,
    ): Coin<T> {
        Coin {
            id: object::new(ctx),
            balance: balance::increase_supply(&mut cap.total_supply, value)
        }
    }

    /// Mint some amount of T as a `Balance` and increase the total
    /// supply in `cap` accordingly.
    /// Aborts if `value` + `cap.total_supply` >= U64_MAX
    public fun mint_balance<T>(
        cap: &mut TreasuryCap<T>, value: u64
    ): Balance<T> {
        balance::increase_supply(&mut cap.total_supply, value)
    }

    /// Destroy the coin `c` and decrease the total supply in `cap`
    /// accordingly.
    public entry fun burn<T>(cap: &mut TreasuryCap<T>, c: Coin<T>): u64 {
        let Coin { id, balance } = c;
        object::delete(id);
        balance::decrease_supply(&mut cap.total_supply, balance)
    }

    /// The index into the deny list vector for the `kanari_framework::coin::Coin` type.
    const DENY_LIST_COIN_INDEX: u64 = 0; // TODO public(package) const

    /// Adds the given address to the deny list, preventing it
    /// from interacting with the specified coin type as an input to a transaction.
    public fun deny_list_add<T>(
       deny_list: &mut DenyList,
       _deny_cap: &mut DenyCap<T>,
       addr: address,
       _ctx: &mut TxContext
    ) {
        let type =
            ascii::into_bytes(type_name::into_string(type_name::get_with_original_ids<T>()));
        deny_list::add(
            deny_list,
            DENY_LIST_COIN_INDEX,
            type,
            addr,
        )
    }

    /// Removes an address from the deny list.
    /// Aborts with `ENotFrozen` if the address is not already in the list.
    public fun deny_list_remove<T>(
       deny_list: &mut DenyList,
       _deny_cap: &mut DenyCap<T>,
       addr: address,
       _ctx: &mut TxContext
    ) {
        let type =
            ascii::into_bytes(type_name::into_string(type_name::get_with_original_ids<T>()));
        deny_list::remove(
            deny_list,
            DENY_LIST_COIN_INDEX,
            type,
            addr,
        )
    }

    /// Returns true iff the given address is denied for the given coin type. It will
    /// return false if given a non-coin type.
    public fun deny_list_contains<T>(
        freezer: &DenyList,
        addr: address,
    ): bool {
        let type_str = type_name::into_string(type_name::get_with_original_ids<T>());
        
        // Check if type is a primitive type - in which case it can't be a coin
        if (type_str == ascii::string(b"bool") ||
            type_str == ascii::string(b"u8") ||
            type_str == ascii::string(b"u64") || 
            type_str == ascii::string(b"u128") ||
            type_str == ascii::string(b"address") ||
            type_str == ascii::string(b"signer")) {
            return false
        };

        let type = ascii::into_bytes(type_str);
        deny_list::contains(
            freezer,
            DENY_LIST_COIN_INDEX,
            type,
            addr,
        )
    }

    // === Entrypoints ===

    /// Mint `amount` of `Coin` and send it to `recipient`. Invokes `mint()`.
    public entry fun mint_and_transfer<T>(
        c: &mut TreasuryCap<T>, amount: u64, recipient: address, ctx: &mut TxContext
    ) {
        transfer::public_transfer(mint(c, amount, ctx), recipient)
    }

    // === Update coin metadata ===

    /// Update name of the coin in `CoinMetadata`
    public entry fun update_name<T>(
        _treasury: &TreasuryCap<T>, metadata: &mut CoinMetadata<T>, name: string::String
    ) {
        metadata.name = name;
    }

    /// Update the symbol of the coin in `CoinMetadata`
    public entry fun update_symbol<T>(
        _treasury: &TreasuryCap<T>, metadata: &mut CoinMetadata<T>, symbol: ascii::String
    ) {
        metadata.symbol = symbol;
    }

    /// Update the description of the coin in `CoinMetadata`
    public entry fun update_description<T>(
        _treasury: &TreasuryCap<T>, metadata: &mut CoinMetadata<T>, description: string::String
    ) {
        metadata.description = description;
    }

    /// Update the url of the coin in `CoinMetadata`
    public entry fun update_icon_url<T>(
        _treasury: &TreasuryCap<T>, metadata: &mut CoinMetadata<T>, url: ascii::String
    ) {
        metadata.icon_url = option::some(url::new_unsafe(url));
    }

    // === Get coin metadata fields for on-chain consumption ===

    public fun get_decimals<T>(metadata: &CoinMetadata<T>): u8 {
        metadata.decimals
    }

    public fun get_name<T>(metadata: &CoinMetadata<T>): string::String {
        metadata.name
    }

    public fun get_symbol<T>(metadata: &CoinMetadata<T>): ascii::String {
        metadata.symbol
    }

    public fun get_description<T>(metadata: &CoinMetadata<T>): string::String {
        metadata.description
    }

    public fun get_icon_url<T>(metadata: &CoinMetadata<T>): Option<Url> {
        metadata.icon_url
    }

    // === Test-only code ===

    #[test_only]
    /// Mint coins of any type for (obviously!) testing purposes only
    public fun mint_for_testing<T>(value: u64, ctx: &mut TxContext): Coin<T> {
        Coin { id: object::new(ctx), balance: balance::create_for_testing(value) }
    }

    #[test_only]
    /// Burn coins of any type for testing purposes only
    public fun burn_for_testing<T>(coin: Coin<T>): u64 {
        let Coin { id, balance } = coin;
        object::delete(id);
        balance::destroy_for_testing(balance)
    }

    #[test_only]
    /// Create a `TreasuryCap` for any `Coin` for testing purposes.
    public fun create_treasury_cap_for_testing<T>(
        ctx: &mut TxContext
    ): TreasuryCap<T> {
        TreasuryCap {
            id: object::new(ctx),
            total_supply: balance::create_supply_for_testing()
        }
    }

    // === Deprecated code ===

    // oops, wanted treasury: &TreasuryCap<T>
    public fun supply<T>(treasury: &mut TreasuryCap<T>): &Supply<T> {
        &treasury.total_supply
    }

    // deprecated as we have CoinMetadata now
    #[allow(unused_field)]
    struct CurrencyCreated<phantom T> has copy, drop {
        decimals: u8
    }
}