// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0
module kanari_system::coin {
    use std::option;
    use std::vector;
    use std::string;
    use std::ascii;
    use std::type_name;
    use kanari_system::url;
    use kanari_system::object;
    use kanari_system::balance::Balance;
    use kanari_system::deny_list::{Self, DenyList, DenyCap};
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
        balance: Balance<T>,
    }

    /// Capability allowing the bearer to mint and burn coins (Removed `drop`)
    struct TreasuryCap<phantom T> has key, store {
        id: object::UID,
        total_supply: u64,
        // Tracking total supply directly in the cap,
    }

    /// Metadata resource for a currency (Removed `drop`)
    struct CoinMetadata<phantom T> has key, store {
        id: object::UID,
        decimals: u8,
        name: string::String,
        symbol: ascii::String,
        description: string::String,
        icon_url: option::Option<url::Url>,
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
        ctx: &mut TxContext,
    ): (TreasuryCap<T>, CoinMetadata<T>) {
        // One canonical currency per coin type: the host rejects a second
        // claim for `T`, so mint authority cannot fragment across caps.
        kanari_system::types::claim_witness(&witness);
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

    /// Create a regulated currency, returning the mint `TreasuryCap`, a
    /// `DenyCap` for managing a per-currency deny list, and the metadata.
    ///
    /// Enforcement model: this creates the *capability* to maintain deny
    /// entries in the shared `deny_list::DenyList` singleton; it does not by
    /// itself block transfers. Entries are managed with the returned
    /// `DenyCap<T>` via `deny_list_add`/`deny_list_remove` and enforced by
    /// the `*_checked` transfer paths (`pay::split_and_transfer_checked`,
    /// `pay::join_vec_and_transfer_checked`,
    /// `multisig::execute_transfer_checked`), which consult the shared list
    /// through `deny_list_contains`. The plain `coin::*` and
    /// `pay::split_and_transfer`/`join_vec_and_transfer` paths stay
    /// permissionless and skip the check, so a regulated coin must be moved
    /// through the checked paths to be enforceable.
    public fun create_regulated_currency<T: drop>(
        witness: T,
        decimals: u8,
        symbol_bytes: vector<u8>,
        name_bytes: vector<u8>,
        description_bytes: vector<u8>,
        icon_url: option::Option<url::Url>,
        ctx: &mut TxContext,
    ): (
        TreasuryCap<T>, kanari_system::deny_list::DenyCap<T>, CoinMetadata<T>
    ) {
        // One canonical currency per coin type (see `create_currency`).
        kanari_system::types::claim_witness(&witness);
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

    /// Canonical deny-list key for `T`: its `type_name` bytes
    /// (e.g. `0x2::my_coin::MY_COIN`, never the `Coin<...>` wrapper).
    fun deny_type_key<T>(): vector<u8> {
        let name = type_name::get<T>();
        *ascii::as_bytes(type_name::borrow_string(&name))
    }

    #[test_only]
    /// Exposes the deny key so tests pin the exact byte format the
    /// runtime's transfer-time enforcement must reproduce.
    public fun type_key_for_testing<T>(): vector<u8> {
        deny_type_key<T>()
    }

    /// Add `addr` to `T`'s deny entries in the shared deny list. Requires
    /// `&DenyCap<T>` minted by `create_regulated_currency`, so a capability
    /// for one coin can never touch another coin's entries.
    public fun deny_list_add<T>(
        deny_list: &mut DenyList,
        _cap: &DenyCap<T>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        deny_list::v1_add(deny_list, deny_list::coin_index(), deny_type_key<T>(), addr, ctx);
    }

    /// Remove `addr` from `T`'s deny entries. Aborts when not denied.
    public fun deny_list_remove<T>(
        deny_list: &mut DenyList,
        _cap: &DenyCap<T>,
        addr: address,
        _ctx: &mut TxContext,
    ) {
        deny_list::v1_remove(deny_list, deny_list::coin_index(), deny_type_key<T>(), addr);
    }

    /// Open membership read used by the `*_checked` transfer paths.
    public fun deny_list_contains<T>(deny_list: &DenyList, addr: address): bool {
        deny_list::v1_contains(deny_list, deny_list::coin_index(), deny_type_key<T>(), addr)
    }

    /// Pause all regulated flows for `T` (fail-closed kill-switch).
    /// Requires `&DenyCap<T>`, like deny entries.
    public fun deny_list_pause<T>(
        deny_list: &mut DenyList,
        _cap: &DenyCap<T>,
        ctx: &mut TxContext,
    ) {
        deny_list::v2_enable_global_pause(
            deny_list,
            deny_list::coin_index(),
            deny_type_key<T>(),
            ctx
        );
    }

    /// Lift the pause for `T`. Requires `&DenyCap<T>`.
    public fun deny_list_unpause<T>(
        deny_list: &mut DenyList,
        _cap: &DenyCap<T>,
        _ctx: &mut TxContext,
    ) {
        deny_list::v2_disable_global_pause(deny_list, deny_list::coin_index(), deny_type_key<T>());
    }

    /// Open read: true iff `T` is currently paused. The `*_checked`
    /// transfer paths reject paused types alongside denied addresses.
    public fun deny_list_paused<T>(deny_list: &DenyList): bool {
        deny_list::v2_is_global_pause_enabled(deny_list, deny_list::coin_index(), deny_type_key<T>())
    }

    /// Mint new coins using TreasuryCap
    public fun mint<T>(
        cap: &mut TreasuryCap<T>,
        amount: u64,
        ctx: &mut TxContext,
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
        ctx: &mut TxContext,
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
        self: &mut Coin<T>,
        n: u64,
        ctx: &mut TxContext,
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
        url: option::Option<url::Url>,
    ) {
        let _ = treasury;
        metadata.icon_url = url;
    }

    public fun update_name<T>(
        treasury: &TreasuryCap<T>,
        metadata: &mut CoinMetadata<T>,
        name: string::String,
    ) {
        let _ = treasury;
        metadata.name = name;
    }

    public fun update_symbol<T>(
        treasury: &TreasuryCap<T>,
        metadata: &mut CoinMetadata<T>,
        symbol: ascii::String,
    ) {
        let _ = treasury;
        metadata.symbol = symbol;
    }

    public fun update_description<T>(
        treasury: &TreasuryCap<T>,
        metadata: &mut CoinMetadata<T>,
        description: string::String,
    ) {
        let _ = treasury;
        metadata.description = description;
    }
}
