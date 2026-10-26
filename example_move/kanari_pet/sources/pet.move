// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// `KanariPet` — a game example implemented with the framework KRC-721 type.
///
/// Deliberately thin. Everything that matters lives in the framework, wired
/// together in `init`:
///
/// | Concern          | Primitive                            |
/// |------------------|--------------------------------------|
/// | token type       | `kanari_system::krc_721::Nft`        |
/// | display metadata | collection-scoped framework display |
/// | trade rules      | `kanari_system::krc_721`             |
/// | royalty          | `kanari_system::royalty_rule`        |
/// | per-token traits | `kanari_system::krc_721` attribute model |
/// | supply           | `kanari_system::collection::NftCap`   |
///
/// Pet progression is stored in issuer-controlled state, separate from
/// holder-editable KRC-721 metadata.
module kanari_pet::pet {
    use std::option::{Self, Option};
    use std::string::{Self, String};
    use std::vector;
    use kanari_system::coin::{Self, Coin};
    use kanari_system::collection::{Self, Collection, NftCap};
    use kanari_system::event;
    use kanari_system::krc_721::{
        Self,
        CollectionDisplay,
        CollectionPolicyCap,
        CollectionPolicyScope,
        Nft,
    };
    use kanari_system::object::{Self, UID};
    use kanari_system::royalty_rule;
    use kanari_system::transfer;
    use kanari_system::transfer_policy::{
        Self,
        TransferPolicy,
    };
    use kanari_system::table::{Self, Table};
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::url;
    use kanari_system::usd::USD;

    // ==================== Progression ====================
    //
    // Progression metadata is rendered as ordinary attributes, but the
    // authoritative level lives in Stats. Framework-level attribute setters
    // remain public, so gameplay must never trust these display values.

    /// Reserved attribute key holding the pet's power rating.
    const ATTR_POWER: vector<u8> = b"power";

    /// Reserved attribute key holding the pet's level.
    const ATTR_LEVEL: vector<u8> = b"level";

    /// Reserved attribute key holding the pet's rarity band name.
    const ATTR_RARITY: vector<u8> = b"rarity";
    const DEFAULT_IMAGE_URL: vector<u8> =
        b"https://avatars.githubusercontent.com/u/127471673?s=200&v=4";

    /// A caller tried to write a reserved key through a pet-module setter.
    const E_RESERVED_ATTRIBUTE: u64 = 0;

    /// The pet is already at `Stats::max_level`.
    const E_MAX_LEVEL: u64 = 1;

    /// The `Coin<USD>` does not cover the quoted upgrade cost.
    const E_INSUFFICIENT_USD: u64 = 2;

    /// The minted level exceeds the configured cap.
    const E_LEVEL_OUT_OF_RANGE: u64 = 3;
    const E_NOT_PET: u64 = 6;

    /// Supply of the collection `init` creates.
    const MAX_SUPPLY: u64 = 10_000;

    /// Upgrade economics for every `Nft`.
    ///
    /// Shared rather than per-pet so the parameters are one object an indexer
    /// can read and the creator can reason about, and so an upgrade reads the
    /// same numbers every time instead of whatever a token happened to carry.
    struct Stats has key, store {
        id: UID,
        /// Where upgrade payments go.
        treasury: address,
        /// USD (6-decimal base units) charged for each level, at a flat rate.
        base_cost_usd: u64,
        /// Power granted per level.
        power_per_level: u64,
        /// Hard ceiling. At this level `upgrade` refuses rather than wrapping.
        max_level: u64,
        /// Authoritative level state. NFT attributes are display metadata only.
        levels: Table<address, u64>,
    }

    /// One-time module witness required by package initialization.
    struct PET has drop {}

    /// Currency used for royalty payments, separate from the NFT itself.
    struct FEE has key, store, drop {}

    /// Emitted when a pet is destroyed, so indexers can retire it without
    /// replaying every transfer.
    struct Burned has copy, drop {
        pet_id: address,
        owner: address,
    }

    /// Emitted when a pet's attributes change. Off-chain state that mirrors
    /// attributes has to hear about this; there is no other signal, because the
    /// object id does not change when a field does.
    struct AttributesChanged has copy, drop {
        pet_id: address,
    }

    /// Emitted when a pet is upgraded. Carries the USD actually charged so an
    /// indexer can reconcile revenue without replaying the price formula.
    struct Upgraded has copy, drop {
        pet_id: address,
        owner: address,
        level_before: u64,
        level_after: u64,
        power_after: u64,
        usd_paid: u64,
    }

    /// Publication-time setup, run once by the runtime.
    ///
    /// Creates collection-bound metadata and a 5% royalty policy.
    public fun init(_otw: PET, _fee_otw: FEE, ctx: &mut TxContext) {
        let sender = tx_context::sender(ctx);
        share_stats(sender, ctx);
        let (collection, mint_cap) = create_pet_collection(ctx);
        create_pet_display(&mint_cap, ctx);
        create_pet_policy(&mint_cap, &collection, sender, ctx);
        transfer::share_object(collection);
        transfer::public_transfer(mint_cap, sender);
    }

    fun share_stats(treasury: address, ctx: &mut TxContext) {
        transfer::share_object(
            Stats {
                id: object::new(ctx),
                treasury,
                base_cost_usd: 1_000_000,
                power_per_level: 10,
                max_level: 1_000,
                levels: table::new<address, u64>(ctx)
            }
        );
    }

    fun create_pet_collection(ctx: &mut TxContext): (Collection, NftCap) {
        collection::create_collection(
            ctx,
            b"Kanari Pet",
            b"An NFT built on the Kanari Sui-parity stack",
            b"https://avatars.githubusercontent.com/u/127471673?s=200&v=4",
            b"https://github.com/KanariNetwork/kanari-sdk",
            MAX_SUPPLY
        )
    }

    fun create_pet_display(cap: &NftCap, ctx: &mut TxContext) {
        let display = krc_721::create_collection_display(
            cap,
            vector[b"name", b"description", b"image_url", b"creator"],
            vector[
                b"Kanari Pet",
                b"An NFT built on the Kanari Sui-parity stack",
                b"https://avatars.githubusercontent.com/u/127471673?s=200&v=4",
                b"Kanari Labs"
            ],
            ctx
        );
        krc_721::share_collection_display(display);
    }

    fun create_pet_policy(
        mint_cap: &NftCap,
        collection: &Collection,
        beneficiary: address,
        ctx: &mut TxContext,
    ) {
        let (policy, scope, cap) =
            krc_721::new_collection_policy(mint_cap, collection, ctx);
        let policy = install_royalty(policy, &cap, beneficiary);
        krc_721::share_collection_policy(policy);
        krc_721::share_collection_policy_scope(scope);
        krc_721::transfer_collection_policy_cap(cap, beneficiary);
    }

    /// Install the 5% royalty, returning the configured policy.
    ///
    /// A helper rather than inline code because this edition has no `let mut`
    /// bindings: taking `policy` by value and returning it is the way to get a
    /// `&mut` borrow.
    fun install_royalty(
        policy: TransferPolicy<Nft>,
        cap: &CollectionPolicyCap,
        beneficiary: address,
    ): TransferPolicy<Nft> {
        royalty_rule::add_for_krc_721_collection(&mut policy, cap, 500, 0, beneficiary);
        policy
    }

    /// Mint a pet with no attributes, straight to `recipient`.
    ///
    /// Supply is enforced by the framework's `NftCap`, whose `issued` counter is
    /// bounded against `max_supply`. A faked `return_from_burn` cannot reopen
    /// supply; that was a real bug fixed in `collection.move`.
    public entry fun mint(
        pet_cap: &mut NftCap,
        collection: &Collection,
        recipient: address,
        stats: &mut Stats,
        ctx: &mut TxContext,
    ) {
        mint_pet(
            pet_cap,
            collection,
            recipient,
            stats,
            string::utf8(b"Kanari Pet"),
            string::utf8(b"An NFT built on the Kanari Sui-parity stack"),
            url::new_unsafe_from_bytes(DEFAULT_IMAGE_URL),
            vector::empty<String>(),
            vector::empty<String>(),
            ctx
        )
    }

    /// Mint a pet with explicit universal NFT metadata and attributes.
    public entry fun mint_with_metadata(
        pet_cap: &mut NftCap,
        collection: &Collection,
        recipient: address,
        stats: &mut Stats,
        name: vector<u8>,
        description: vector<u8>,
        image_url: vector<u8>,
        attribute_keys: vector<vector<u8>>,
        attribute_values: vector<vector<u8>>,
        ctx: &mut TxContext,
    ) {
        mint_pet(
            pet_cap,
            collection,
            recipient,
            stats,
            string::utf8(name),
            string::utf8(description),
            url::new_unsafe_from_bytes(image_url),
            utf8_all(attribute_keys),
            utf8_all(attribute_values),
            ctx
        );
    }

    /// Mint a pet carrying per-token attributes.
    ///
    /// Keys and values arrive as raw byte vectors because that is what a CLI can
    /// actually serialize — a `vector<String>` parameter has no usable encoding
    /// on the wire, so the entry boundary takes bytes and converts here.
    ///
    /// Metadata defaults to the showcase values. Use `mint_with_metadata`
    /// to choose a per-token name, image URL, and description.
    public entry fun mint_with_attributes(
        pet_cap: &mut NftCap,
        collection: &Collection,
        recipient: address,
        stats: &mut Stats,
        attribute_keys: vector<vector<u8>>,
        attribute_values: vector<vector<u8>>,
        ctx: &mut TxContext,
    ) {
        mint_pet(
            pet_cap,
            collection,
            recipient,
            stats,
            string::utf8(b"Kanari Pet"),
            string::utf8(b"An NFT built on the Kanari Sui-parity stack"),
            url::new_unsafe_from_bytes(DEFAULT_IMAGE_URL),
            utf8_all(attribute_keys),
            utf8_all(attribute_values),
            ctx
        );
    }

    fun mint_pet(
        pet_cap: &mut NftCap,
        collection: &Collection,
        recipient: address,
        stats: &mut Stats,
        name: String,
        description: String,
        image_url: kanari_system::url::Url,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
        ctx: &mut TxContext,
    ) {
        let i = 0;
        while (i < vector::length(&attribute_keys)) {
            assert!(!is_reserved(vector::borrow(&attribute_keys, i)), E_RESERVED_ATTRIBUTE);
            i = i + 1;
        };
        let pet =
            krc_721::mint(
                pet_cap,
                collection,
                name,
                option::some(description),
                option::some(image_url),
                attribute_keys,
                attribute_values,
                ctx
            );
        table::add(&mut stats.levels, krc_721::nft_address(&pet), 0);
        krc_721::transfer(pet, recipient);
    }

    /// Destroy a pet and return its supply unit to the cap.
    ///
    /// Takes the `(cap, collection)` pair rather than just the pet, because
    /// `return_from_burn` must be bound to the collection that actually issued
    /// the token: a cap transferred away from its collection would otherwise let
    /// its holder reclaim supply from an unrelated pool.
    ///
    /// Anyone holding the pet can burn it. That is deliberate — a holder must
    /// never be stuck with an asset they cannot dispose of — and it is safe
    /// because burning returns supply rather than creating it. A malicious
    /// holder cannot mint their way out: `issued_counter` is monotonic and stays
    /// bounded by `max_supply`, so the collection can never exceed its cap.
    public entry fun burn(
        pet_cap: &mut NftCap,
        collection: &Collection,
        stats: &mut Stats,
        pet: Nft,
        ctx: &TxContext,
    ) {
        let pet_address = krc_721::nft_address(&pet);
        let owner = tx_context::sender(ctx);
        event::emit(Burned { pet_id: pet_address, owner });
        if (table::contains(&stats.levels, pet_address)) {
            let _ = table::remove(&mut stats.levels, pet_address);
        };
        krc_721::burn(pet_cap, collection, pet);
    }

    /// Transfer the pet to another address.
    ///
    /// Ownership moves as-is: the pet keeps its progression metadata,
    /// number, and artwork. This is the direct-transfer path — it matches
    /// `transfer::public_transfer` semantics, so royalties on the shared
    /// `TransferPolicy` do not apply here. Trading through a marketplace should
    /// use the kiosk/policy flow instead.
    ///
    /// Only the current owner can call this: the pet arrives as a declared
    /// owned object input, and the runtime rejects mutable owned inputs whose
    /// stored owner is not the sender.
    public entry fun transfer(pet: Nft, recipient: address) {
        krc_721::transfer(pet, recipient);
    }

    /// Insert or overwrite one attribute, taking raw bytes for the CLI.
    ///
    /// Anyone holding the pet may do this, but **not** for a reserved key.
    /// `power`, `level` and `rarity` are paid progression state; letting them
    /// through a free setter would let a holder mint themselves a level 50 pet
    /// for nothing, which is the entire economy collapsed into one call. Those
    /// keys move only via `mint_with_level` or `upgrade`.
    public entry fun set_attribute(
        pet: &mut Nft,
        key: vector<u8>,
        value: vector<u8>,
    ) {
        set_string_attribute(pet, string::utf8(key), string::utf8(value));
    }

    /// The `String`-typed body behind `set_attribute`.
    public fun set_string_attribute(
        pet: &mut Nft,
        key: String,
        value: String,
    ) {
        assert!(!is_reserved(&key), E_RESERVED_ATTRIBUTE);
        let pet_address = krc_721::nft_address(pet);
        krc_721::set_attribute(pet, key, value);
        event::emit(AttributesChanged { pet_id: pet_address });
    }

    /// Remove one attribute and return its value. Aborts if absent.
    ///
    /// Reserved keys cannot be removed through this wrapper so wallet metadata
    /// stays consistent. Gameplay progression is stored separately in `Stats`.
    public fun remove_attribute(pet: &mut Nft, key: &String): String {
        assert!(!is_reserved(key), E_RESERVED_ATTRIBUTE);
        let pet_address = krc_721::nft_address(pet);
        let value = krc_721::remove_attribute(pet, key);
        event::emit(AttributesChanged { pet_id: pet_address });
        value
    }

    /// Replace the whole attribute set from parallel vectors.
    ///
    /// Reserved keys are **preserved**, not replaced and not dropped, keeping
    /// display metadata consistent when a caller replaces free-form attributes.
    public entry fun set_attributes(
        pet: &mut Nft,
        attribute_keys: vector<vector<u8>>,
        attribute_values: vector<vector<u8>>,
    ) {
        set_string_attributes(pet, utf8_all(attribute_keys), utf8_all(attribute_values));
    }

    /// The `String`-typed body behind `set_attributes`.
    ///
    /// Reserved keys are **preserved**, not replaced and not dropped, keeping
    /// display metadata consistent when a caller replaces free-form attributes.
    public fun set_string_attributes(
        pet: &mut Nft,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
    ) {
        let i = 0;
        while (i < vector::length(&attribute_keys)) {
            assert!(
                !is_reserved(vector::borrow(&attribute_keys, i)),
                E_RESERVED_ATTRIBUTE
            );
            i = i + 1;
        };
        let pet_address = krc_721::nft_address(pet);
        let (old_keys, old_values) = krc_721::all_attributes(pet);
        let i = 0;
        while (i < vector::length(&old_keys)) {
            let old_key = *vector::borrow(&old_keys, i);
            if (is_reserved(&old_key)) {
                vector::push_back(&mut attribute_keys, old_key);
                vector::push_back(&mut attribute_values, *vector::borrow(&old_values, i));
            };
            i = i + 1;
        };
        krc_721::set_attributes(pet, attribute_keys, attribute_values);
        event::emit(AttributesChanged { pet_id: pet_address });
    }

    /// The value stored under `key`, or `none`.
    public fun attribute(pet: &Nft, key: &String): Option<String> {
        krc_721::attribute(pet, key)
    }

    /// True iff `key` is present.
    public fun has_attribute(pet: &Nft, key: &String): bool {
        krc_721::has_attribute(pet, key)
    }

    /// How many attributes the pet carries.
    public fun attribute_count(pet: &Nft): u64 {
        krc_721::attribute_count(pet)
    }

    /// Attribute keys in insertion order.
    public fun attribute_keys(pet: &Nft): vector<String> {
        krc_721::attribute_keys(pet)
    }

    /// Every `(key, value)` pair in insertion order, index-aligned.
    public fun all_attributes(pet: &Nft): (vector<String>, vector<String>) {
        krc_721::all_attributes(pet)
    }

    public fun name(pet: &Nft): String {
        krc_721::name(pet)
    }

    public fun image_url(pet: &Nft): Option<kanari_system::url::Url> {
        krc_721::image_url(pet)
    }

    public fun description(pet: &Nft): Option<String> {
        krc_721::description(pet)
    }

    public fun number(pet: &Nft): u64 {
        krc_721::number(pet)
    }

    public fun collection_id(pet: &Nft): address {
        krc_721::collection_of(pet)
    }

    // ==================== Progression: reading ====================

    /// The authoritative pet level. Generic KRC-721 attribute setters cannot
    /// change this issuer-controlled value.
    public fun level(stats: &Stats, pet: &Nft): u64 {
        let pet_address = krc_721::nft_address(pet);
        assert!(table::contains(&stats.levels, pet_address), E_NOT_PET);
        *table::borrow(&stats.levels, pet_address)
    }

    /// The authoritative power rating, derived from issuer-controlled level.
    public fun power(stats: &Stats, pet: &Nft): u64 {
        power_for_level(stats, level(stats, pet))
    }

    /// The authoritative rarity band, derived from issuer-controlled level.
    public fun rarity(stats: &Stats, pet: &Nft): String {
        rarity_for_level(level(stats, pet))
    }

    // ==================== Progression: upgrading ====================

    /// The USD cost of taking a pet from `current_level` to the next one.
    ///
    /// Flat: every level costs exactly `base_cost_usd` — $1.00 per level by
    /// default, so a full climb to `max_level` is 1,000 x $1.00, not an
    /// escalating sum. Public and pure so a client can quote the next step
    /// before committing, and so the charged amount and the quoted amount
    /// cannot drift apart — `upgrade` calls this, not a private copy of the
    /// formula.
    public fun upgrade_cost(stats: &Stats, _current_level: u64): u64 {
        stats.base_cost_usd
    }

    /// Raise a pet one level, paid in USD.
    ///
    /// The payment is taken before the pet is touched, so a caller cannot end up
    /// with a free upgrade by tripping an assert afterwards. `power` and
    /// `rarity` are recomputed from the new level rather than accepted from the
    /// caller — a paid upgrade that let its buyer name their own power rating
    /// would not be a progression system at all.
    public entry fun upgrade(
        pet: &mut Nft,
        stats: &mut Stats,
        payment: &mut Coin<USD>,
        ctx: &mut TxContext,
    ) {
        let pet_address = krc_721::nft_address(pet);
        let owner = tx_context::sender(ctx);

        let current = level(stats, pet);
        assert!(current < stats.max_level, E_MAX_LEVEL);

        let cost = upgrade_cost(stats, current);
        assert!(coin::value(payment) >= cost, E_INSUFFICIENT_USD);

        // Pay first. If anything below were to abort, the whole transaction
        // unwinds anyway, so this ordering is about clarity, not safety.
        let paid = coin::split(payment, cost, ctx);
        transfer::public_transfer(paid, stats.treasury);

        let next = current + 1;
        let next_power = power_for_level(stats, next);

        *table::borrow_mut(&mut stats.levels, pet_address) = next;
        set_reserved(pet, ATTR_LEVEL, u64_string(next));
        set_reserved(pet, ATTR_POWER, u64_string(next_power));
        set_reserved(pet, ATTR_RARITY, rarity_for_level(next));

        event::emit(
            Upgraded {
                pet_id: pet_address,
                owner,
                level_before: current,
                level_after: next,
                power_after: next_power,
                usd_paid: cost
            }
        );
    }

    /// Mint a pet at a specific level, with `power` and `rarity` derived from it.
    ///
    /// Level is free at birth but still clamped to `Stats::max_level`, so a pet
    /// cannot start above the ceiling that `upgrade` enforces and then be stuck
    /// with a level the rest of the module cannot reason about.
    public entry fun mint_with_level(
        pet_cap: &mut NftCap,
        collection: &Collection,
        recipient: address,
        stats: &mut Stats,
        initial_level: u64,
        ctx: &mut TxContext,
    ) {
        assert!(initial_level <= stats.max_level, E_LEVEL_OUT_OF_RANGE);
        // Attributes are validated and progression stamped before the supply
        // unit is consumed, so a rejected level cannot cost a mint slot.
        let attribute_keys = vector::empty<String>();
        let attribute_values = vector::empty<String>();
        let (attribute_keys, attribute_values) =
            seed_progression(
                stats,
                initial_level,
                attribute_keys,
                attribute_values
            );
        let pet =
            krc_721::mint(
                pet_cap,
                collection,
                string::utf8(b"Kanari Pet"),
                option::some(string::utf8(b"An NFT built on the Kanari Sui-parity stack")),
                option::some(url::new_unsafe_from_bytes(DEFAULT_IMAGE_URL)),
                attribute_keys,
                attribute_values,
                ctx
            );
        table::add(&mut stats.levels, krc_721::nft_address(&pet), initial_level);
        krc_721::transfer(pet, recipient);
    }

    /// Power for `level`: purely a function of level, so two pets at the same
    /// level can never disagree and no path can mint an inconsistent rating.
    public fun power_for_level(stats: &Stats, lvl: u64): u64 {
        stats.power_per_level * lvl
    }

    /// Rarity band for `level`.
    ///
    /// Bands are a fixed ladder rather than something a caller can pass in, so
    /// "legendary" is a claim the chain agrees with.
    public fun rarity_for_level(lvl: u64): String {
        if (lvl >= 40) {
            string::utf8(b"Legendary")
        } else if (lvl >= 25) {
            string::utf8(b"Epic")
        } else if (lvl >= 10) {
            string::utf8(b"Rare")
        } else if (lvl >= 5) {
            string::utf8(b"Uncommon")
        } else {
            string::utf8(b"Common")
        }
    }

    /// The upgrade treasury.
    public fun treasury(stats: &Stats): address {
        stats.treasury
    }

    /// The level ceiling.
    public fun max_level(stats: &Stats): u64 {
        stats.max_level
    }

    #[test_only]
    /// Create a `Stats` object for tests.
    ///
    /// **`#[test_only]` is the security control, not a convenience.** This is
    /// deliberately unreachable outside tests: `upgrade` and `mint_with_level`
    /// take a `&mut Stats`, so a publicly constructible `Stats` would let any
    /// caller pass their own economics — a one-unit price, a million power per
    /// level, no ceiling — and buy arbitrary progression for pennies. `init` is
    /// the only thing that may build one, which makes the shared object the
    /// single source of truth.
    ///
    /// See `test_exploit_arbitrary_stats_is_impossible`.
    public fun new_stats(
        treasury: address,
        base_cost_usd: u64,
        power_per_level: u64,
        max_level: u64,
        ctx: &mut TxContext,
    ): Stats {
        Stats {
            id: object::new(ctx),
            treasury,
            base_cost_usd,
            power_per_level,
            max_level,
            levels: table::new<address, u64>(ctx)
        }
    }

    // ==================== Progression: internals ====================

    /// Convert a `vector<vector<u8>>` of raw bytes into `vector<String>`.
    ///
    /// The entry boundary takes bytes because that is what a CLI can serialize;
    /// `vector<String>` has no practical wire encoding. Conversion happens once,
    /// at the edge, so everything downstream works in `String`.
    public fun utf8_all(items: vector<vector<u8>>): vector<String> {
        let out = vector::empty<String>();
        let i = 0;
        let n = vector::length(&items);
        while (i < n) {
            vector::push_back(&mut out, string::utf8(*vector::borrow(&items, i)));
            i = i + 1;
        };
        out
    }

    /// True iff `key` is one of the paid-progression attributes.
    public fun is_reserved(key: &String): bool {
        let k = *key;
        k == string::utf8(b"power")
            || k == string::utf8(b"level")
            || k == string::utf8(b"rarity")
    }

    /// Write display metadata for a reserved key. The game never reads these
    /// values as authoritative progression; only `Stats` is trusted.
    fun set_reserved(pet: &mut Nft, key: vector<u8>, value: String) {
        krc_721::set_attribute(pet, string::utf8(key), value);
    }

    /// Decimal rendering of `value`, written by hand because this edition has no
    /// numeric-to-string helper. Only ever called on small values.
    fun u64_string(value: u64): String {
        if (value == 0) {
            return string::utf8(b"0")
        };
        let digits = vector::empty<u8>();
        let n = value;
        while (n > 0) {
            let d = ((n % 10) as u8) + 48;
            vector::push_back(&mut digits, d);
            n = n / 10;
        };
        vector::reverse(&mut digits);
        string::utf8(digits)
    }

    /// Transfer a pet directly, with no trade involved.
    ///
    /// No `TransferRequest` is created: a direct transfer has no sale price for
    /// a royalty rule to charge against. If the owner wants rules on every
    /// movement, route through `trade` instead.
    public entry fun transfer_pet(pet: Nft, recipient: address) {
        krc_721::transfer(pet, recipient);
    }

    /// Sell a pet: open a request, let the royalty rule take its cut, confirm.
    ///
    /// The whole enforcement path in one function. `royalty_rule::pay` is what
    /// adds the receipt; `confirm_request` aborts without it. Because
    /// `TransferRequest` has no `drop`, omitting either step fails the whole
    /// transaction instead of letting a free trade through.
    public entry fun trade(
        policy: &mut TransferPolicy<Nft>,
        policy_scope: &CollectionPolicyScope,
        pet: Nft,
        payment: &mut Coin<FEE>,
        recipient: address,
        ctx: &mut TxContext,
    ) {
        let from = object::id_from_address(tx_context::sender(ctx));
        let paid = coin::value(payment);
        let request =
            krc_721::new_collection_request(policy, policy_scope, &pet, paid, from);
        let request = pay_royalty(policy, request, payment, ctx);

        let (_item, _paid, _from) = transfer_policy::confirm_request<Nft>(
            policy, request
        );
        krc_721::transfer(pet, recipient);
    }

    /// `royalty_rule::pay` needs a `&mut TransferRequest`, and this edition has
    /// no `let mut` bindings, so the hot potato is threaded through a helper
    /// rather than rebound at the call site. Returning it keeps ownership: the
    /// value is still the same hot potato, merely moved.
    fun pay_royalty(
        policy: &mut TransferPolicy<Nft>,
        request: transfer_policy::TransferRequest<Nft>,
        payment: &mut Coin<FEE>,
        ctx: &mut TxContext,
    ): transfer_policy::TransferRequest<Nft> {
        let mut_ref = &mut request;
        royalty_rule::pay<Nft, FEE>(policy, mut_ref, payment, ctx);
        request
    }

    /// The royalty owed on a sale of `paid` base units, quotable before a trade
    /// so a buyer can price it.
    public fun quote_royalty(policy: &TransferPolicy<Nft>, paid: u64): u64 {
        royalty_rule::fee_amount<Nft>(policy, paid)
    }

    /// Read a field off the collection-scoped KRC-721 display.
    public fun display_field(
        d: &CollectionDisplay,
        key: vector<u8>,
    ): option::Option<String> {
        krc_721::collection_display_field(d, key)
    }

    /// The display's object address, for clients that want to find it.
    public fun display_address(d: &CollectionDisplay): address {
        krc_721::collection_display_address(d)
    }

    #[test_only]
    /// The fee witness, for tests that need a `Coin<FEE>`.
    public fun fee_witness(): FEE {
        FEE {}
    }

    #[test_only]
    /// Mint straight into the caller's hands. `mint` transfers to the tx sender,
    /// which unit tests cannot retrieve objects from, so the tests need a way to
    /// hold the pet in order to read and mutate its attributes.
    public fun mint_for_test(
        pet_cap: &mut NftCap,
        collection: &Collection,
        ctx: &mut TxContext,
    ): Nft {
        mint_with_attributes_for_test(
            pet_cap,
            collection,
            vector::empty<String>(),
            vector::empty<String>(),
            ctx
        )
    }

    #[test_only]
    /// As `mint_for_test`, but with attributes, so the keyed-attribute paths can
    /// be exercised on a pet that was minted with them.
    public fun mint_with_attributes_for_test(
        pet_cap: &mut NftCap,
        collection: &Collection,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
        ctx: &mut TxContext,
    ): Nft {
        krc_721::mint(
            pet_cap,
            collection,
            string::utf8(b"Kanari Pet"),
            option::some(string::utf8(b"Test pet")),
            option::some(url::new_unsafe_from_bytes(DEFAULT_IMAGE_URL)),
            attribute_keys,
            attribute_values,
            ctx
        )
    }

    #[test_only]
    /// `mint_with_level` returning the pet instead of transferring it, so a
    /// test can hold it across an upgrade.
    public fun mint_with_level_for_test(
        pet_cap: &mut NftCap,
        collection: &Collection,
        stats: &mut Stats,
        initial_level: u64,
        ctx: &mut TxContext,
    ): Nft {
        // Same ceiling check as `mint_with_level`, so the test variant cannot
        // construct a pet the public entry point would refuse.
        assert!(initial_level <= stats.max_level, E_LEVEL_OUT_OF_RANGE);
        let attribute_keys = vector::empty<String>();
        let attribute_values = vector::empty<String>();
        let (attribute_keys, attribute_values) =
            seed_progression(
                stats,
                initial_level,
                attribute_keys,
                attribute_values
            );
        let pet = krc_721::mint(
            pet_cap,
            collection,
            string::utf8(b"Kanari Pet"),
            option::some(string::utf8(b"Test pet")),
            option::some(url::new_unsafe_from_bytes(DEFAULT_IMAGE_URL)),
            attribute_keys,
            attribute_values,
            ctx
        );
        table::add(&mut stats.levels, krc_721::nft_address(&pet), initial_level);
        pet
    }

    /// Stamp progression display metadata onto a fresh KRC-721 attribute set.
    fun seed_progression(
        stats: &Stats,
        initial_level: u64,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
    ): (vector<String>, vector<String>) {
        let attribute_keys = attribute_keys;
        let attribute_values = attribute_values;
        let start_power = power_for_level(stats, initial_level);
        vector::push_back(&mut attribute_keys, string::utf8(ATTR_LEVEL));
        vector::push_back(&mut attribute_values, u64_string(initial_level));
        vector::push_back(&mut attribute_keys, string::utf8(ATTR_POWER));
        vector::push_back(&mut attribute_values, u64_string(start_power));
        vector::push_back(&mut attribute_keys, string::utf8(ATTR_RARITY));
        vector::push_back(&mut attribute_values, rarity_for_level(initial_level));
        (attribute_keys, attribute_values)
    }
}
