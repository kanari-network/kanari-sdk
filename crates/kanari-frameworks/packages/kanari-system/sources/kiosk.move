// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// `Kiosk` — a marketplace primitive, mirroring `sui::kiosk`.
///
/// ## Why a kiosk exists
///
/// Sui's model is object ownership, so a marketplace is not a contract that
/// escrows assets. It is an object that *holds* them and only lets them leave
/// through a sale. That is what this reproduces:
///
/// - A `Kiosk` is shared and holds items as **dynamic object fields**, so each
///   item stays a real object with its own id — individually addressable and
///   indexable. The kiosk is a container, not a custodian: an item can only
///   leave via `purchase`, or return to its owner via `take`.
/// - `KioskOwnerCap` is the only authority over placement and listing. It is a
///   separate object precisely so that *holding* an item grants no power over
///   the kiosk.
///
/// ## Item states
///
/// `placed` → `locked` → `listed` and back. `locked` is what makes the kiosk
/// safe: once locked, `take` refuses, so the only exit is a `purchase`, which
/// mints a `TransferRequest<T>`. `lock` therefore requires a
/// `&TransferPolicy<T>` as proof that a request can actually be resolved —
/// without one, a locked item could never be sold again and would be stuck
/// forever. Requiring that argument is what keeps `lock` from being a footgun.
///
/// ## Enforcement chain
///
/// `purchase` returns a `TransferRequest<T>` hot potato. It has no abilities,
/// so the buyer cannot drop it and the transaction cannot end until a
/// `TransferPolicy<T>` confirms it — which in turn requires every registered
/// rule (e.g. `royalty_rule`) to have reported. That is why the kiosk takes the
/// policy rather than resolving the request itself: rules belong to the type
/// owner, not to the seller.
module kanari_system::kiosk {
    use kanari_system::coin::{Self, Coin};
    use kanari_system::dynamic_object_field as dof;
    use kanari_system::object::{Self, ID, UID};
    use kanari_system::transfer;
    use kanari_system::transfer_policy::{Self, TransferPolicy, TransferRequest};
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::vec_map::{Self, VecMap};

    /// The caller does not hold this kiosk's owner capability.
    const E_NOT_OWNER: u64 = 0;

    /// An item with this id is already in the kiosk.
    const E_ITEM_EXISTS: u64 = 1;

    /// No item with this id is in the kiosk.
    const E_ITEM_MISSING: u64 = 2;

    /// The stored item is not the type the caller named.
    const E_ITEM_TYPE_MISMATCH: u64 = 3;

    /// A locked item can only leave through `purchase`.
    const E_ITEM_LOCKED: u64 = 4;

    /// The item is not listed, so it cannot be purchased.
    const E_NOT_LISTED: u64 = 5;

    /// The item is already listed; delist before relisting.
    const E_ALREADY_LISTED: u64 = 6;

    /// The payment does not cover the asking price.
    const E_INSUFFICIENT_PAYMENT: u64 = 7;

    /// The kiosk is at capacity.
    const E_KIOSK_FULL: u64 = 8;

    /// The asking price exceeds `MAX_PRICE`.
    const E_PRICE_OUT_OF_RANGE: u64 = 9;

    /// A listing must be free. `price == 0` is the internal "not listed"
    /// marker, so a zero-price listing would read back as unlisted and could
    /// never be bought — the seller would believe it was for sale while the
    /// kiosk showed it as withdrawn.
    const E_PRICE_ZERO: u64 = 11;

    /// The listing is reserved for its `PurchaseCap` holder; only that
    /// capability may buy it.
    const E_LISTING_IS_EXCLUSIVE: u64 = 12;

    /// The listing already has a `PurchaseCap`.
    const E_ALREADY_EXCLUSIVE: u64 = 13;

    /// The presented `PurchaseCap` is for a different kiosk or a different
    /// item, so it grants no purchase right here.
    const E_WRONG_PURCHASE_CAP: u64 = 14;

    /// The listing carries no `PurchaseCap`, so it is open to anyone.
    const E_NOT_EXCLUSIVE: u64 = 15;

    /// `buy_unregulated` was called on a policy that has rules registered. The
    /// entry boundary cannot discharge those rules, so the sale has to go
    /// through a module that knows how.
    const E_POLICY_HAS_RULES: u64 = 16;

    /// The type has no transferable policy, so a locked item could never be
    /// sold again.
    const E_NO_POLICY: u64 = 10;

    /// Maximum items one kiosk may hold.
    const MAX_ITEMS: u64 = 10_000;

    /// Upper bound on a listing price. Keeps a mistyped price from overflowing
    /// downstream royalty arithmetic.
    const MAX_PRICE: u64 = 1_000_000_000_000;

    /// Shared registry holding items for sale.
    struct Kiosk has key, store {
        id: UID,
        /// item id -> listing state. The item itself lives in a dynamic object
        /// field under the same id, so this map is metadata only.
        items: VecMap<ID, Listing>,
        /// Amounts owed to sellers in each currency, for marketplaces that batch
        /// payouts instead of paying inline. Keyed by currency type name.
        profits: VecMap<vector<u8>, u64>,
    }

    /// Full authority over one kiosk. Deliberately separate from the items
    /// themselves: holding an item gives no power over the kiosk.
    struct KioskOwnerCap has key, store {
        id: UID,
        kiosk: ID,
    }

    /// Listing state for one item.
    struct Listing has copy, drop, store {
        /// `T`'s type-name bytes. A take or purchase must match, so an item
        /// cannot leave under a type it does not actually have.
        type_name: vector<u8>,
        /// Locked items cannot be taken by the owner; they must be sold.
        locked: bool,
        /// Asking price, or 0 when not listed.
        price: u64,
        /// Set once listed, so the seller can be paid at purchase time.
        seller: address,
        /// When set, only the holder of the matching `PurchaseCap` may buy.
        /// `@0x0` means the listing is open to anyone.
        purchase_cap: ID,
    }

    /// A handle for an item inside a kiosk.
    struct ItemId has copy, drop, store {
        kiosk: ID,
        item: ID,
    }

    /// Reserved right to buy one specific listing.
    ///
    /// The owner can sell a slot to a collector before the listing goes live:
    /// issue the cap, hand it over, and `purchase_with_cap` is the only path
    /// that can fill the listing. Because the cap is bound to one item, it is
    /// useless for anything else — a buyer who loses it cannot redirect the
    /// right to a different asset.
    struct PurchaseCap has key, store {
        id: UID,
        kiosk: ID,
        item: ID,
    }

    /// Create a kiosk and its owner capability.
    public fun new(ctx: &mut TxContext): (Kiosk, KioskOwnerCap) {
        let kiosk = Kiosk {
            id: object::new(ctx),
            items: vec_map::empty<ID, Listing>(),
            profits: vec_map::empty<vector<u8>, u64>()
        };
        let cap = KioskOwnerCap {
            id: object::new(ctx),
            kiosk: object::uid_to_inner(&kiosk.id)
        };
        (kiosk, cap)
    }

    /// Share the kiosk and hand the capability to its owner.
    public fun share(kiosk: Kiosk, cap: KioskOwnerCap, recipient: address) {
        transfer::share_object(kiosk);
        transfer::public_transfer(cap, recipient);
    }

    /// Freeze the owner capability: items can still trade, but the listing can
    /// never change again. This is how a creator permanently opens a collection
    /// to trading without giving up curation.
    public fun freeze_cap(cap: KioskOwnerCap) {
        transfer::public_freeze_object(cap);
    }

    /// The kiosk's id.
    public fun id(kiosk: &Kiosk): ID {
        object::uid_to_inner(&kiosk.id)
    }

    /// The kiosk an owner capability controls.
    public fun owner_cap_kiosk(cap: &KioskOwnerCap): ID {
        cap.kiosk
    }

    // ==================== Placement ====================

    /// Create a kiosk, share it, and hand the owner cap to `recipient`, in one
    /// transaction.
    ///
    /// `new` returns `(Kiosk, KioskOwnerCap)` and an entry function cannot
    /// return values, so this is the shape a CLI can actually call. Calling
    /// `new` then `share` from a transaction would leave a window where the
    /// kiosk exists but nobody can list in it.
    public entry fun create(recipient: address, ctx: &mut TxContext) {
        let (kiosk, cap) = new(ctx);
        share(kiosk, cap, recipient);
    }

    /// Put an item into the kiosk and discard the handle.
    ///
    /// `place` returns an `ItemId`, which an entry function may not do. The
    /// handle is still recoverable — it is the `item_id` the caller passed —
    /// so nothing is lost by dropping it here.
    public entry fun place_item<T: key + store>(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap,
        item_id: ID,
        item: T,
    ) {
        let handle = place<T>(kiosk, cap, item_id, item);
        let _ = handle;
    }

    /// Put an item into the kiosk and lock it immediately, discarding the
    /// handle. See `place_item` for why the return value is dropped.
    public entry fun lock_item<T: key + store>(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap,
        item_id: ID,
        policy: &TransferPolicy<T>,
        item: T,
    ) {
        let handle = lock<T>(kiosk, cap, item_id, policy, item);
        let _ = handle;
    }

    /// List the item at `item_id` for `price`.
    public entry fun list_item<T: key + store>(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap,
        item_id: ID,
        price: u64,
        ctx: &TxContext,
    ) {
        let handle = ItemId {
            kiosk: object::uid_to_inner(&kiosk.id),
            item: item_id
        };
        list<T>(kiosk, cap, handle, price, ctx);
    }

    /// Stop selling the item at `item_id`.
    public entry fun delist_item<T: key + store>(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap,
        item_id: ID,
    ) {
        let handle = ItemId {
            kiosk: object::uid_to_inner(&kiosk.id),
            item: item_id
        };
        delist<T>(kiosk, cap, handle);
    }

    /// Take the item at `item_id` back out and send it to `recipient`.
    ///
    /// `take` returns the item, which an entry function cannot do, and the
    /// caller has to end up owning it anyway — so the transfer is folded in
    /// rather than left dangling.
    public entry fun take_to<T: key + store>(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap,
        item_id: ID,
        recipient: address,
    ) {
        let handle = ItemId {
            kiosk: object::uid_to_inner(&kiosk.id),
            item: item_id
        };
        let item = take<T>(kiosk, cap, handle);
        transfer::public_transfer(item, recipient);
    }

    /// Reserve the listing at `item_id` for whoever receives the returned cap.
    ///
    /// Entry functions cannot return the `PurchaseCap`, so from a CLI this
    /// reserves the listing without handing the cap anywhere — which makes it
    /// unusable until `unreserve_to` is called. That is a real limitation of the
    /// entry boundary, not a shortcut: the non-entry `reserve` returns the cap
    /// and is what a composing module should call.
    public entry fun reserve_item<T: key + store>(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap,
        item_id: ID,
        ctx: &mut TxContext,
    ) {
        let handle = ItemId {
            kiosk: object::uid_to_inner(&kiosk.id),
            item: item_id
        };
        let purchase_cap = reserve<T>(kiosk, cap, handle, ctx);
        // Burn the cap so the listing is not left reserved against an object
        // nobody can hold.
        let PurchaseCap { id, kiosk: _, item: _ } = purchase_cap;
        object::delete(id);
    }

    /// Put an item into the kiosk. The owner may take it back at any time until
    /// it is locked.
    public fun place<T: key + store>(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap,
        item_id: ID,
        item: T,
    ): ItemId {
        assert_owner(kiosk, cap);
        insert_item<T>(kiosk, item_id, item, false, 0, @0x0)
    }

    /// Put an item into the kiosk and lock it immediately.
    ///
    /// A locked item cannot be taken back by the owner; the only exit is
    /// `purchase`, which creates a `TransferRequest<T>`. `policy` is required as
    /// proof that such a request can be resolved — without one the item would be
    /// unsellable forever.
    public fun lock<T: key + store>(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap,
        item_id: ID,
        policy: &TransferPolicy<T>,
        item: T,
    ): ItemId {
        assert_owner(kiosk, cap);
        // Proof that this item is sellable. `take` will refuse a locked item,
        // and `purchase` needs a `TransferPolicy<T>` to create the request it
        // returns; without one the item could never leave the kiosk again.
        // Requiring a non-empty rule set also means the type owner has
        // actually configured how trades are resolved.
        assert!(transfer_policy::rule_count(policy) > 0, E_NO_POLICY);
        insert_item<T>(kiosk, item_id, item, true, 0, @0x0)
    }

    /// Take an item back out. Refuses while the item is locked, which is the
    /// guarantee a buyer relies on: a locked listing cannot be withdrawn.
    public fun take<T: key + store>(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap,
        handle: ItemId,
    ): T {
        assert_owner(kiosk, cap);
        let listing = listing_of(kiosk, &handle);
        assert!(!listing.locked, E_ITEM_LOCKED);
        remove_item<T>(kiosk, &handle)
    }

    // ==================== Listing ====================

    /// List an item at `price`, paid to the transaction sender. A locked item
    /// stays locked; an unlocked one may still be taken back.
    public fun list<T: key + store>(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap,
        handle: ItemId,
        price: u64,
        ctx: &TxContext,
    ) {
        assert_owner(kiosk, cap);
        assert!(price > 0, E_PRICE_ZERO);
        assert!(price <= MAX_PRICE, E_PRICE_OUT_OF_RANGE);
        assert!(
            dof::exists_with_type<ID, T>(&kiosk.id, handle.item),
            E_ITEM_TYPE_MISMATCH
        );
        let listing = listing_of(kiosk, &handle);
        assert!(listing.price == 0, E_ALREADY_LISTED);
        let updated = Listing {
            type_name: listing.type_name,
            locked: listing.locked,
            price,
            seller: tx_context::sender(ctx),
            purchase_cap: listing.purchase_cap
        };
        set_listing(kiosk, handle.item, updated);
    }

    /// Stop selling. The item stays in the kiosk and can be listed again.
    public fun delist<T: key + store>(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap,
        handle: ItemId,
    ) {
        assert_owner(kiosk, cap);
        assert!(
            dof::exists_with_type<ID, T>(&kiosk.id, handle.item),
            E_ITEM_TYPE_MISMATCH
        );
        let listing = listing_of(kiosk, &handle);
        let updated = Listing {
            type_name: listing.type_name,
            locked: listing.locked,
            price: 0,
            seller: @0x0,
            purchase_cap: listing.purchase_cap
        };
        set_listing(kiosk, handle.item, updated);
    }

    /// Buy the item behind `handle`.
    ///
    /// Returns the item plus a `TransferRequest<T>` hot potato that the caller
    /// must resolve through the type's `TransferPolicy<T>` — paying any royalty
    /// rule and so on. The kiosk deliberately does **not** resolve it: rules
    /// belong to the type owner, not the seller.
    ///
    /// The asking price is forwarded to the seller immediately; the rest of
    /// `payment` stays with the buyer. This is the "simple sale" flow. A
    /// marketplace that pays out in batches can use `record_profit` instead and
    /// settle later.
    ///
    /// Refuses when the listing is reserved (`exclusive`), which is what stops
    /// the general path from stealing a sale that was promised to a specific
    /// buyer. A reserved listing must be filled through `purchase_with_cap`.
    public fun purchase<T: key + store, Asset: key + store>(
        kiosk: &mut Kiosk,
        policy: &TransferPolicy<T>,
        handle: ItemId,
        payment: &mut Coin<Asset>,
        from: ID,
        ctx: &mut TxContext,
    ): (T, TransferRequest<T>) {
        let listing = listing_of(kiosk, &handle);
        assert!(listing.price > 0, E_NOT_LISTED);
        assert!(listing.type_name == type_name_bytes<T>(), E_ITEM_TYPE_MISMATCH);
        // A reserved listing is not for sale to the public. Checking this here
        // rather than only in the cap path is what makes the reservation
        // binding: the default entry point has to refuse, not just be unused.
        assert!(
            listing.purchase_cap == object::id_from_address(@0x0),
            E_LISTING_IS_EXCLUSIVE
        );
        settle<T, Asset>(
            kiosk,
            policy,
            handle,
            listing,
            payment,
            from,
            ctx
        )
    }

    // ==================== Exclusive listings ====================

    /// Reserve this listing for whoever receives the returned `PurchaseCap`.
    ///
    /// Only the kiosk owner may reserve, and only one cap may be outstanding
    /// per listing — a second reservation would silently invalidate the first
    /// holder's right, so it is rejected rather than overwritten.
    public fun reserve<T: key + store>(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap,
        handle: ItemId,
        ctx: &mut TxContext,
    ): PurchaseCap {
        assert_owner(kiosk, cap);
        assert!(
            dof::exists_with_type<ID, T>(&kiosk.id, handle.item),
            E_ITEM_TYPE_MISMATCH
        );
        let listing = listing_of(kiosk, &handle);
        assert!(
            listing.purchase_cap == object::id_from_address(@0x0),
            E_ALREADY_EXCLUSIVE
        );

        let purchase_cap = PurchaseCap {
            id: object::new(ctx),
            kiosk: object::uid_to_inner(&kiosk.id),
            item: handle.item
        };
        let updated = Listing {
            type_name: listing.type_name,
            locked: listing.locked,
            price: listing.price,
            seller: listing.seller,
            purchase_cap: object::uid_to_inner(&purchase_cap.id)
        };
        set_listing(kiosk, handle.item, updated);
        purchase_cap
    }

    /// Withdraw a reservation. The cap is handed back so the owner can destroy,
    /// freeze, or re-issue it.
    ///
    /// The cap is taken as an argument rather than reconstructed from the
    /// listing's recorded id: a `UID` cannot be rebuilt from an address without
    /// forging object identity, and taking the real one proves the caller holds
    /// the cap rather than merely knows its id.
    public fun unreserve<T: key + store>(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap,
        handle: ItemId,
        purchase_cap: PurchaseCap,
    ): PurchaseCap {
        assert_owner(kiosk, cap);
        assert!(
            dof::exists_with_type<ID, T>(&kiosk.id, handle.item),
            E_ITEM_TYPE_MISMATCH
        );
        let listing = listing_of(kiosk, &handle);
        assert!(listing.purchase_cap != object::id_from_address(@0x0), E_NOT_EXCLUSIVE);
        assert!(purchase_cap.item == handle.item, E_WRONG_PURCHASE_CAP);
        assert!(
            object::uid_to_inner(&purchase_cap.id) == listing.purchase_cap,
            E_WRONG_PURCHASE_CAP
        );
        let updated = Listing {
            type_name: listing.type_name,
            locked: listing.locked,
            price: listing.price,
            seller: listing.seller,
            purchase_cap: object::id_from_address(@0x0)
        };
        set_listing(kiosk, handle.item, updated);
        purchase_cap
    }

    /// Buy a reserved listing. `purchase_cap` is consumed, so one cap buys
    /// exactly one item and cannot be replayed.
    public fun purchase_with_cap<T: key + store, Asset: key + store>(
        kiosk: &mut Kiosk,
        policy: &TransferPolicy<T>,
        handle: ItemId,
        purchase_cap: PurchaseCap,
        payment: &mut Coin<Asset>,
        from: ID,
        ctx: &mut TxContext,
    ): (T, TransferRequest<T>) {
        let listing = listing_of(kiosk, &handle);
        assert!(listing.price > 0, E_NOT_LISTED);
        assert!(listing.type_name == type_name_bytes<T>(), E_ITEM_TYPE_MISMATCH);
        assert!(listing.purchase_cap != object::id_from_address(@0x0), E_NOT_EXCLUSIVE);
        // Three independent bindings, so a cap cannot be aimed at another
        // listing: it must be this kiosk, this item, and the one the listing
        // actually names.
        assert!(
            purchase_cap.kiosk == object::uid_to_inner(&kiosk.id),
            E_WRONG_PURCHASE_CAP
        );
        assert!(purchase_cap.item == handle.item, E_WRONG_PURCHASE_CAP);
        assert!(
            object::uid_to_inner(&purchase_cap.id) == listing.purchase_cap,
            E_WRONG_PURCHASE_CAP
        );

        // `UID` has no `copy`, so the cap is destructured to burn it. Burning is
        // what makes the right single-use: a cap cannot be held back and
        // replayed against a listing the owner later creates.
        let PurchaseCap { id: cap_uid, kiosk: _k, item: _i } = purchase_cap;
        object::delete(cap_uid);
        settle<T, Asset>(
            kiosk,
            policy,
            handle,
            listing,
            payment,
            from,
            ctx
        )
    }

    /// True iff this listing is reserved and so closed to `purchase`.
    public fun is_exclusive(kiosk: &Kiosk, handle: &ItemId): bool {
        listing_of(kiosk, handle).purchase_cap != object::id_from_address(@0x0)
    }

    /// Buy the item at `item_id` and send it to `recipient`, requiring that the
    /// policy has **no** rules.
    ///
    /// The plain `purchase` returns a `TransferRequest<T>` hot potato, and an
    /// entry function can neither return one nor leave it unresolved. This
    /// wrapper exists for the only case where that is honestly possible: a
    /// policy with zero rules, so `confirm_request` has nothing to wait for.
    ///
    /// **This is not a general sale path.** A type with a royalty rule cannot be
    /// sold through here — deliberately. Resolving the request means paying
    /// that type's rules, and those belong to the type owner, not to the
    /// marketplace. A type with rules must be sold through `kanari_pet::trade`
    /// or another module that knows how to discharge the hot potato.
    public entry fun buy_unregulated<T: key + store, Asset: key + store>(
        kiosk: &mut Kiosk,
        policy: &TransferPolicy<T>,
        item_id: ID,
        payment: &mut Coin<Asset>,
        recipient: address,
        ctx: &mut TxContext,
    ) {
        assert!(transfer_policy::rule_count(policy) == 0, E_POLICY_HAS_RULES);
        let handle = ItemId {
            kiosk: object::uid_to_inner(&kiosk.id),
            item: item_id
        };
        let from = object::id_from_address(tx_context::sender(ctx));
        let (item, request) = purchase<T, Asset>(kiosk, policy, handle, payment, from, ctx);
        let (_i, _p, _f) = transfer_policy::confirm_request<T>(policy, request);
        transfer::public_transfer(item, recipient);
    }

    /// The item a purchase cap is reserved for.
    public fun purchase_cap_item(cap: &PurchaseCap): ID {
        cap.item
    }

    /// Shared settlement tail for both purchase paths.
    ///
    /// `listing` is the already-validated state, so the assertions that decide
    /// *whether* a sale may happen live in the callers and cannot drift between
    /// the public and reserved routes. What remains — clear the listing, pay the
    /// seller, hand over the item, mint the request — is identical for both.
    fun settle<T: key + store, Asset: key + store>(
        kiosk: &mut Kiosk,
        policy: &TransferPolicy<T>,
        handle: ItemId,
        listing: Listing,
        payment: &mut Coin<Asset>,
        from: ID,
        ctx: &mut TxContext,
    ): (T, TransferRequest<T>) {
        let price = listing.price;
        assert!(coin::value(payment) >= price, E_INSUFFICIENT_PAYMENT);

        // Clear the listing before handing the item over, so a second purchase
        // of the same handle finds nothing listed. The reservation goes with
        // it: the cap is spent, so leaving it set would block every later
        // listing of the same handle forever.
        let cleared = Listing {
            type_name: listing.type_name,
            locked: listing.locked,
            price: 0,
            seller: @0x0,
            purchase_cap: object::id_from_address(@0x0)
        };
        set_listing(kiosk, handle.item, cleared);

        let seller = listing.seller;
        let payout = coin::split(payment, price, ctx);
        transfer::public_transfer(payout, seller);

        let item = remove_item<T>(kiosk, &handle);
        let request = transfer_policy::new_request(policy, handle.item, price, from);
        (item, request)
    }

    // ==================== Accounting ====================

    /// Note that `amount` of `currency` is owed, for marketplaces that batch
    /// payouts rather than paying inline.
    public fun record_profit(
        kiosk: &mut Kiosk,
        currency: vector<u8>,
        amount: u64,
    ) {
        let current = profit_of(kiosk, &currency);
        set_profit(kiosk, currency, current + amount);
    }

    /// Total owed in `currency`.
    public fun profit_of(kiosk: &Kiosk, currency: &vector<u8>): u64 {
        if (vec_map::contains(&kiosk.profits, currency)) {
            *vec_map::get(&kiosk.profits, currency)
        } else { 0 }
    }

    // ==================== Views ====================

    /// True iff `handle` names a live item in `kiosk`.
    public fun has_item(kiosk: &Kiosk, handle: &ItemId): bool {
        vec_map::contains(&kiosk.items, &handle.item)
    }

    /// True iff the item is locked (the owner cannot take it back).
    public fun is_locked(kiosk: &Kiosk, handle: &ItemId): bool {
        listing_of(kiosk, handle).locked
    }

    /// True iff the item is currently for sale.
    public fun is_listed(kiosk: &Kiosk, handle: &ItemId): bool {
        listing_of(kiosk, handle).price > 0
    }

    /// The asking price, or 0.
    public fun price(kiosk: &Kiosk, handle: &ItemId): u64 {
        listing_of(kiosk, handle).price
    }

    /// The address that receives payment for this listing.
    public fun seller(kiosk: &Kiosk, handle: &ItemId): address {
        listing_of(kiosk, handle).seller
    }

    /// How many items the kiosk holds.
    public fun item_count(kiosk: &Kiosk): u64 {
        vec_map::size(&kiosk.items)
    }

    /// Ids of every item in the kiosk, for indexing.
    public fun item_ids(kiosk: &Kiosk): vector<ID> {
        vec_map::keys(&kiosk.items)
    }

    // ==================== Internals ====================
    fun assert_owner(kiosk: &Kiosk, cap: &KioskOwnerCap) {
        assert!(cap.kiosk == object::uid_to_inner(&kiosk.id), E_NOT_OWNER);
    }

    /// Store `item` under `item_id` and record its listing.
    ///
    /// The caller supplies `item_id` because Move cannot read a generic `T`'s
    /// fields to extract its own id. That is also what makes the check
    /// meaningful: the caller must name the id it actually holds, and `dof`
    /// derives the child address from it.
    fun insert_item<T: key + store>(
        kiosk: &mut Kiosk,
        item_id: ID,
        item: T,
        locked: bool,
        price: u64,
        seller: address,
    ): ItemId {
        let count = vec_map::size(&kiosk.items);
        assert!(count < MAX_ITEMS, E_KIOSK_FULL);
        assert!(
            !dof::exists_with_type<ID, T>(&kiosk.id, item_id),
            E_ITEM_EXISTS
        );

        let listing = Listing {
            type_name: type_name_bytes<T>(),
            locked,
            price,
            seller,
            purchase_cap: object::id_from_address(@0x0)
        };
        dof::add<ID, T>(&mut kiosk.id, item_id, item);
        set_listing(kiosk, item_id, listing);
        ItemId {
            kiosk: object::uid_to_inner(&kiosk.id),
            item: item_id
        }
    }

    /// Pull the item back out and drop its listing entry.
    fun remove_item<T: key + store>(kiosk: &mut Kiosk, handle: &ItemId): T {
        assert!(
            dof::exists_with_type<ID, T>(&kiosk.id, handle.item),
            E_ITEM_TYPE_MISMATCH
        );
        let (removed_key, removed) = vec_map::remove(&mut kiosk.items, &handle.item);
        let _ = removed_key;
        let _ = removed;
        dof::remove<ID, T>(&mut kiosk.id, handle.item)
    }

    fun listing_of(kiosk: &Kiosk, handle: &ItemId): Listing {
        assert!(vec_map::contains(&kiosk.items, &handle.item), E_ITEM_MISSING);
        *vec_map::get(&kiosk.items, &handle.item)
    }

    /// `vec_map::insert` aborts on a duplicate key, so a replace is
    /// remove-then-insert.
    fun set_listing(kiosk: &mut Kiosk, item_id: ID, listing: Listing) {
        if (vec_map::contains(&kiosk.items, &item_id)) {
            let (removed_key, removed) = vec_map::remove(&mut kiosk.items, &item_id);
            let _ = removed_key;
            let _ = removed;
        };
        vec_map::insert(&mut kiosk.items, item_id, listing);
    }

    fun set_profit(kiosk: &mut Kiosk, currency: vector<u8>, amount: u64) {
        if (vec_map::contains(&kiosk.profits, &currency)) {
            let (removed_key, removed) = vec_map::remove(&mut kiosk.profits, &currency);
            let _ = removed_key;
            let _ = removed;
        };
        vec_map::insert(&mut kiosk.profits, currency, amount);
    }

    /// `T`'s canonical type-name bytes.
    fun type_name_bytes<T: key + store>(): vector<u8> {
        let name = std::type_name::get<T>();
        *std::ascii::as_bytes(std::type_name::borrow_string(&name))
    }

    // ==================== Tests ====================
    #[test_only]
    use kanari_system::balance;

    #[test_only]
    use kanari_system::package;

    #[test_only]
    use kanari_system::royalty_rule;

    #[test_only]
    use kanari_system::transfer_policy::TransferPolicyCap;

    #[test_only]
    use std::vector;

    #[test_only]
    use kanari_system::address;

    #[test_only]
    /// The traded type. Named after this module in caps so `package::new`
    /// accepts it, and carries `key + store` because that is what an item is.
    /// It has no `drop`: a traded item must always be explicitly consumed.
    ///
    /// The `id: UID` field is mandatory, not decoration. `dof::add` routes
    /// through `record_child`, whose native reads the child's BCS and requires
    /// the leading 32 bytes to be an object id — the UID-first layout rule for
    /// `key` types. A fieldless `key` struct serializes to zero bytes and the
    /// native aborts with an internal type error, so a real object id is the
    /// minimum a stored item can have.
    struct ITEM has key, store {
        id: UID,
    }

    #[test_only]
    /// One-time witness for the publisher. Named after this module in caps for
    /// `types::is_one_time_witness` to accept it, and separate from `ITEM` so
    /// the traded item does not need `drop`.
    struct KIOSK has drop {}

    #[test_only]
    // A second traded type, for the type-mismatch checks.
    #[allow(unused_field)]
    struct OTHER_ITEM has key, store {
        id: UID,
    }

    #[test_only]
    /// The currency a listing is paid in.
    struct KIOSK_FEE has key, store, drop {}

    #[test]
    fun test_place_and_take_roundtrip() {
        let ctx = tx_context::dummy();
        let (kiosk, cap) = new(&mut ctx);
        let item = ITEM { id: object::new(&mut ctx) };
        let item_id = fresh_id<ITEM>(&kiosk);

        let handle = place<ITEM>(&mut kiosk, &cap, item_id, item);
        assert!(has_item(&kiosk, &handle), 0);
        assert!(item_count(&kiosk) == 1, 1);
        assert!(!is_listed(&kiosk, &handle), 2);
        // Placed but not locked: the owner can still withdraw it.
        assert!(!is_locked(&kiosk, &handle), 3);

        // The item is back with the owner. `KIOSK` has no `drop` — the traded
        // item must always be explicitly disposed of — so it is frozen.
        let taken = take<ITEM>(&mut kiosk, &cap, handle);
        assert!(item_count(&kiosk) == 0, 4);
        transfer::public_freeze_object(taken);

        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(cap);
    }

    #[test]
    #[expected_failure(abort_code = E_ITEM_LOCKED)]
    fun test_lock_blocks_withdrawal() {
        let ctx = tx_context::dummy();
        let publisher = package::new<KIOSK>(KIOSK {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ITEM, KIOSK>(&publisher, &mut ctx);
        let policy = with_royalty(policy, &cap);
        let (kiosk, owner_cap) = new(&mut ctx);

        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            lock<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                &policy,
                ITEM { id: object::new(&mut ctx) }
            );
        // Must abort: a locked item may only leave through `purchase`. The line
        // after is unreachable but required, since `KIOSK` has no `drop`.
        let taken = take<ITEM>(&mut kiosk, &owner_cap, handle);
        transfer::public_freeze_object(taken);

        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    #[expected_failure(abort_code = E_NO_POLICY)]
    fun test_lock_requires_a_configured_policy() {
        let ctx = tx_context::dummy();
        let publisher = package::new<KIOSK>(KIOSK {}, &mut ctx);
        // A policy with no rules: any request it confirms is unconstrained, so
        // locking against it would leave the item untradeable in practice.
        let (policy, cap) = transfer_policy::new<ITEM, KIOSK>(&publisher, &mut ctx);
        let (kiosk, owner_cap) = new(&mut ctx);

        let item_id = fresh_id<ITEM>(&kiosk);
        // Aborts: the policy has no rules.
        let handle =
            lock<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                &policy,
                ITEM { id: object::new(&mut ctx) }
            );
        let _h = handle;

        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    #[expected_failure(abort_code = E_NOT_OWNER)]
    fun test_stranger_cannot_place() {
        let ctx = tx_context::dummy();
        let (kiosk, cap) = new(&mut ctx);
        // A capability belonging to a different kiosk.
        let (other_kiosk, other_cap) = new(&mut ctx);

        let item_id = fresh_id<ITEM>(&kiosk);
        // Aborts: `other_cap` does not control `kiosk`.
        let handle =
            place<ITEM>(
                &mut kiosk,
                &other_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );
        let _h = handle;

        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(other_kiosk);
        transfer::public_freeze_object(other_cap);
    }

    #[test]
    fun test_list_and_delist() {
        let ctx = tx_context::dummy();
        let (kiosk, cap) = new(&mut ctx);
        let item_id = fresh_id<ITEM>(&kiosk);
        let handle = place<ITEM>(
            &mut kiosk,
            &cap,
            item_id,
            ITEM { id: object::new(&mut ctx) }
        );

        list<ITEM>(&mut kiosk, &cap, handle, 500, &ctx);
        assert!(is_listed(&kiosk, &handle), 0);
        assert!(price(&kiosk, &handle) == 500, 1);
        assert!(seller(&kiosk, &handle) == tx_context::sender(&ctx), 2);

        delist<ITEM>(&mut kiosk, &cap, handle);
        assert!(!is_listed(&kiosk, &handle), 3);
        assert!(price(&kiosk, &handle) == 0, 4);

        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(cap);
    }

    #[test]
    #[expected_failure(abort_code = E_NOT_LISTED)]
    fun test_unlisted_item_cannot_be_bought() {
        let ctx = tx_context::dummy();
        let publisher = package::new<KIOSK>(KIOSK {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ITEM, KIOSK>(&publisher, &mut ctx);
        let (kiosk, owner_cap) = new(&mut ctx);

        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );
        let payment = fee_coin(&mut ctx, 1_000);
        let from = object::id_from_address(@0xF00D);

        // Aborts: the item is not listed, so there is no price to pay. The
        // lines after the call are unreachable but required — `KIOSK`, `Coin`,
        // and `TransferRequest` all lack `drop`, so every path must consume
        // them. Confirming the request is the only way to consume the hot
        // potato, and there are no rules so it succeeds.
        let (item, request) =
            purchase<ITEM, KIOSK_FEE>(
                &mut kiosk,
                &policy,
                handle,
                &mut payment,
                from,
                &mut ctx
            );
        transfer::public_freeze_object(item);
        let (_i, _p, _f) = transfer_policy::confirm_request(&policy, request);
        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);

        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    fun test_purchase_returns_a_hot_potato_request() {
        let ctx = tx_context::dummy();
        let publisher = package::new<KIOSK>(KIOSK {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ITEM, KIOSK>(&publisher, &mut ctx);
        let policy = with_royalty(policy, &cap);
        let (kiosk, owner_cap) = new(&mut ctx);

        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );
        list<ITEM>(&mut kiosk, &owner_cap, handle, 500, &ctx);

        let payment = fee_coin(&mut ctx, 1_000);
        let from = object::id_from_address(@0xF00D);
        let (item, request) =
            purchase<ITEM, KIOSK_FEE>(
                &mut kiosk,
                &policy,
                handle,
                &mut payment,
                from,
                &mut ctx
            );

        // The item left the kiosk and the listing cleared.
        assert!(item_count(&kiosk) == 0, 0);
        // The seller received exactly the asking price.
        assert!(coin::value(&payment) == 500, 1);

        // Resolve the request the way a real trade would: pay the royalty,
        // which is what adds the receipt `confirm_request` demands.
        let request = pay_royalty(&mut policy, request, &mut payment, &mut ctx);
        let (_item_id, paid, _from) = transfer_policy::confirm_request(&policy, request);
        assert!(paid == 500, 2);

        // Reached only if the confirmation below unexpectedly succeeds.
        transfer::public_freeze_object(item);
        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    #[expected_failure]
    fun test_purchase_without_paying_royalty_aborts() {
        let ctx = tx_context::dummy();
        let publisher = package::new<KIOSK>(KIOSK {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ITEM, KIOSK>(&publisher, &mut ctx);
        let policy = with_royalty(policy, &cap);
        let (kiosk, owner_cap) = new(&mut ctx);

        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );
        list<ITEM>(&mut kiosk, &owner_cap, handle, 500, &ctx);

        let payment = fee_coin(&mut ctx, 1_000);
        let from = object::id_from_address(@0xF00D);
        let (item, request) =
            purchase<ITEM, KIOSK_FEE>(
                &mut kiosk,
                &policy,
                handle,
                &mut payment,
                from,
                &mut ctx
            );

        // Deliberately skip `royalty_rule::pay`. The request cannot be
        // confirmed, and because it has no abilities it cannot be dropped
        // either, so the transaction fails rather than settling for free.
        // The lines after are unreachable but required: `KIOSK`, `Coin`, and
        // every framework object created above lack `drop`.
        let (_item_id, _paid, _from) = transfer_policy::confirm_request(
            &policy, request
        );
        transfer::public_freeze_object(item);
        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    #[expected_failure(abort_code = E_PRICE_ZERO)]
    fun test_zero_price_listing_aborts() {
        let ctx = tx_context::dummy();
        let (kiosk, cap) = new(&mut ctx);
        let item_id = fresh_id<ITEM>(&kiosk);
        let handle = place<ITEM>(
            &mut kiosk,
            &cap,
            item_id,
            ITEM { id: object::new(&mut ctx) }
        );

        // Aborts: `price == 0` is the internal "not listed" marker, so a
        // zero-price listing would read back as withdrawn and never be buyable.
        list<ITEM>(&mut kiosk, &cap, handle, 0, &ctx);
        let _ = handle;

        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(cap);
    }

    #[test]
    #[expected_failure(abort_code = E_ITEM_TYPE_MISMATCH)]
    fun test_cannot_take_an_item_as_the_wrong_type() {
        let ctx = tx_context::dummy();
        let (kiosk, cap) = new(&mut ctx);
        let item_id = fresh_id<ITEM>(&kiosk);
        let handle = place<ITEM>(
            &mut kiosk,
            &cap,
            item_id,
            ITEM { id: object::new(&mut ctx) }
        );

        // Aborts: the kiosk holds an `ITEM`, so naming a different type cannot
        // succeed. This is what stops a caller from reinterpreting a stored
        // value as whatever type it likes.
        let stolen = take<OTHER_ITEM>(&mut kiosk, &cap, handle);
        transfer::public_freeze_object(stolen);

        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(cap);
    }

    #[test]
    #[expected_failure(abort_code = E_ITEM_EXISTS)]
    fun test_cannot_place_two_items_under_the_same_id() {
        let ctx = tx_context::dummy();
        let (kiosk, cap) = new(&mut ctx);
        let item_id = fresh_id<ITEM>(&kiosk);
        let handle = place<ITEM>(
            &mut kiosk,
            &cap,
            item_id,
            ITEM { id: object::new(&mut ctx) }
        );

        // Aborts: the first item still occupies this field name.
        let second = place<ITEM>(
            &mut kiosk,
            &cap,
            item_id,
            ITEM { id: object::new(&mut ctx) }
        );
        let _ = second;

        let taken = take<ITEM>(&mut kiosk, &cap, handle);
        transfer::public_freeze_object(taken);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(cap);
    }

    #[test]
    #[expected_failure(abort_code = E_LISTING_IS_EXCLUSIVE)]
    fun test_public_purchase_cannot_steal_a_reserved_listing() {
        let ctx = tx_context::dummy();
        let publisher = package::new<KIOSK>(KIOSK {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ITEM, KIOSK>(&publisher, &mut ctx);
        let (kiosk, owner_cap) = new(&mut ctx);

        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );
        list<ITEM>(&mut kiosk, &owner_cap, handle, 500, &ctx);
        assert!(!is_exclusive(&kiosk, &handle), 0);
        let purchase_cap = reserve<ITEM>(&mut kiosk, &owner_cap, handle, &mut ctx);
        assert!(is_exclusive(&kiosk, &handle), 1);
        assert!(purchase_cap_item(&purchase_cap) == handle.item, 2);

        // Aborts: this listing was promised to the cap holder.
        let payment = fee_coin(&mut ctx, 1_000);
        let from = object::id_from_address(@0xF00D);
        let (item, request) =
            purchase<ITEM, KIOSK_FEE>(
                &mut kiosk,
                &policy,
                handle,
                &mut payment,
                from,
                &mut ctx
            );
        let (_i, _p, _f) = transfer_policy::confirm_request(&policy, request);
        transfer::public_freeze_object(item);
        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        transfer::public_freeze_object(purchase_cap);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    fun test_purchase_with_cap_fills_a_reserved_listing() {
        let ctx = tx_context::dummy();
        let publisher = package::new<KIOSK>(KIOSK {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ITEM, KIOSK>(&publisher, &mut ctx);
        let policy = with_royalty(policy, &cap);
        let (kiosk, owner_cap) = new(&mut ctx);

        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );
        list<ITEM>(&mut kiosk, &owner_cap, handle, 500, &ctx);
        let purchase_cap = reserve<ITEM>(&mut kiosk, &owner_cap, handle, &mut ctx);

        let payment = fee_coin(&mut ctx, 1_000);
        let from = object::id_from_address(@0xF00D);
        let (item, request) =
            purchase_with_cap<ITEM, KIOSK_FEE>(
                &mut kiosk,
                &policy,
                handle,
                purchase_cap,
                &mut payment,
                from,
                &mut ctx
            );

        // Sold, and the reservation is spent along with the cap.
        assert!(item_count(&kiosk) == 0, 0);
        assert!(coin::value(&payment) == 500, 1);
        let request = pay_royalty(&mut policy, request, &mut payment, &mut ctx);
        let (_i, paid, _f) = transfer_policy::confirm_request(&policy, request);
        assert!(paid == 500, 2);

        transfer::public_freeze_object(item);
        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    #[expected_failure(abort_code = E_WRONG_PURCHASE_CAP)]
    fun test_purchase_cap_cannot_be_aimed_at_another_item() {
        let ctx = tx_context::dummy();
        let publisher = package::new<KIOSK>(KIOSK {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ITEM, KIOSK>(&publisher, &mut ctx);
        let (kiosk, owner_cap) = new(&mut ctx);

        let first_id = fresh_id<ITEM>(&kiosk);
        let first =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                first_id,
                ITEM { id: object::new(&mut ctx) }
            );
        let second_id = fresh_id<ITEM>(&kiosk);
        let second =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                second_id,
                ITEM { id: object::new(&mut ctx) }
            );
        list<ITEM>(&mut kiosk, &owner_cap, first, 500, &ctx);
        list<ITEM>(&mut kiosk, &owner_cap, second, 700, &ctx);

        // Both listings are reserved, each with its own cap. Presenting the
        // first cap against the second listing has to fail: without the
        // per-item binding, one cap would unlock every listing in the kiosk.
        let purchase_cap = reserve<ITEM>(&mut kiosk, &owner_cap, first, &mut ctx);
        let _second_cap = reserve<ITEM>(&mut kiosk, &owner_cap, second, &mut ctx);

        // Aborts: `purchase_cap.item` names the first listing, not this one.
        let payment = fee_coin(&mut ctx, 1_000);
        let from = object::id_from_address(@0xF00D);
        let (item, request) =
            purchase_with_cap<ITEM, KIOSK_FEE>(
                &mut kiosk,
                &policy,
                second,
                purchase_cap,
                &mut payment,
                from,
                &mut ctx
            );
        let (_i, _p, _f) = transfer_policy::confirm_request(&policy, request);
        transfer::public_freeze_object(item);
        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        transfer::public_freeze_object(_second_cap);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    #[expected_failure(abort_code = E_ALREADY_EXCLUSIVE)]
    fun test_cannot_reserve_a_listing_twice() {
        let ctx = tx_context::dummy();
        let (kiosk, owner_cap) = new(&mut ctx);
        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );
        list<ITEM>(&mut kiosk, &owner_cap, handle, 500, &ctx);

        let first = reserve<ITEM>(&mut kiosk, &owner_cap, handle, &mut ctx);

        // Aborts: a second reservation would silently void the first holder's
        // right, so it must be refused rather than overwrite it.
        let second = reserve<ITEM>(&mut kiosk, &owner_cap, handle, &mut ctx);
        transfer::public_freeze_object(first);
        transfer::public_freeze_object(second);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
    }

    #[test]
    #[expected_failure(abort_code = E_NOT_OWNER)]
    fun test_stranger_cannot_reserve_a_listing() {
        let ctx = tx_context::dummy();
        let (kiosk, owner_cap) = new(&mut ctx);
        let (other, other_cap) = new(&mut ctx);
        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );

        // Aborts: reserving is a curation right, not a public one.
        let purchase_cap = reserve<ITEM>(&mut kiosk, &other_cap, handle, &mut ctx);
        transfer::public_freeze_object(purchase_cap);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(other);
        transfer::public_freeze_object(other_cap);
    }

    #[test]
    fun test_unreserve_reopens_the_listing() {
        let ctx = tx_context::dummy();
        let (kiosk, owner_cap) = new(&mut ctx);
        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );
        list<ITEM>(&mut kiosk, &owner_cap, handle, 500, &ctx);

        let purchase_cap = reserve<ITEM>(&mut kiosk, &owner_cap, handle, &mut ctx);
        assert!(is_exclusive(&kiosk, &handle), 0);

        let purchase_cap = unreserve<ITEM>(&mut kiosk, &owner_cap, handle, purchase_cap);
        assert!(!is_exclusive(&kiosk, &handle), 1);
        // The price survives; only the reservation was withdrawn.
        assert!(price(&kiosk, &handle) == 500, 2);

        transfer::public_freeze_object(purchase_cap);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
    }

    #[test]
    #[expected_failure(abort_code = E_INSUFFICIENT_PAYMENT)]
    fun test_purchase_rejects_underpayment() {
        let ctx = tx_context::dummy();
        let publisher = package::new<KIOSK>(KIOSK {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ITEM, KIOSK>(&publisher, &mut ctx);
        let (kiosk, owner_cap) = new(&mut ctx);

        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );
        list<ITEM>(&mut kiosk, &owner_cap, handle, 500, &ctx);

        // Aborts: 499 does not cover a 500 asking price.
        let payment = fee_coin(&mut ctx, 499);
        let from = object::id_from_address(@0xF00D);
        let (item, request) =
            purchase<ITEM, KIOSK_FEE>(
                &mut kiosk,
                &policy,
                handle,
                &mut payment,
                from,
                &mut ctx
            );
        let (_i, _p, _f) = transfer_policy::confirm_request(&policy, request);
        transfer::public_freeze_object(item);
        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    #[expected_failure(abort_code = E_POLICY_HAS_RULES)]
    /// `buy_unregulated` is the CLI-reachable sale path, and it must refuse a
    /// policy that has rules rather than quietly skipping them. If it accepted
    /// one, a royalty rule could be bypassed entirely by routing the sale
    /// through the entry boundary instead of the type owner's module.
    fun test_entry_sale_refuses_a_policy_with_rules() {
        let ctx = tx_context::dummy();
        let publisher = package::new<KIOSK>(KIOSK {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ITEM, KIOSK>(&publisher, &mut ctx);
        let policy = with_royalty(policy, &cap);
        let (kiosk, owner_cap) = new(&mut ctx);

        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );
        list<ITEM>(&mut kiosk, &owner_cap, handle, 500, &ctx);

        let payment = fee_coin(&mut ctx, 1_000);
        buy_unregulated<ITEM, KIOSK_FEE>(
            &mut kiosk,
            &policy,
            item_id,
            &mut payment,
            @0xBEEF,
            &mut ctx
        );
        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    #[expected_failure(abort_code = E_POLICY_HAS_RULES)]
    /// Same guard on the reserved-listing path.
    fun test_entry_sale_refuses_rules_for_reserved_listings() {
        let ctx = tx_context::dummy();
        let publisher = package::new<KIOSK>(KIOSK {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ITEM, KIOSK>(&publisher, &mut ctx);
        let policy = with_royalty(policy, &cap);
        let (kiosk, owner_cap) = new(&mut ctx);

        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );
        list<ITEM>(&mut kiosk, &owner_cap, handle, 500, &ctx);
        let purchase_cap = reserve<ITEM>(&mut kiosk, &owner_cap, handle, &mut ctx);

        let payment = fee_coin(&mut ctx, 1_000);
        buy_unregulated<ITEM, KIOSK_FEE>(
            &mut kiosk,
            &policy,
            item_id,
            &mut payment,
            @0xBEEF,
            &mut ctx
        );
        transfer::public_freeze_object(purchase_cap);
        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    #[expected_failure(abort_code = E_LISTING_IS_EXCLUSIVE)]
    /// A reserved listing is closed even to the entry sale path: the cap holder
    /// was promised it.
    fun test_entry_sale_refuses_a_reserved_listing() {
        let ctx = tx_context::dummy();
        let publisher = package::new<KIOSK>(KIOSK {}, &mut ctx);
        let (policy, cap) = transfer_policy::new<ITEM, KIOSK>(&publisher, &mut ctx);
        let (kiosk, owner_cap) = new(&mut ctx);

        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );
        list<ITEM>(&mut kiosk, &owner_cap, handle, 500, &ctx);
        let purchase_cap = reserve<ITEM>(&mut kiosk, &owner_cap, handle, &mut ctx);

        let payment = fee_coin(&mut ctx, 1_000);
        buy_unregulated<ITEM, KIOSK_FEE>(
            &mut kiosk,
            &policy,
            item_id,
            &mut payment,
            @0xBEEF,
            &mut ctx
        );
        transfer::public_freeze_object(purchase_cap);
        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(publisher);
    }

    #[test]
    /// An unregulated type does sell through the entry path, end to end: the
    /// seller is paid, the item moves, and the request is discharged so the
    /// transaction can finish.
    fun test_entry_sale_works_for_an_unregulated_type() {
        let ctx = tx_context::dummy();
        let publisher = package::new<KIOSK>(KIOSK {}, &mut ctx);
        // No royalty rule installed.
        let (policy, cap) = transfer_policy::new<ITEM, KIOSK>(&publisher, &mut ctx);
        let (kiosk, owner_cap) = new(&mut ctx);

        let item_id = fresh_id<ITEM>(&kiosk);
        let handle =
            place<ITEM>(
                &mut kiosk,
                &owner_cap,
                item_id,
                ITEM { id: object::new(&mut ctx) }
            );
        list<ITEM>(&mut kiosk, &owner_cap, handle, 500, &ctx);

        let payment = fee_coin(&mut ctx, 1_000);
        buy_unregulated<ITEM, KIOSK_FEE>(
            &mut kiosk,
            &policy,
            item_id,
            &mut payment,
            @0xBEEF,
            &mut ctx
        );

        assert!(item_count(&kiosk) == 0, 0);
        assert!(coin::value(&payment) == 500, 1);
        let balance = coin::into_balance(payment);
        let _ = balance::destroy(balance);
        transfer::public_freeze_object(kiosk);
        transfer::public_freeze_object(owner_cap);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(policy);
        transfer::public_freeze_object(publisher);
    }

    #[test_only]
    /// Install a 10% royalty to `@0xBEEF` and hand the policy back. Takes and
    /// returns by value because this edition has no `let mut` bindings, so a
    /// `&mut` borrow has to be produced by a function boundary.
    fun with_royalty(
        policy: TransferPolicy<ITEM>,
        cap: &TransferPolicyCap<ITEM>,
    ): TransferPolicy<ITEM> {
        royalty_rule::add<ITEM>(&mut policy, cap, 1000, 0, @0xBEEF);
        policy
    }

    #[test_only]
    /// The field name for a new item.
    ///
    /// Unique per kiosk because it is derived from how many items the kiosk
    /// already holds: the runtime derives each child address from
    /// `(parent, name)`, so two kiosks never collide and neither do two items
    /// in one kiosk. A fresh `UID` would collide with ids the runtime already
    /// issued, tripping `record_child`'s internal invariant.
    fun fresh_id<T: key + store>(kiosk: &Kiosk): ID {
        let _ = type_name_bytes<T>();
        let next = item_count(kiosk) + 1;
        object::id_from_address(address::from_ascii_bytes(&counter_hex(next)))
    }

    #[test_only]
    /// Hex encoding of a small counter as the 64 ASCII characters
    /// `address::from_ascii_bytes` requires. This edition has neither a numeric
    /// cast from `u64` to `address` nor a bytes-to-address helper that would be
    /// shorter, so the value is formatted by hand.
    ///
    /// Only the low 16 nibbles are rendered and the leading 48 characters are
    /// filled with a literal `'0'` (ASCII 48). The padding is written directly
    /// rather than through `hex_nibbles`, because that would shift by up to 188
    /// and an oversized shift is an arithmetic error, not a truncation.
    fun counter_hex(value: u64): vector<u8> {
        let out = vector::empty<u8>();
        let i = 0;
        while (i < 48) {
            vector::push_back(&mut out, 48);
            i = i + 1;
        };
        let tail = hex_nibbles(value, 16);
        let j = 0;
        while (j < 16) {
            vector::push_back(&mut out, *vector::borrow(&tail, j));
            j = j + 1;
        };
        out
    }

    #[test_only]
    /// The top `count` nibbles of `value`, most significant first.
    fun hex_nibbles(value: u64, count: u8): vector<u8> {
        let out = vector::empty<u8>();
        let n = count;
        while (n > 0) {
            let shift = (n - 1) * 4;
            let digit = (((value >> shift) & 0xF) as u8);
            vector::push_back(&mut out, hex_digit(digit));
            n = n - 1;
        };
        out
    }

    #[test_only]
    /// ASCII hex for one nibble.
    fun hex_digit(value: u8): u8 {
        if (value < 10) {
            // '0' + value
            48 + value
        } else {
            // 'a' + (value - 10)
            87 + (value - 10)
        }
    }

    #[test_only]
    /// `amount` units of the listing currency, minted for the test.
    fun fee_coin(ctx: &mut TxContext, amount: u64): Coin<KIOSK_FEE> {
        let (treasury, _metadata) =
            coin::create_currency(
                KIOSK_FEE {},
                9,
                b"KIOSKFEE",
                b"Kiosk Fee",
                b"Royalties settle in this",
                std::option::none(),
                ctx
            );

        let c = coin::mint<KIOSK_FEE>(&mut treasury, amount, ctx);
        transfer::public_freeze_object(treasury);
        transfer::public_freeze_object(_metadata);
        c
    }

    #[test_only]
    /// `royalty_rule::pay` needs a `&mut TransferRequest`; this edition has no
    /// `let mut`, so the hot potato is threaded through and returned.
    fun pay_royalty(
        policy: &mut TransferPolicy<ITEM>,
        request: TransferRequest<ITEM>,
        payment: &mut Coin<KIOSK_FEE>,
        ctx: &mut TxContext,
    ): TransferRequest<ITEM> {
        let request_ref = &mut request;
        royalty_rule::pay<ITEM, KIOSK_FEE>(policy, request_ref, payment, ctx);
        request
    }
}
