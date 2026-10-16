module james::thb {
    use kanari_system::coin;
    use kanari_system::coin::{Coin, TreasuryCap};
    use kanari_system::deny_list::{DenyCap, DenyList};
    use kanari_system::tx_context::{TxContext};
    use std::option;
    use kanari_system::transfer;
    use kanari_system::url;
    use std::string;
    use std::ascii;

    /// Name of the coin
    struct THB has drop {}

    // ==========================================
    // DAO Configuration
    // ==========================================
    /// DAO wallet address for collecting transfer fees (0.1%)
    const DAO_ADDRESS: address =
        @0x7e669cf309e9761beb2083d12564895ccea1389f2851a479edcc0d1ad19e4350;

    /// Transfer fee rate: 0.1% = 1/1000 (in basis points: 10 out of 10000)
    const FEE_RATE_NUMERATOR: u64 = 1;
    const FEE_RATE_DENOMINATOR: u64 = 1000;

    /// Abort code when the recipient is denied or THB is paused.
    const EDENIED: u64 = 1;

    #[allow(unused_function)]
    /// Initialize and register the THB currency as a *regulated* coin.
    ///
    /// Runs exactly once: the runtime invokes `init` on fresh publish only
    /// (never on upgrade), with `tx_context::sender` set to the publisher,
    /// so TreasuryCap, DenyCap and metadata land with the deployer and no
    /// replayable setup entry needs to exist.
    fun init(witness: THB, ctx: &mut TxContext) {
        let (treasury, deny_cap, metadata) = coin::create_regulated_currency<THB>(
            witness,
            6,
            b"THB",
            b"THB Token",
            b"",
            option::none<kanari_system::url::Url>(),
            ctx,
        );
        let sender = kanari_system::tx_context::sender(ctx);
        transfer::public_transfer(treasury, sender);
        transfer::public_transfer(deny_cap, sender);
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
        ctx: &mut TxContext,
    ) {
        // Mint a new Coin with the specified amount
        let coin = coin::mint<THB>(treasury_cap, amount, ctx);
        // Transfer the Coin to the recipient
        // The runtime will merge Coins of the same type automatically
        transfer::public_transfer(coin, recipient);
    }

    /// Transfer a specific `amount` of THB from a mutable Coin held by the caller
    /// A 0.1% fee is deducted and sent to the DAO wallet.
    /// Regulated path: aborts `EDENIED` when `recipient` is on the THB deny
    /// list or THB is globally paused. Takes the shared `DenyList` singleton
    /// (`@0x403`) as a read-only reference.
    public entry fun transfer_amount(
        c: &mut coin::Coin<THB>,
        amount: u64,
        recipient: address,
        deny_list: &DenyList,
        ctx: &mut TxContext,
    ) {
        // 1. Check if sender is the same as recipient, if so, do nothing
        let sender = kanari_system::tx_context::sender(ctx);
        // 2. sender is not the same as recipient, proceed with transfer
        if (sender == recipient) {
            return
        };
        // 3. Regulated check first: denied recipients and paused coin abort
        // before any balance is touched.
        assert!(!coin::deny_list_contains<THB>(deny_list, recipient), EDENIED);
        assert!(!coin::deny_list_paused<THB>(deny_list), EDENIED);
        // 4. Calculate the 0.1% fee
        let fee = calculate_fee(amount);
        // 5. Ensure there's enough balance for amount + fee
        let total_required = amount + fee;
        assert!(coin::value(c) >= total_required, 0);
        // 6. Split the total amount (including fee) from sender's coin
        let split_coin = coin::split(c, total_required, ctx);
        // 7. From the split coin, separate the fee portion
        let fee_coin = coin::split(&mut split_coin, fee, ctx);
        // 8. Transfer the fee to DAO wallet
        transfer::public_transfer(fee_coin, DAO_ADDRESS);
        // 9. Transfer the remaining amount to the recipient
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
        ctx: &mut TxContext,
    ) {
        let to_burn = coin::split(c, amount, ctx);
        let _burned = coin::burn(treasury_cap, to_burn);
    }

    // ==========================================
    // Entry wrappers for CLI calling
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

    // ==========================================
    // Deny-list management (regulated coin)
    // ==========================================
    /// Deny-list operations are gated by `&DenyCap<THB>` minted in `init`,
    /// so only the deployer (or whoever holds the cap) can maintain entries.

    /// Add `addr` to the THB deny list.
    public entry fun deny_add(
        deny_list: &mut DenyList,
        cap: &DenyCap<THB>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_add(deny_list, cap, addr, ctx);
    }

    /// Remove `addr` from the THB deny list. Aborts when not denied.
    public entry fun deny_remove(
        deny_list: &mut DenyList,
        cap: &DenyCap<THB>,
        addr: address,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_remove(deny_list, cap, addr, ctx);
    }

    /// Pause all regulated THB flows (fail-closed kill-switch).
    public entry fun deny_pause(
        deny_list: &mut DenyList,
        cap: &DenyCap<THB>,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_pause(deny_list, cap, ctx);
    }

    /// Lift the THB pause.
    public entry fun deny_unpause(
        deny_list: &mut DenyList,
        cap: &DenyCap<THB>,
        ctx: &mut TxContext,
    ) {
        coin::deny_list_unpause(deny_list, cap, ctx);
    }

    /// Open read: true iff `addr` is denied for THB.
    public fun is_denied(deny_list: &DenyList, addr: address): bool {
        coin::deny_list_contains<THB>(deny_list, addr)
    }

    /// Open read: true iff THB is globally paused.
    public fun is_paused(deny_list: &DenyList): bool {
        coin::deny_list_paused<THB>(deny_list)
    }

    #[test_only]
    /// Mint a fresh regulated THB currency for tests (the real `init` runs
    /// only on publish, so tests build the same objects directly).
    public fun create_regulated_for_test(
        ctx: &mut TxContext,
    ): (TreasuryCap<THB>, DenyCap<THB>, coin::CoinMetadata<THB>) {
        coin::create_regulated_currency(
            THB {},
            6,
            b"THB",
            b"THB Token",
            b"",
            option::none(),
            ctx,
        )
    }
}
