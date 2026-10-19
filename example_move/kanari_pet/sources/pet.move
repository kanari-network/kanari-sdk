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
    use std::string::String;
    use std::vector;
    use kanari_system::coin::{Self, Coin};
    use kanari_system::collection::{Self, Collection, NftCap};
    use kanari_system::display::{Self, Display};
    use kanari_system::event;
    use kanari_system::nft;
    use kanari_system::object::{Self, UID};
    use kanari_system::package::Publisher;
    use kanari_system::royalty_rule;
    use kanari_system::transfer;
    use kanari_system::transfer_policy::{Self, TransferPolicy, TransferPolicyCap};
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::vec_map::VecMap;

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

        transfer::public_transfer(cap, sender);
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
    public fun mint(
        pet_cap: &mut NftCap,
        collection: &Collection,
        recipient: address,
        ctx: &mut TxContext,
    ) {
        mint_with_attributes(
            pet_cap,
            collection,
            recipient,
            vector::empty<String>(),
            vector::empty<String>(),
            ctx,
        )
    }

    /// Mint a pet carrying per-token attributes, given as parallel key and
    /// value vectors.
    ///
    /// The vectors are only an input convenience — they are converted to a map
    /// immediately by `nft::build_attributes`, which aborts on a length
    /// mismatch. That is why this entry point can never construct a pet whose
    /// traits are ambiguously paired.
    public fun mint_with_attributes(
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
    public fun burn(
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

    /// Insert or overwrite one attribute.
    ///
    /// Anyone holding the pet may do this. Attributes are descriptive, not
    /// authoritative — nothing in the framework reads them to make a decision —
    /// so a holder editing their own token is not a security issue. The bound
    /// is enforced by `nft::set_attribute`.
    public fun set_attribute(
        pet: &mut KanariPet,
        key: String,
        value: String,
    ) {
        let pet_address = object::uid_address(&pet.id);
        let attrs = &mut pet.attributes;
        nft::set_attribute_on(attrs, key, value);
        event::emit(AttributesChanged { pet_id: pet_address });
    }

    /// Remove one attribute and return its value. Aborts if absent.
    public fun remove_attribute(
        pet: &mut KanariPet,
        key: &String,
    ): String {
        let pet_address = object::uid_address(&pet.id);
        let attrs = &mut pet.attributes;
        let value = nft::remove_attribute_from(attrs, key);
        event::emit(AttributesChanged { pet_id: pet_address });
        value
    }

    /// Replace the whole attribute set from parallel vectors.
    ///
    /// Validated before anything is mutated, so a rejected call leaves the
    /// existing attributes intact rather than half-applied.
    public fun set_attributes(
        pet: &mut KanariPet,
        attribute_keys: vector<String>,
        attribute_values: vector<String>,
    ) {
        let pet_address = object::uid_address(&pet.id);
        let replacement = nft::build_attributes(attribute_keys, attribute_values);
        let attrs = &mut pet.attributes;
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

    /// Transfer a pet directly, with no trade involved.
    ///
    /// No `TransferRequest` is created: a direct transfer has no sale price for
    /// a royalty rule to charge against. If the owner wants rules on every
    /// movement, route through `trade` instead.
    public fun transfer_pet(pet: KanariPet, recipient: address) {
        transfer::public_transfer(pet, recipient);
    }

    /// Sell a pet: open a request, let the royalty rule take its cut, confirm.
    ///
    /// The whole enforcement path in one function. `royalty_rule::pay` is what
    /// adds the receipt; `confirm_request` aborts without it. Because
    /// `TransferRequest` has no `drop`, omitting either step fails the whole
    /// transaction instead of letting a free trade through.
    public fun trade(
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
}
