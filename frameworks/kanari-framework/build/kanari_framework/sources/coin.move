module kanari_framework::coin {
    use std::ascii;
    use std::string;
    use std::vector;
    use std::option::Option;
    use kanari_framework::tx_context::{Self, TxContext};
    use kanari_framework::object::{Self, ID, UID};
    use kanari_framework::balance::{Self, Balance, Supply};
    use kanari_framework::url::Url;
    
    
    /// A type passed to create_supply is not a one-time witness.
    const EBadWitness: u64 = 0;
    /// Invalid arguments are passed to a function.
    const EInvalidArg: u64 = 1;
    /// Trying to split a coin more times than its balance allows.
    const ENotEnough: u64 = 2;
    // #[error]
    // const EGlobalPauseNotAllowed: vector<u8> =
    //    b"Kill switch was not allowed at the creation of the DenyCap";
    const EGlobalPauseNotAllowed: u64 = 3;

    struct Coin<phantom T> has store {
        id: UID,
        balance: Balance<T>,
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
        icon_url: Option<Url>,
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
        total_supply: Supply<T>,
    }    

    /// Capability allowing the bearer to deny addresses from using the currency's coins--
    /// immediately preventing those addresses from interacting with the coin as an input to a
    /// transaction and at the start of the next preventing them from receiving the coin.
    /// If `allow_global_pause` is true, the bearer can enable a global pause that behaves as if
    /// all addresses were added to the deny list.
    struct DenyCapV2<phantom T> has key, store {
        id: UID,
        allow_global_pause: bool,
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
        object::delete(id); // Use object::delete instead of id.delete()
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


    /// Get immutable reference to the balance of a coin.
    public fun balance<T>(coin: &Coin<T>): &Balance<T> {
        &coin.balance
    }

    /// Get a mutable reference to the balance of a coin.
    public fun balance_mut<T>(coin: &mut Coin<T>): &mut Balance<T> {
        &mut coin.balance
    }

    /// Wrap a balance into a Coin to make it transferable.

    /// Destruct a Coin wrapper and keep the balance.
    public fun into_balance<T>(coin: Coin<T>): Balance<T> {
        let Coin { id, balance } = coin;
        object::delete(id);
        balance
    }

}