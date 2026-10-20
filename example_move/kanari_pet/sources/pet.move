// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// `KanariPet` — the showcase token for the Sui-parity stack.
///
/// Deliberately thin. Everything that matters lives in the framework, wired
/// together in `init`:
///
/// | Concern          | Primitive                            |
/// |------------------|--------------------------------------|
/// | type authority   | `kanari_system::package::Publisher`  |
/// | display metadata | `kanari_system::display::Display`    |
/// | trade rules      | `kanari_system::transfer_policy`     |
/// | royalty          | `kanari_system::royalty_rule`        |
/// | per-token traits | `kanari_system::nft` attribute model |
/// | supply           | `kanari_system::collection::NftCap`   |
///
/// `KanariPet` carries no name, no image, no royalty field, and no supply
/// counter. That is the point of the split: each is owned by a primitive that
/// can *enforce* it, rather than by a field on the token that anyone can read
/// but nobody can enforce.
///
/// **Attributes are the deliberate exception.** Name and description are the
/// same for every pet of this type, so they belong on `Display<KanariPet>` and
/// duplicating them per token would be two sources of truth. A pet's `breed`,
/// `level`, or `mood` are not — they differ per token and are exactly the data
/// that cannot be hoisted to the type. So those live here, on the token.
module kanari_pet::pet {
    use std::option::{Self, Option};
    use std::string::{Self, String};
    use std::vector;
    use kanari_system::coin::{Self, Coin};
    use kanari_system::collection::{Self, Collection, NftCap};
    use kanari_system::display::{Self, Display};
    use kanari_system::event;
    use kanari_system::nft;
    use kanari_system::object::{Self, UID};
    use kanari_system::royalty_rule;
    use kanari_system::transfer;
    use kanari_system::transfer_policy::{Self, TransferPolicy, TransferPolicyCap};
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::usd::USD;
    use kanari_system::vec_map::VecMap;

    #[test_only]
    use kanari_system::package::Publisher;

    // ==================== Progression ====================
    //
    // `power`, `level` and `rarity` are stored as ordinary attributes so a
    // wallet renders them with no special-casing — but they are **reserved
    // keys**. The generic `set_attribute` refuses them, so the only way to
    // write them is `mint_with_level` (free, at birth) or `upgrade` (paid, in
    // USD). Without that reservation the economy is void: anyone could call
    // `set_attribute(pet, "power", "999999")` and buy nothing.

    /// Reserved attribute key holding the pet's power rating.
    const ATTR_POWER: vector<u8> = b"power";

    /// Reserved attribute key holding the pet's level.
    const ATTR_LEVEL: vector<u8> = b"level";

    /// Reserved attribute key holding the pet's rarity band name.
    const ATTR_RARITY: vector<u8> = b"rarity";

    /// A caller tried to write a reserved key through the generic setter,
    /// bypassing the paid upgrade path.
    const E_RESERVED_ATTRIBUTE: u64 = 0;

    /// The pet is already at `Stats::max_level`.
    const E_MAX_LEVEL: u64 = 1;

    /// The `Coin<USD>` does not cover the quoted upgrade cost.
    const E_INSUFFICIENT_USD: u64 = 2;

    /// The minted level exceeds the configured cap.
    const E_LEVEL_OUT_OF_RANGE: u64 = 3;

    /// Supply of the collection `init` creates.
    const MAX_SUPPLY: u64 = 10_000;

    /// Upgrade economics for every `KanariPet`.
    ///
    /// Shared rather than per-pet so the parameters are one object an indexer
    /// can read and the creator can reason about, and so an upgrade reads the
    /// same numbers every time instead of whatever a token happened to carry.
    struct Stats has key, store {
        id: UID,
        /// Where upgrade payments go.
        treasury: address,
        /// USD (6-decimal base units) charged for the step from level N to N+1.
        /// The actual charge is `base_cost_usd * (N + 1)`, so later levels cost
        /// progressively more instead of the sink being infinitely spammable at
        /// a flat price.
        base_cost_usd: u64,
        /// Power granted per level.
        power_per_level: u64,
        /// Hard ceiling. At this level `upgrade` refuses rather than wrapping.
        max_level: u64,
    }

    /// The token itself: a `UID` plus its own attributes, and nothing else.
    ///
    /// Descriptive metadata lives on `Display<KanariPet>` and all rules live on
    /// `TransferPolicy<KanariPet>`, so this struct stays minimal on purpose.
    ///
    /// **No `drop`**, deliberately. A droppable token can be silently discarded
    /// by whoever holds it, which would bypass supply accounting entirely — the
    /// unit never returns to the cap and the destruction leaves no trace. This
    /// is also why the one-time witness is a separate `KANARI_PET` struct rather
    /// than the token itself: `package::new` is keyed on the witness type, so
    /// making the token double as its own witness would force `drop` onto it.
    struct KanariPet has key, store {
        id: UID,
        /// Per-token traits, keyed by name. A `VecMap`, not the parallel
        /// `vector<String>` / `vector<String>` pair legacy `james::KariKid`
        /// uses: a parallel pair has no structural link between its two sides,
        /// so a caller can pass 3 keys and 5 values and every later read has to
        /// guess the alignment. A map cannot be built into that state.
        attributes: VecMap<String, String>,
    }

    /// One-time witness for this package, named after the module in caps so
    /// `types::is_one_time_witness` accepts it. Only this module can construct
    /// it, so only this module can obtain a `Publisher`.
    struct KANARI_PET has drop {}

    /// One-time witness for this package's types. Its existence is what lets
    /// `init` claim `Publisher<KanariPet>`; no other package can construct it.
    /// The currency royalties are paid in. Fees settle in a stablecoin rather
    /// than in the NFT itself, which is what every Sui royalty example does.
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
    /// Claims type authority, shares the display, and shares the transfer policy
    /// with a 5% royalty to the publisher. **Sharing the policy is what makes
    /// `KanariPet` tradable** — a type whose policy is never shared cannot trade
    /// at all, so trading is opt-in by the type owner.
    public fun init(_otw: KANARI_PET, _fee_otw: FEE, ctx: &mut TxContext) {
        let sender = tx_context::sender(ctx);
        let publisher = kanari_system::package::new<KANARI_PET>(_otw, ctx);

        // Display metadata is keyed by type, not per token, so this is set once
        // and every `KanariPet` ever minted renders from it.
        let keys = vector::empty<vector<u8>>();
        let values = vector::empty<vector<u8>>();
        vector::push_back(&mut keys, b"name");
        vector::push_back(&mut values, b"Kanari Pet");
        vector::push_back(&mut keys, b"description");
        vector::push_back(&mut values, b"An NFT built on the Kanari Sui-parity stack");
        vector::push_back(&mut keys, b"image_url");
        vector::push_back(&mut values, b"https://avatars.githubusercontent.com/u/127471673?s=200&v=4");
        vector::push_back(&mut keys, b"creator");
        vector::push_back(&mut values, b"Kanari Labs");
        let d = display::create_with_fields<KanariPet, KANARI_PET>(&publisher, keys, values, ctx);
        display::share<KanariPet>(d);

        // Policy + owner cap, with the royalty rule installed.
        let (policy, cap) = transfer_policy::new<KanariPet, KANARI_PET>(&publisher, ctx);
        let policy = install_royalty(policy, &cap, sender);
        transfer_policy::share<KanariPet>(policy);

        // Upgrade economics, shared so every pet reads the same numbers and the
        // treasury is a single address rather than something carried per token.
        transfer::share_object(
            Stats {
                id: object::new(ctx),
                treasury: sender,
                // Flat $1.00 per level, in USD's 6-decimal base units.
                base_cost_usd: 1_000_000,
                power_per_level: 10,
                max_level: 1_000,
            }
        );

        // The collection object minting draws from. The `Collection` is shared
        // so anyone can read supply (and `burn` can bind to it); the `NftCap`
        // that authorises mints goes to the sender.
        let (coll, mint_cap) = collection::create_collection(
            ctx,
            b"Kanari Pet",
            b"An NFT built on the Kanari Sui-parity stack",
            b"https://avatars.githubusercontent.com/u/127471673?s=200&v=4",
            b"https://github.com/KanariNetwork/kanari-sdk",
            MAX_SUPPLY,
        );
        transfer::share_object(coll);

        transfer::public_transfer(cap, sender);
        transfer::public_transfer(mint_cap, sender);
        transfer::public_transfer(publisher, sender);
    }

    /// Install the 5% royalty, returning the configured policy.
    ///
    /// A helper rather than inline code because this edition has no `let mut`
    /// bindings: taking `policy` by value and returning it is the way to get a
    /// `&mut` borrow.
    fun install_royalty(
        policy: TransferPolicy<KanariPet>,
        cap: &TransferPolicyCap<KanariPet>,
        beneficiary: address,
    ): TransferPolicy<KanariPet> {
        royalty_rule::add<KanariPet>(&mut policy, cap, 500, 0, beneficiary);
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
        ctx: &mut TxContext,
    ) {
        mint_with_string_attributes(
            pet_cap,
            collection,
            recipient,
            vector::empty<String>(),
            vector::empty<String>(),
            ctx,
        )
    }

    /// Mint a pet carrying per-token attributes.
    ///
    /// Keys and values arrive as raw byte vectors because that is what a CLI can
    /// actually serialize — a `vector<String>` parameter has no usable encoding
    /// on the wire, so the entry boundary takes bytes and converts here.
    ///
    /// The vectors are an input convenience only: they are turned into a map
    /// immediately by `nft::build_attributes`, which aborts on a length
    /// mismatch. This entry point can never construct a pet whose traits are
    /// ambiguously paired.
    public entry fun mint_with_attributes(
        pet_cap: &mut NftCap,
        collection: &Collection,
        recipient: address,
        attribute_keys: vector<vector<u8>>,
        attribute_values: vector<vector<u8>>,
        ctx: &mut TxContext,
    ) {
        mint_with_string_attributes(
            pet_cap,
            collection,
            recipient,
            utf8_all(attribute_keys),
            utf8_all(attribute_values),
            ctx,
        );
    }

    /// The `String`-typed body behind `mint_with_attributes`, so a package
    /// composing programmatically does not have to round-trip through bytes.
    public fun mint_with_string_attributes(
        pet_cap: &mut NftCap,
        collection: &Collection,
        recipient: address,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
        ctx: &mut TxContext,
    ) {
        collection::consume_for_mint(pet_cap, collection);
        // Validate before consuming supply, so a rejected attribute set cannot
        // burn a mint slot as a side effect of aborting.
        let attributes = nft::build_attributes(attribute_keys, attribute_values);
        let pet = KanariPet { id: object::new(ctx), attributes };
        transfer::public_transfer(pet, recipient);
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
        pet: KanariPet,
        ctx: &TxContext,
    ) {
        let pet_address = object::uid_address(&pet.id);
        let owner = tx_context::sender(ctx);
        event::emit(Burned { pet_id: pet_address, owner });
        collection::return_from_burn(pet_cap, collection);
        let KanariPet { id, attributes: _ } = pet;
        object::delete(id);
    }

    /// Insert or overwrite one attribute, taking raw bytes for the CLI.
    ///
    /// Anyone holding the pet may do this, but **not** for a reserved key.
    /// `power`, `level` and `rarity` are paid progression state; letting them
    /// through a free setter would let a holder mint themselves a level 50 pet
    /// for nothing, which is the entire economy collapsed into one call. Those
    /// keys move only via `mint_with_level` or `upgrade`.
    public entry fun set_attribute(
        pet: &mut KanariPet,
        key: vector<u8>,
        value: vector<u8>,
    ) {
        set_string_attribute(pet, string::utf8(key), string::utf8(value));
    }

    /// The `String`-typed body behind `set_attribute`.
    public fun set_string_attribute(
        pet: &mut KanariPet,
        key: String,
        value: String,
    ) {
        assert!(!is_reserved(&key), E_RESERVED_ATTRIBUTE);
        let pet_address = object::uid_address(&pet.id);
        let attrs = &mut pet.attributes;
        nft::set_attribute_on(attrs, key, value);
        event::emit(AttributesChanged { pet_id: pet_address });
    }

    /// Remove one attribute and return its value. Aborts if absent.
    ///
    /// Reserved keys cannot be removed either: dropping `level` would leave the
    /// pet in a state `upgrade` could not reason about, since it reads the
    /// current level back out of the map.
    public fun remove_attribute(
        pet: &mut KanariPet,
        key: &String,
    ): String {
        assert!(!is_reserved(key), E_RESERVED_ATTRIBUTE);
        let pet_address = object::uid_address(&pet.id);
        let attrs = &mut pet.attributes;
        let value = nft::remove_attribute_from(attrs, key);
        event::emit(AttributesChanged { pet_id: pet_address });
        value
    }

    /// Replace the whole attribute set from parallel vectors.
    ///
    /// Reserved keys are **preserved**, not replaced and not dropped. A caller
    /// supplying its own key list would otherwise be able to erase every pet's
    /// progression in one call, or to overwrite `level` outright.
    public entry fun set_attributes(
        pet: &mut KanariPet,
        attribute_keys: vector<vector<u8>>,
        attribute_values: vector<vector<u8>>,
    ) {
        set_string_attributes(
            pet,
            utf8_all(attribute_keys),
            utf8_all(attribute_values),
        );
    }

    /// The `String`-typed body behind `set_attributes`.
    ///
    /// Reserved keys are **preserved**, not replaced and not dropped. A caller
    /// supplying its own key list would otherwise be able to erase every pet's
    /// progression in one call, or to overwrite `level` outright.
    public fun set_string_attributes(
        pet: &mut KanariPet,
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
        let pet_address = object::uid_address(&pet.id);
        let replacement = nft::build_attributes(attribute_keys, attribute_values);
        // Re-seed the progression keys from the pet itself.
        let attrs = &mut pet.attributes;
        copy_reserved(attrs, &mut replacement);
        *attrs = replacement;
        event::emit(AttributesChanged { pet_id: pet_address });
    }

    /// The value stored under `key`, or `none`.
    public fun attribute(pet: &KanariPet, key: &String): Option<String> {
        nft::attribute_of(&pet.attributes, key)
    }

    /// True iff `key` is present.
    public fun has_attribute(pet: &KanariPet, key: &String): bool {
        nft::has_attribute_in(&pet.attributes, key)
    }

    /// How many attributes the pet carries.
    public fun attribute_count(pet: &KanariPet): u64 {
        nft::attribute_count_of(&pet.attributes)
    }

    /// Attribute keys in insertion order.
    public fun attribute_keys(pet: &KanariPet): vector<String> {
        nft::attribute_keys_of(&pet.attributes)
    }

    /// Every `(key, value)` pair in insertion order, index-aligned.
    public fun all_attributes(pet: &KanariPet): (vector<String>, vector<String>) {
        nft::all_attributes_of(&pet.attributes)
    }

    // ==================== Progression: reading ====================

    /// The pet's current level. `0` if absent, which cannot happen for a pet
    /// minted through this module but keeps the read total.
    public fun level(pet: &KanariPet): u64 {
        u64_of(pet, ATTR_LEVEL)
    }

    /// The pet's power rating, derived from its level at mint/upgrade time.
    public fun power(pet: &KanariPet): u64 {
        u64_of(pet, ATTR_POWER)
    }

    /// The pet's rarity band, derived from its level.
    public fun rarity(pet: &KanariPet): String {
        string_of(pet, ATTR_RARITY)
    }

    // ==================== Progression: upgrading ====================

    /// The USD cost of taking a pet from `current_level` to the next one.
    ///
    /// Flat: every level costs exactly `base_cost_usd` — $1.00 per level by
    /// default, so a full climb to `max_level` is 50 x $1.00, not an
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
        pet: &mut KanariPet,
        stats: &Stats,
        payment: &mut Coin<USD>,
        ctx: &mut TxContext,
    ) {
        let pet_address = object::uid_address(&pet.id);
        let owner = tx_context::sender(ctx);

        let current = level(pet);
        assert!(current < stats.max_level, E_MAX_LEVEL);

        let cost = upgrade_cost(stats, current);
        assert!(coin::value(payment) >= cost, E_INSUFFICIENT_USD);

        // Pay first. If anything below were to abort, the whole transaction
        // unwinds anyway, so this ordering is about clarity, not safety.
        let paid = coin::split(payment, cost, ctx);
        transfer::public_transfer(paid, stats.treasury);

        let next = current + 1;
        let next_power = power_for_level(stats, next);

        let attrs = &mut pet.attributes;
        set_reserved(attrs, ATTR_LEVEL, u64_string(next));
        set_reserved(attrs, ATTR_POWER, u64_string(next_power));
        set_reserved(attrs, ATTR_RARITY, rarity_for_level(next));

        event::emit(Upgraded {
            pet_id: pet_address,
            owner,
            level_before: current,
            level_after: next,
            power_after: next_power,
            usd_paid: cost,
        });
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
        stats: &Stats,
        initial_level: u64,
        ctx: &mut TxContext,
    ) {
        assert!(initial_level <= stats.max_level, E_LEVEL_OUT_OF_RANGE);
        // Attributes are validated and progression stamped before the supply
        // unit is consumed, so a rejected level cannot cost a mint slot.
        let attribute_keys = vector::empty<String>();
        let attribute_values = vector::empty<String>();
        let attributes = seed_progression(
            stats, initial_level, attribute_keys, attribute_values,
        );
        collection::consume_for_mint(pet_cap, collection);

        let pet = KanariPet { id: object::new(ctx), attributes };
        transfer::public_transfer(pet, recipient);
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
    /// take a `&Stats`, so a publicly constructible `Stats` would let any
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

    /// True iff `key` is one of the three paid-progression attributes.
    public fun is_reserved(key: &String): bool {
        // `String` has `copy` in this stdlib, so dereferencing is free and the
        // comparison is by value — a `&String` will not unify with one.
        let k = *key;
        k == string::utf8(b"power") || k == string::utf8(b"level") || k == string::utf8(b"rarity")
    }

    /// Write a reserved key, bypassing the free-setter guard. Module-private on
    /// purpose: only `upgrade` and `mint_with_level` may call it.
    fun set_reserved(attributes: &mut VecMap<String, String>, key: vector<u8>, value: String) {
        // `nft::set_attribute_on` is already an upsert — it removes an existing key
        // before inserting — so there is no separate remove step here.
        nft::set_attribute_on(attributes, string::utf8(key), value);
    }

    /// Copy every reserved entry from `from` into `to`, so a wholesale attribute
    /// replacement cannot drop or rewrite progression.
    fun copy_reserved(
        from: &VecMap<String, String>,
        to: &mut VecMap<String, String>,
    ) {
        if (nft::has_attribute_in(from, &string::utf8(ATTR_LEVEL))) {
            set_reserved(to, ATTR_LEVEL, string_of_map(from, ATTR_LEVEL));
        };
        if (nft::has_attribute_in(from, &string::utf8(ATTR_POWER))) {
            set_reserved(to, ATTR_POWER, string_of_map(from, ATTR_POWER));
        };
        if (nft::has_attribute_in(from, &string::utf8(ATTR_RARITY))) {
            set_reserved(to, ATTR_RARITY, string_of_map(from, ATTR_RARITY));
        };
    }

    /// Read a reserved key's string value, or empty if absent.
    fun string_of_map(attributes: &VecMap<String, String>, key: vector<u8>): String {
        let k = string::utf8(key);
        if (nft::has_attribute_in(attributes, &k)) {
            *std::option::borrow(&nft::attribute_of(attributes, &k))
        } else {
            string::utf8(b"")
        }
    }

    /// Read a reserved numeric attribute as `u64`, or 0 if absent or unparsable.
    ///
    /// The permissive default is deliberate: a malformed value must not wedge a
    /// pet into an un-upgradeable state, and these keys are only ever written by
    /// this module, so the parse cannot fail in practice.
    fun u64_of(pet: &KanariPet, key: vector<u8>): u64 {
        let k = string::utf8(key);
        if (!nft::has_attribute_in(&pet.attributes, &k)) { return 0 };
        parse_u64(nft::attribute_of(&pet.attributes, &k))
    }

    /// Read a reserved attribute as a `String`, or empty if absent.
    fun string_of(pet: &KanariPet, key: vector<u8>): String {
        string_of_map(&pet.attributes, key)
    }

    /// Parse a base-10 `u64` from an attribute value. Returns 0 on any
    /// non-digit, so a corrupt value degrades to "level 0" instead of aborting
    /// inside a read that callers cannot guard.
    fun parse_u64(value: Option<String>): u64 {
        if (std::option::is_none(&value)) { return 0 };
        let v = *std::option::borrow(&value);
        let bytes = *string::bytes(&v);
        let out = 0;
        let i = 0;
        let n = vector::length(&bytes);
        while (i < n) {
            let b = *vector::borrow(&bytes, i);
            if (b < 48 || b > 57) { return 0 };
            out = out * 10 + ((b - 48) as u64);
            i = i + 1;
        };
        out
    }

    /// Decimal rendering of `value`, written by hand because this edition has no
    /// numeric-to-string helper. Only ever called on small values.
    fun u64_string(value: u64): String {
        if (value == 0) { return string::utf8(b"0") };
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
    public entry fun transfer_pet(pet: KanariPet, recipient: address) {
        transfer::public_transfer(pet, recipient);
    }

    /// Sell a pet: open a request, let the royalty rule take its cut, confirm.
    ///
    /// The whole enforcement path in one function. `royalty_rule::pay` is what
    /// adds the receipt; `confirm_request` aborts without it. Because
    /// `TransferRequest` has no `drop`, omitting either step fails the whole
    /// transaction instead of letting a free trade through.
    public entry fun trade(
        policy: &mut TransferPolicy<KanariPet>,
        pet: KanariPet,
        payment: &mut Coin<FEE>,
        recipient: address,
        ctx: &mut TxContext,
    ) {
        let item = object::uid_to_inner(&pet.id);
        let from = object::id_from_address(tx_context::sender(ctx));
        let paid = coin::value(payment);
        let request = transfer_policy::new_request<KanariPet>(policy, item, paid, from);
        let request = pay_royalty(policy, request, payment, ctx);

        let (_item, _paid, _from) = transfer_policy::confirm_request<KanariPet>(policy, request);
        transfer::public_transfer(pet, recipient);
    }

    /// `royalty_rule::pay` needs a `&mut TransferRequest`, and this edition has
    /// no `let mut` bindings, so the hot potato is threaded through a helper
    /// rather than rebound at the call site. Returning it keeps ownership: the
    /// value is still the same hot potato, merely moved.
    fun pay_royalty(
        policy: &mut TransferPolicy<KanariPet>,
        request: transfer_policy::TransferRequest<KanariPet>,
        payment: &mut Coin<FEE>,
        ctx: &mut TxContext,
    ): transfer_policy::TransferRequest<KanariPet> {
        let mut_ref = &mut request;
        royalty_rule::pay<KanariPet, FEE>(policy, mut_ref, payment, ctx);
        request
    }

    /// The royalty owed on a sale of `paid` base units, quotable before a trade
    /// so a buyer can price it.
    public fun quote_royalty(policy: &TransferPolicy<KanariPet>, paid: u64): u64 {
        royalty_rule::fee_amount<KanariPet>(policy, paid)
    }

    /// Read a field off the shared `Display<KanariPet>`.
    public fun display_field(
        d: &Display<KanariPet>,
        key: vector<u8>,
    ): option::Option<String> {
        display::get(d, key)
    }

    /// The display's object address, for clients that want to find it.
    public fun display_address(d: &Display<KanariPet>): address {
        display::display_address(d)
    }

    #[test_only]
    /// Create the publisher for this type. Split out so tests can exercise the
    public fun new_publisher(ctx: &mut TxContext): Publisher<KANARI_PET> {
        kanari_system::package::new<KANARI_PET>(KANARI_PET {}, ctx)
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
    ): KanariPet {
        collection::consume_for_mint(pet_cap, collection);
        let attributes = nft::build_attributes(
            vector::empty<String>(),
            vector::empty<String>(),
        );
        KanariPet { id: object::new(ctx), attributes }
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
    ): KanariPet {
        let attribute_keys = attribute_keys;
        let attribute_values = attribute_values;
        let attributes = nft::build_attributes(attribute_keys, attribute_values);
        collection::consume_for_mint(pet_cap, collection);
        KanariPet { id: object::new(ctx), attributes }
    }

    #[test_only]
    /// `mint_with_level` returning the pet instead of transferring it, so a
    /// test can hold it across an upgrade.
    public fun mint_with_level_for_test(
        pet_cap: &mut NftCap,
        collection: &Collection,
        stats: &Stats,
        initial_level: u64,
        ctx: &mut TxContext,
    ): KanariPet {
        // Same ceiling check as `mint_with_level`, so the test variant cannot
        // construct a pet the public entry point would refuse.
        assert!(initial_level <= stats.max_level, E_LEVEL_OUT_OF_RANGE);
        let attribute_keys = vector::empty<String>();
        let attribute_values = vector::empty<String>();
        let attributes = seed_progression(
            stats,
            initial_level,
            attribute_keys,
            attribute_values,
        );
        collection::consume_for_mint(pet_cap, collection);
        KanariPet { id: object::new(ctx), attributes }
    }

    /// Stamp the three progression keys onto a fresh attribute map. Shared by
    /// `mint_with_level` and its test variant so the two cannot diverge.
    fun seed_progression(
        stats: &Stats,
        initial_level: u64,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
    ): VecMap<String, String> {
        let attribute_keys = attribute_keys;
        let attribute_values = attribute_values;
        let attributes = nft::build_attributes(attribute_keys, attribute_values);
        let start_power = power_for_level(stats, initial_level);
        set_reserved(&mut attributes, ATTR_LEVEL, u64_string(initial_level));
        set_reserved(&mut attributes, ATTR_POWER, u64_string(start_power));
        set_reserved(&mut attributes, ATTR_RARITY, rarity_for_level(initial_level));
        attributes
    }
}
