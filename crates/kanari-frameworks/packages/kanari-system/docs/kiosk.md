
<a name="0x2_kiosk"></a>

# Module `0x2::kiosk`

<code><a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a></code> — a marketplace primitive, mirroring <code>sui::kiosk</code>.


<a name="@Why_a_kiosk_exists_0"></a>

### Why a kiosk exists


Sui's model is object ownership, so a marketplace is not a contract that
escrows assets. It is an object that *holds* them and only lets them leave
through a sale. That is what this reproduces:

- A <code><a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a></code> is shared and holds items as **dynamic object fields**, so each
item stays a real object with its own id — individually addressable and
indexable. The kiosk is a container, not a custodian: an item can only
leave via <code>purchase</code>, or return to its owner via <code>take</code>.
- <code><a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a></code> is the only authority over placement and listing. It is a
separate object precisely so that *holding* an item grants no power over
the kiosk.


<a name="@Item_states_1"></a>

### Item states


<code>placed</code> → <code>locked</code> → <code>listed</code> and back. <code>locked</code> is what makes the kiosk
safe: once locked, <code>take</code> refuses, so the only exit is a <code>purchase</code>, which
mints a <code>TransferRequest&lt;T&gt;</code>. <code>lock</code> therefore requires a
<code>&TransferPolicy&lt;T&gt;</code> as proof that a request can actually be resolved —
without one, a locked item could never be sold again and would be stuck
forever. Requiring that argument is what keeps <code>lock</code> from being a footgun.


<a name="@Enforcement_chain_2"></a>

### Enforcement chain


<code>purchase</code> returns a <code>TransferRequest&lt;T&gt;</code> hot potato. It has no abilities,
so the buyer cannot drop it and the transaction cannot end until a
<code>TransferPolicy&lt;T&gt;</code> confirms it — which in turn requires every registered
rule (e.g. <code><a href="royalty_rule.md#0x2_royalty_rule">royalty_rule</a></code>) to have reported. That is why the kiosk takes the
policy rather than resolving the request itself: rules belong to the type
owner, not to the seller.


    -  [Why a kiosk exists](#@Why_a_kiosk_exists_0)
    -  [Item states](#@Item_states_1)
    -  [Enforcement chain](#@Enforcement_chain_2)
-  [Resource `Kiosk`](#0x2_kiosk_Kiosk)
-  [Resource `KioskOwnerCap`](#0x2_kiosk_KioskOwnerCap)
-  [Struct `Listing`](#0x2_kiosk_Listing)
-  [Struct `ItemId`](#0x2_kiosk_ItemId)
-  [Resource `PurchaseCap`](#0x2_kiosk_PurchaseCap)
-  [Constants](#@Constants_3)
-  [Function `new`](#0x2_kiosk_new)
-  [Function `share`](#0x2_kiosk_share)
-  [Function `freeze_cap`](#0x2_kiosk_freeze_cap)
-  [Function `id`](#0x2_kiosk_id)
-  [Function `owner_cap_kiosk`](#0x2_kiosk_owner_cap_kiosk)
-  [Function `create`](#0x2_kiosk_create)
-  [Function `place_item`](#0x2_kiosk_place_item)
-  [Function `lock_item`](#0x2_kiosk_lock_item)
-  [Function `list_item`](#0x2_kiosk_list_item)
-  [Function `delist_item`](#0x2_kiosk_delist_item)
-  [Function `take_to`](#0x2_kiosk_take_to)
-  [Function `reserve_item`](#0x2_kiosk_reserve_item)
-  [Function `place`](#0x2_kiosk_place)
-  [Function `lock`](#0x2_kiosk_lock)
-  [Function `take`](#0x2_kiosk_take)
-  [Function `list`](#0x2_kiosk_list)
-  [Function `delist`](#0x2_kiosk_delist)
-  [Function `purchase`](#0x2_kiosk_purchase)
-  [Function `reserve`](#0x2_kiosk_reserve)
-  [Function `unreserve`](#0x2_kiosk_unreserve)
-  [Function `purchase_with_cap`](#0x2_kiosk_purchase_with_cap)
-  [Function `is_exclusive`](#0x2_kiosk_is_exclusive)
-  [Function `buy_unregulated`](#0x2_kiosk_buy_unregulated)
-  [Function `purchase_cap_item`](#0x2_kiosk_purchase_cap_item)
-  [Function `settle`](#0x2_kiosk_settle)
-  [Function `record_profit`](#0x2_kiosk_record_profit)
-  [Function `profit_of`](#0x2_kiosk_profit_of)
-  [Function `has_item`](#0x2_kiosk_has_item)
-  [Function `is_locked`](#0x2_kiosk_is_locked)
-  [Function `is_listed`](#0x2_kiosk_is_listed)
-  [Function `price`](#0x2_kiosk_price)
-  [Function `seller`](#0x2_kiosk_seller)
-  [Function `item_count`](#0x2_kiosk_item_count)
-  [Function `item_ids`](#0x2_kiosk_item_ids)
-  [Function `assert_owner`](#0x2_kiosk_assert_owner)
-  [Function `insert_item`](#0x2_kiosk_insert_item)
-  [Function `remove_item`](#0x2_kiosk_remove_item)
-  [Function `listing_of`](#0x2_kiosk_listing_of)
-  [Function `set_listing`](#0x2_kiosk_set_listing)
-  [Function `set_profit`](#0x2_kiosk_set_profit)
-  [Function `type_name_bytes`](#0x2_kiosk_type_name_bytes)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/ascii.md#0x1_ascii">0x1::ascii</a>;
<b>use</b> <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">0x1::type_name</a>;
<b>use</b> <a href="coin.md#0x2_coin">0x2::coin</a>;
<b>use</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field">0x2::dynamic_object_field</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="transfer_policy.md#0x2_transfer_policy">0x2::transfer_policy</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
<b>use</b> <a href="vec_map.md#0x2_vec_map">0x2::vec_map</a>;
</code></pre>



<a name="0x2_kiosk_Kiosk"></a>

## Resource `Kiosk`

Shared registry holding items for sale.


<pre><code><b>struct</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="object.md#0x2_object_UID">object::UID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>items: <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;<a href="object.md#0x2_object_ID">object::ID</a>, <a href="kiosk.md#0x2_kiosk_Listing">kiosk::Listing</a>&gt;</code>
</dt>
<dd>
 item id -> listing state. The item itself lives in a dynamic object
 field under the same id, so this map is metadata only.
</dd>
<dt>
<code>profits: <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;</code>
</dt>
<dd>
 Amounts owed to sellers in each currency, for marketplaces that batch
 payouts instead of paying inline. Keyed by currency type name.
</dd>
</dl>


</details>

<a name="0x2_kiosk_KioskOwnerCap"></a>

## Resource `KioskOwnerCap`

Full authority over one kiosk. Deliberately separate from the items
themselves: holding an item gives no power over the kiosk.


<pre><code><b>struct</b> <a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="object.md#0x2_object_UID">object::UID</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="kiosk.md#0x2_kiosk">kiosk</a>: <a href="object.md#0x2_object_ID">object::ID</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_kiosk_Listing"></a>

## Struct `Listing`

Listing state for one item.


<pre><code><b>struct</b> <a href="kiosk.md#0x2_kiosk_Listing">Listing</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 <code>T</code>'s type-name bytes. A take or purchase must match, so an item
 cannot leave under a type it does not actually have.
</dd>
<dt>
<code>locked: bool</code>
</dt>
<dd>
 Locked items cannot be taken by the owner; they must be sold.
</dd>
<dt>
<code>price: u64</code>
</dt>
<dd>
 Asking price, or 0 when not listed.
</dd>
<dt>
<code>seller: <b>address</b></code>
</dt>
<dd>
 Set once listed, so the seller can be paid at purchase time.
</dd>
<dt>
<code>purchase_cap: <a href="object.md#0x2_object_ID">object::ID</a></code>
</dt>
<dd>
 When set, only the holder of the matching <code><a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a></code> may buy.
 <code>@0x0</code> means the listing is open to anyone.
</dd>
</dl>


</details>

<a name="0x2_kiosk_ItemId"></a>

## Struct `ItemId`

A handle for an item inside a kiosk.


<pre><code><b>struct</b> <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="kiosk.md#0x2_kiosk">kiosk</a>: <a href="object.md#0x2_object_ID">object::ID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>item: <a href="object.md#0x2_object_ID">object::ID</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_kiosk_PurchaseCap"></a>

## Resource `PurchaseCap`

Reserved right to buy one specific listing.

The owner can sell a slot to a collector before the listing goes live:
issue the cap, hand it over, and <code>purchase_with_cap</code> is the only path
that can fill the listing. Because the cap is bound to one item, it is
useless for anything else — a buyer who loses it cannot redirect the
right to a different asset.


<pre><code><b>struct</b> <a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="object.md#0x2_object_UID">object::UID</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="kiosk.md#0x2_kiosk">kiosk</a>: <a href="object.md#0x2_object_ID">object::ID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>item: <a href="object.md#0x2_object_ID">object::ID</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_3"></a>

## Constants


<a name="0x2_kiosk_E_ALREADY_EXCLUSIVE"></a>

The listing already has a <code><a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a></code>.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_ALREADY_EXCLUSIVE">E_ALREADY_EXCLUSIVE</a>: u64 = 13;
</code></pre>



<a name="0x2_kiosk_E_ALREADY_LISTED"></a>

The item is already listed; delist before relisting.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_ALREADY_LISTED">E_ALREADY_LISTED</a>: u64 = 6;
</code></pre>



<a name="0x2_kiosk_E_INSUFFICIENT_PAYMENT"></a>

The payment does not cover the asking price.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_INSUFFICIENT_PAYMENT">E_INSUFFICIENT_PAYMENT</a>: u64 = 7;
</code></pre>



<a name="0x2_kiosk_E_ITEM_EXISTS"></a>

An item with this id is already in the kiosk.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_ITEM_EXISTS">E_ITEM_EXISTS</a>: u64 = 1;
</code></pre>



<a name="0x2_kiosk_E_ITEM_LOCKED"></a>

A locked item can only leave through <code>purchase</code>.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_ITEM_LOCKED">E_ITEM_LOCKED</a>: u64 = 4;
</code></pre>



<a name="0x2_kiosk_E_ITEM_MISSING"></a>

No item with this id is in the kiosk.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_ITEM_MISSING">E_ITEM_MISSING</a>: u64 = 2;
</code></pre>



<a name="0x2_kiosk_E_ITEM_TYPE_MISMATCH"></a>

The stored item is not the type the caller named.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_ITEM_TYPE_MISMATCH">E_ITEM_TYPE_MISMATCH</a>: u64 = 3;
</code></pre>



<a name="0x2_kiosk_E_KIOSK_FULL"></a>

The kiosk is at capacity.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_KIOSK_FULL">E_KIOSK_FULL</a>: u64 = 8;
</code></pre>



<a name="0x2_kiosk_E_LISTING_IS_EXCLUSIVE"></a>

The listing is reserved for its <code><a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a></code> holder; only that
capability may buy it.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_LISTING_IS_EXCLUSIVE">E_LISTING_IS_EXCLUSIVE</a>: u64 = 12;
</code></pre>



<a name="0x2_kiosk_E_NOT_EXCLUSIVE"></a>

The listing carries no <code><a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a></code>, so it is open to anyone.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_NOT_EXCLUSIVE">E_NOT_EXCLUSIVE</a>: u64 = 15;
</code></pre>



<a name="0x2_kiosk_E_NOT_LISTED"></a>

The item is not listed, so it cannot be purchased.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_NOT_LISTED">E_NOT_LISTED</a>: u64 = 5;
</code></pre>



<a name="0x2_kiosk_E_NOT_OWNER"></a>

The caller does not hold this kiosk's owner capability.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_NOT_OWNER">E_NOT_OWNER</a>: u64 = 0;
</code></pre>



<a name="0x2_kiosk_E_NO_POLICY"></a>

The type has no transferable policy, so a locked item could never be
sold again.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_NO_POLICY">E_NO_POLICY</a>: u64 = 10;
</code></pre>



<a name="0x2_kiosk_E_POLICY_HAS_RULES"></a>

<code>buy_unregulated</code> was called on a policy that has rules registered. The
entry boundary cannot discharge those rules, so the sale has to go
through a module that knows how.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_POLICY_HAS_RULES">E_POLICY_HAS_RULES</a>: u64 = 16;
</code></pre>



<a name="0x2_kiosk_E_PRICE_OUT_OF_RANGE"></a>

The asking price exceeds <code><a href="kiosk.md#0x2_kiosk_MAX_PRICE">MAX_PRICE</a></code>.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_PRICE_OUT_OF_RANGE">E_PRICE_OUT_OF_RANGE</a>: u64 = 9;
</code></pre>



<a name="0x2_kiosk_E_PRICE_ZERO"></a>

A listing must be free. <code>price == 0</code> is the internal "not listed"
marker, so a zero-price listing would read back as unlisted and could
never be bought — the seller would believe it was for sale while the
kiosk showed it as withdrawn.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_PRICE_ZERO">E_PRICE_ZERO</a>: u64 = 11;
</code></pre>



<a name="0x2_kiosk_E_WRONG_PURCHASE_CAP"></a>

The presented <code><a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a></code> is for a different kiosk or a different
item, so it grants no purchase right here.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_E_WRONG_PURCHASE_CAP">E_WRONG_PURCHASE_CAP</a>: u64 = 14;
</code></pre>



<a name="0x2_kiosk_MAX_ITEMS"></a>

Maximum items one kiosk may hold.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_MAX_ITEMS">MAX_ITEMS</a>: u64 = 10000;
</code></pre>



<a name="0x2_kiosk_MAX_PRICE"></a>

Upper bound on a listing price. Keeps a mistyped price from overflowing
downstream royalty arithmetic.


<pre><code><b>const</b> <a href="kiosk.md#0x2_kiosk_MAX_PRICE">MAX_PRICE</a>: u64 = 1000000000000;
</code></pre>



<a name="0x2_kiosk_new"></a>

## Function `new`

Create a kiosk and its owner capability.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_new">new</a>(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): (<a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, <a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_new">new</a>(ctx: &<b>mut</b> TxContext): (<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, <a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>) {
    <b>let</b> <a href="kiosk.md#0x2_kiosk">kiosk</a> = <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        items: <a href="vec_map.md#0x2_vec_map_empty">vec_map::empty</a>&lt;ID, <a href="kiosk.md#0x2_kiosk_Listing">Listing</a>&gt;(),
        profits: <a href="vec_map.md#0x2_vec_map_empty">vec_map::empty</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;()
    };
    <b>let</b> cap = <a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        <a href="kiosk.md#0x2_kiosk">kiosk</a>: <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id)
    };
    (<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap)
}
</code></pre>



</details>

<a name="0x2_kiosk_share"></a>

## Function `share`

Share the kiosk and hand the capability to its owner.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_share">share</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: <a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, recipient: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_share">share</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, cap: <a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>, recipient: <b>address</b>) {
    <a href="transfer.md#0x2_transfer_share_object">transfer::share_object</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>);
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(cap, recipient);
}
</code></pre>



</details>

<a name="0x2_kiosk_freeze_cap"></a>

## Function `freeze_cap`

Freeze the owner capability: items can still trade, but the listing can
never change again. This is how a creator permanently opens a collection
to trading without giving up curation.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_freeze_cap">freeze_cap</a>(cap: <a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_freeze_cap">freeze_cap</a>(cap: <a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>) {
    <a href="transfer.md#0x2_transfer_public_freeze_object">transfer::public_freeze_object</a>(cap);
}
</code></pre>



</details>

<a name="0x2_kiosk_id"></a>

## Function `id`

The kiosk's id.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_id">id</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>): <a href="object.md#0x2_object_ID">object::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_id">id</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>): ID {
    <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id)
}
</code></pre>



</details>

<a name="0x2_kiosk_owner_cap_kiosk"></a>

## Function `owner_cap_kiosk`

The kiosk an owner capability controls.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_owner_cap_kiosk">owner_cap_kiosk</a>(cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>): <a href="object.md#0x2_object_ID">object::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_owner_cap_kiosk">owner_cap_kiosk</a>(cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>): ID {
    cap.<a href="kiosk.md#0x2_kiosk">kiosk</a>
}
</code></pre>



</details>

<a name="0x2_kiosk_create"></a>

## Function `create`

Create a kiosk, share it, and hand the owner cap to <code>recipient</code>, in one
transaction.

<code>new</code> returns <code>(<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, <a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>)</code> and an entry function cannot
return values, so this is the shape a CLI can actually call. Calling
<code>new</code> then <code>share</code> from a transaction would leave a window where the
kiosk exists but nobody can list in it.


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_create">create</a>(recipient: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_create">create</a>(recipient: <b>address</b>, ctx: &<b>mut</b> TxContext) {
    <b>let</b> (<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap) = <a href="kiosk.md#0x2_kiosk_new">new</a>(ctx);
    <a href="kiosk.md#0x2_kiosk_share">share</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap, recipient);
}
</code></pre>



</details>

<a name="0x2_kiosk_place_item"></a>

## Function `place_item`

Put an item into the kiosk and discard the handle.

<code>place</code> returns an <code><a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a></code>, which an entry function may not do. The
handle is still recoverable — it is the <code>item_id</code> the caller passed —
so nothing is lost by dropping it here.


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_place_item">place_item</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, item_id: <a href="object.md#0x2_object_ID">object::ID</a>, item: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_place_item">place_item</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>,
    item_id: ID,
    item: T,
) {
    <b>let</b> handle = <a href="kiosk.md#0x2_kiosk_place">place</a>&lt;T&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap, item_id, item);
    <b>let</b> _ = handle;
}
</code></pre>



</details>

<a name="0x2_kiosk_lock_item"></a>

## Function `lock_item`

Put an item into the kiosk and lock it immediately, discarding the
handle. See <code>place_item</code> for why the return value is dropped.


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_lock_item">lock_item</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, item_id: <a href="object.md#0x2_object_ID">object::ID</a>, policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, item: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_lock_item">lock_item</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>,
    item_id: ID,
    policy: &TransferPolicy&lt;T&gt;,
    item: T,
) {
    <b>let</b> handle = <a href="kiosk.md#0x2_kiosk_lock">lock</a>&lt;T&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap, item_id, policy, item);
    <b>let</b> _ = handle;
}
</code></pre>



</details>

<a name="0x2_kiosk_list_item"></a>

## Function `list_item`

List the item at <code>item_id</code> for <code>price</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_list_item">list_item</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, item_id: <a href="object.md#0x2_object_ID">object::ID</a>, price: u64, ctx: &<a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_list_item">list_item</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>,
    item_id: ID,
    price: u64,
    ctx: &TxContext,
) {
    <b>let</b> handle = <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a> {
        <a href="kiosk.md#0x2_kiosk">kiosk</a>: <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id),
        item: item_id
    };
    <a href="kiosk.md#0x2_kiosk_list">list</a>&lt;T&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap, handle, price, ctx);
}
</code></pre>



</details>

<a name="0x2_kiosk_delist_item"></a>

## Function `delist_item`

Stop selling the item at <code>item_id</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_delist_item">delist_item</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, item_id: <a href="object.md#0x2_object_ID">object::ID</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_delist_item">delist_item</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>,
    item_id: ID,
) {
    <b>let</b> handle = <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a> {
        <a href="kiosk.md#0x2_kiosk">kiosk</a>: <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id),
        item: item_id
    };
    <a href="kiosk.md#0x2_kiosk_delist">delist</a>&lt;T&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap, handle);
}
</code></pre>



</details>

<a name="0x2_kiosk_take_to"></a>

## Function `take_to`

Take the item at <code>item_id</code> back out and send it to <code>recipient</code>.

<code>take</code> returns the item, which an entry function cannot do, and the
caller has to end up owning it anyway — so the transfer is folded in
rather than left dangling.


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_take_to">take_to</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, item_id: <a href="object.md#0x2_object_ID">object::ID</a>, recipient: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_take_to">take_to</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>,
    item_id: ID,
    recipient: <b>address</b>,
) {
    <b>let</b> handle = <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a> {
        <a href="kiosk.md#0x2_kiosk">kiosk</a>: <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id),
        item: item_id
    };
    <b>let</b> item = <a href="kiosk.md#0x2_kiosk_take">take</a>&lt;T&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap, handle);
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(item, recipient);
}
</code></pre>



</details>

<a name="0x2_kiosk_reserve_item"></a>

## Function `reserve_item`

Reserve the listing at <code>item_id</code> for whoever receives the returned cap.

Entry functions cannot return the <code><a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a></code>, so from a CLI this
reserves the listing without handing the cap anywhere — which makes it
unusable until <code>unreserve_to</code> is called. That is a real limitation of the
entry boundary, not a shortcut: the non-entry <code>reserve</code> returns the cap
and is what a composing module should call.


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_reserve_item">reserve_item</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, item_id: <a href="object.md#0x2_object_ID">object::ID</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_reserve_item">reserve_item</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>,
    item_id: ID,
    ctx: &<b>mut</b> TxContext,
) {
    <b>let</b> handle = <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a> {
        <a href="kiosk.md#0x2_kiosk">kiosk</a>: <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id),
        item: item_id
    };
    <b>let</b> purchase_cap = <a href="kiosk.md#0x2_kiosk_reserve">reserve</a>&lt;T&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap, handle, ctx);
    // Burn the cap so the listing is not left reserved against an <a href="object.md#0x2_object">object</a>
    // nobody can hold.
    <b>let</b> <a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a> { id, <a href="kiosk.md#0x2_kiosk">kiosk</a>: _, item: _ } = purchase_cap;
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
}
</code></pre>



</details>

<a name="0x2_kiosk_place"></a>

## Function `place`

Put an item into the kiosk. The owner may take it back at any time until
it is locked.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_place">place</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, item_id: <a href="object.md#0x2_object_ID">object::ID</a>, item: T): <a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_place">place</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>,
    item_id: ID,
    item: T,
): <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a> {
    <a href="kiosk.md#0x2_kiosk_assert_owner">assert_owner</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap);
    <a href="kiosk.md#0x2_kiosk_insert_item">insert_item</a>&lt;T&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>, item_id, item, <b>false</b>, 0, @0x0)
}
</code></pre>



</details>

<a name="0x2_kiosk_lock"></a>

## Function `lock`

Put an item into the kiosk and lock it immediately.

A locked item cannot be taken back by the owner; the only exit is
<code>purchase</code>, which creates a <code>TransferRequest&lt;T&gt;</code>. <code>policy</code> is required as
proof that such a request can be resolved — without one the item would be
unsellable forever.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_lock">lock</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, item_id: <a href="object.md#0x2_object_ID">object::ID</a>, policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, item: T): <a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_lock">lock</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>,
    item_id: ID,
    policy: &TransferPolicy&lt;T&gt;,
    item: T,
): <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a> {
    <a href="kiosk.md#0x2_kiosk_assert_owner">assert_owner</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap);
    // Proof that this item is sellable. `take` will refuse a locked item,
    // and `purchase` needs a `TransferPolicy&lt;T&gt;` <b>to</b> create the request it
    // returns; without one the item could never leave the <a href="kiosk.md#0x2_kiosk">kiosk</a> again.
    // Requiring a non-empty rule set also means the type owner <b>has</b>
    // actually configured how trades are resolved.
    <b>assert</b>!(<a href="transfer_policy.md#0x2_transfer_policy_rule_count">transfer_policy::rule_count</a>(policy) &gt; 0, <a href="kiosk.md#0x2_kiosk_E_NO_POLICY">E_NO_POLICY</a>);
    <a href="kiosk.md#0x2_kiosk_insert_item">insert_item</a>&lt;T&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>, item_id, item, <b>true</b>, 0, @0x0)
}
</code></pre>



</details>

<a name="0x2_kiosk_take"></a>

## Function `take`

Take an item back out. Refuses while the item is locked, which is the
guarantee a buyer relies on: a locked listing cannot be withdrawn.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_take">take</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, handle: <a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_take">take</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>,
    handle: <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>,
): T {
    <a href="kiosk.md#0x2_kiosk_assert_owner">assert_owner</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap);
    <b>let</b> listing = <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, &handle);
    <b>assert</b>!(!listing.locked, <a href="kiosk.md#0x2_kiosk_E_ITEM_LOCKED">E_ITEM_LOCKED</a>);
    <a href="kiosk.md#0x2_kiosk_remove_item">remove_item</a>&lt;T&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>, &handle)
}
</code></pre>



</details>

<a name="0x2_kiosk_list"></a>

## Function `list`

List an item at <code>price</code>, paid to the transaction sender. A locked item
stays locked; an unlocked one may still be taken back.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_list">list</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, handle: <a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>, price: u64, ctx: &<a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_list">list</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>,
    handle: <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>,
    price: u64,
    ctx: &TxContext,
) {
    <a href="kiosk.md#0x2_kiosk_assert_owner">assert_owner</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap);
    <b>assert</b>!(price &gt; 0, <a href="kiosk.md#0x2_kiosk_E_PRICE_ZERO">E_PRICE_ZERO</a>);
    <b>assert</b>!(<a href="kiosk.md#0x2_kiosk_price">price</a> &lt;= <a href="kiosk.md#0x2_kiosk_MAX_PRICE">MAX_PRICE</a>, <a href="kiosk.md#0x2_kiosk_E_PRICE_OUT_OF_RANGE">E_PRICE_OUT_OF_RANGE</a>);
    <b>assert</b>!(
        dof::exists_with_type&lt;ID, T&gt;(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id, handle.item),
        <a href="kiosk.md#0x2_kiosk_E_ITEM_TYPE_MISMATCH">E_ITEM_TYPE_MISMATCH</a>
    );
    <b>let</b> listing = <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, &handle);
    <b>assert</b>!(listing.price == 0, <a href="kiosk.md#0x2_kiosk_E_ALREADY_LISTED">E_ALREADY_LISTED</a>);
    <b>let</b> updated = <a href="kiosk.md#0x2_kiosk_Listing">Listing</a> {
        <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>: listing.<a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>,
        locked: listing.locked,
        price,
        seller: <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx),
        purchase_cap: listing.purchase_cap
    };
    <a href="kiosk.md#0x2_kiosk_set_listing">set_listing</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, handle.item, updated);
}
</code></pre>



</details>

<a name="0x2_kiosk_delist"></a>

## Function `delist`

Stop selling. The item stays in the kiosk and can be listed again.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_delist">delist</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, handle: <a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_delist">delist</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>,
    handle: <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>,
) {
    <a href="kiosk.md#0x2_kiosk_assert_owner">assert_owner</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap);
    <b>assert</b>!(
        dof::exists_with_type&lt;ID, T&gt;(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id, handle.item),
        <a href="kiosk.md#0x2_kiosk_E_ITEM_TYPE_MISMATCH">E_ITEM_TYPE_MISMATCH</a>
    );
    <b>let</b> listing = <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, &handle);
    <b>let</b> updated = <a href="kiosk.md#0x2_kiosk_Listing">Listing</a> {
        <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>: listing.<a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>,
        locked: listing.locked,
        price: 0,
        seller: @0x0,
        purchase_cap: listing.purchase_cap
    };
    <a href="kiosk.md#0x2_kiosk_set_listing">set_listing</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, handle.item, updated);
}
</code></pre>



</details>

<a name="0x2_kiosk_purchase"></a>

## Function `purchase`

Buy the item behind <code>handle</code>.

Returns the item plus a <code>TransferRequest&lt;T&gt;</code> hot potato that the caller
must resolve through the type's <code>TransferPolicy&lt;T&gt;</code> — paying any royalty
rule and so on. The kiosk deliberately does **not** resolve it: rules
belong to the type owner, not the seller.

The asking price is forwarded to the seller immediately; the rest of
<code>payment</code> stays with the buyer. This is the "simple sale" flow. A
marketplace that pays out in batches can use <code>record_profit</code> instead and
settle later.

Refuses when the listing is reserved (<code>exclusive</code>), which is what stops
the general path from stealing a sale that was promised to a specific
buyer. A reserved listing must be filled through <code>purchase_with_cap</code>.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_purchase">purchase</a>&lt;T: store, key, Asset: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, handle: <a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>, payment: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;Asset&gt;, from: <a href="object.md#0x2_object_ID">object::ID</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): (T, <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_purchase">purchase</a>&lt;T: key + store, Asset: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    policy: &TransferPolicy&lt;T&gt;,
    handle: <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>,
    payment: &<b>mut</b> Coin&lt;Asset&gt;,
    from: ID,
    ctx: &<b>mut</b> TxContext,
): (T, TransferRequest&lt;T&gt;) {
    <b>let</b> listing = <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, &handle);
    <b>assert</b>!(listing.price &gt; 0, <a href="kiosk.md#0x2_kiosk_E_NOT_LISTED">E_NOT_LISTED</a>);
    <b>assert</b>!(listing.<a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a> == <a href="kiosk.md#0x2_kiosk_type_name_bytes">type_name_bytes</a>&lt;T&gt;(), <a href="kiosk.md#0x2_kiosk_E_ITEM_TYPE_MISMATCH">E_ITEM_TYPE_MISMATCH</a>);
    // A reserved listing is not for sale <b>to</b> the <b>public</b>. Checking this here
    // rather than only in the cap path is what makes the reservation
    // binding: the default entry point <b>has</b> <b>to</b> refuse, not just be unused.
    <b>assert</b>!(
        listing.purchase_cap == <a href="object.md#0x2_object_id_from_address">object::id_from_address</a>(@0x0),
        <a href="kiosk.md#0x2_kiosk_E_LISTING_IS_EXCLUSIVE">E_LISTING_IS_EXCLUSIVE</a>
    );
    <a href="kiosk.md#0x2_kiosk_settle">settle</a>&lt;T, Asset&gt;(
        <a href="kiosk.md#0x2_kiosk">kiosk</a>,
        policy,
        handle,
        listing,
        payment,
        from,
        ctx
    )
}
</code></pre>



</details>

<a name="0x2_kiosk_reserve"></a>

## Function `reserve`

Reserve this listing for whoever receives the returned <code><a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a></code>.

Only the kiosk owner may reserve, and only one cap may be outstanding
per listing — a second reservation would silently invalidate the first
holder's right, so it is rejected rather than overwritten.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_reserve">reserve</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, handle: <a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="kiosk.md#0x2_kiosk_PurchaseCap">kiosk::PurchaseCap</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_reserve">reserve</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>,
    handle: <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>,
    ctx: &<b>mut</b> TxContext,
): <a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a> {
    <a href="kiosk.md#0x2_kiosk_assert_owner">assert_owner</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap);
    <b>assert</b>!(
        dof::exists_with_type&lt;ID, T&gt;(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id, handle.item),
        <a href="kiosk.md#0x2_kiosk_E_ITEM_TYPE_MISMATCH">E_ITEM_TYPE_MISMATCH</a>
    );
    <b>let</b> listing = <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, &handle);
    <b>assert</b>!(
        listing.purchase_cap == <a href="object.md#0x2_object_id_from_address">object::id_from_address</a>(@0x0),
        <a href="kiosk.md#0x2_kiosk_E_ALREADY_EXCLUSIVE">E_ALREADY_EXCLUSIVE</a>
    );

    <b>let</b> purchase_cap = <a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        <a href="kiosk.md#0x2_kiosk">kiosk</a>: <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id),
        item: handle.item
    };
    <b>let</b> updated = <a href="kiosk.md#0x2_kiosk_Listing">Listing</a> {
        <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>: listing.<a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>,
        locked: listing.locked,
        price: listing.price,
        seller: listing.seller,
        purchase_cap: <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&purchase_cap.id)
    };
    <a href="kiosk.md#0x2_kiosk_set_listing">set_listing</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, handle.item, updated);
    purchase_cap
}
</code></pre>



</details>

<a name="0x2_kiosk_unreserve"></a>

## Function `unreserve`

Withdraw a reservation. The cap is handed back so the owner can destroy,
freeze, or re-issue it.

The cap is taken as an argument rather than reconstructed from the
listing's recorded id: a <code>UID</code> cannot be rebuilt from an address without
forging object identity, and taking the real one proves the caller holds
the cap rather than merely knows its id.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_unreserve">unreserve</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>, handle: <a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>, purchase_cap: <a href="kiosk.md#0x2_kiosk_PurchaseCap">kiosk::PurchaseCap</a>): <a href="kiosk.md#0x2_kiosk_PurchaseCap">kiosk::PurchaseCap</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_unreserve">unreserve</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>,
    handle: <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>,
    purchase_cap: <a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a>,
): <a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a> {
    <a href="kiosk.md#0x2_kiosk_assert_owner">assert_owner</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, cap);
    <b>assert</b>!(
        dof::exists_with_type&lt;ID, T&gt;(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id, handle.item),
        <a href="kiosk.md#0x2_kiosk_E_ITEM_TYPE_MISMATCH">E_ITEM_TYPE_MISMATCH</a>
    );
    <b>let</b> listing = <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, &handle);
    <b>assert</b>!(listing.purchase_cap != <a href="object.md#0x2_object_id_from_address">object::id_from_address</a>(@0x0), <a href="kiosk.md#0x2_kiosk_E_NOT_EXCLUSIVE">E_NOT_EXCLUSIVE</a>);
    <b>assert</b>!(purchase_cap.item == handle.item, <a href="kiosk.md#0x2_kiosk_E_WRONG_PURCHASE_CAP">E_WRONG_PURCHASE_CAP</a>);
    <b>assert</b>!(
        <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&purchase_cap.id) == listing.purchase_cap,
        <a href="kiosk.md#0x2_kiosk_E_WRONG_PURCHASE_CAP">E_WRONG_PURCHASE_CAP</a>
    );
    <b>let</b> updated = <a href="kiosk.md#0x2_kiosk_Listing">Listing</a> {
        <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>: listing.<a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>,
        locked: listing.locked,
        price: listing.price,
        seller: listing.seller,
        purchase_cap: <a href="object.md#0x2_object_id_from_address">object::id_from_address</a>(@0x0)
    };
    <a href="kiosk.md#0x2_kiosk_set_listing">set_listing</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, handle.item, updated);
    purchase_cap
}
</code></pre>



</details>

<a name="0x2_kiosk_purchase_with_cap"></a>

## Function `purchase_with_cap`

Buy a reserved listing. <code>purchase_cap</code> is consumed, so one cap buys
exactly one item and cannot be replayed.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_purchase_with_cap">purchase_with_cap</a>&lt;T: store, key, Asset: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, handle: <a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>, purchase_cap: <a href="kiosk.md#0x2_kiosk_PurchaseCap">kiosk::PurchaseCap</a>, payment: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;Asset&gt;, from: <a href="object.md#0x2_object_ID">object::ID</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): (T, <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_purchase_with_cap">purchase_with_cap</a>&lt;T: key + store, Asset: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    policy: &TransferPolicy&lt;T&gt;,
    handle: <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>,
    purchase_cap: <a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a>,
    payment: &<b>mut</b> Coin&lt;Asset&gt;,
    from: ID,
    ctx: &<b>mut</b> TxContext,
): (T, TransferRequest&lt;T&gt;) {
    <b>let</b> listing = <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, &handle);
    <b>assert</b>!(listing.price &gt; 0, <a href="kiosk.md#0x2_kiosk_E_NOT_LISTED">E_NOT_LISTED</a>);
    <b>assert</b>!(listing.<a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a> == <a href="kiosk.md#0x2_kiosk_type_name_bytes">type_name_bytes</a>&lt;T&gt;(), <a href="kiosk.md#0x2_kiosk_E_ITEM_TYPE_MISMATCH">E_ITEM_TYPE_MISMATCH</a>);
    <b>assert</b>!(listing.purchase_cap != <a href="object.md#0x2_object_id_from_address">object::id_from_address</a>(@0x0), <a href="kiosk.md#0x2_kiosk_E_NOT_EXCLUSIVE">E_NOT_EXCLUSIVE</a>);
    // Three independent bindings, so a cap cannot be aimed at another
    // listing: it must be this <a href="kiosk.md#0x2_kiosk">kiosk</a>, this item, and the one the listing
    // actually names.
    <b>assert</b>!(
        purchase_cap.<a href="kiosk.md#0x2_kiosk">kiosk</a> == <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id),
        <a href="kiosk.md#0x2_kiosk_E_WRONG_PURCHASE_CAP">E_WRONG_PURCHASE_CAP</a>
    );
    <b>assert</b>!(purchase_cap.item == handle.item, <a href="kiosk.md#0x2_kiosk_E_WRONG_PURCHASE_CAP">E_WRONG_PURCHASE_CAP</a>);
    <b>assert</b>!(
        <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&purchase_cap.id) == listing.purchase_cap,
        <a href="kiosk.md#0x2_kiosk_E_WRONG_PURCHASE_CAP">E_WRONG_PURCHASE_CAP</a>
    );

    // `UID` <b>has</b> no `<b>copy</b>`, so the cap is destructured <b>to</b> burn it. Burning is
    // what makes the right single-<b>use</b>: a cap cannot be held back and
    // replayed against a listing the owner later creates.
    <b>let</b> <a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a> { id: cap_uid, <a href="kiosk.md#0x2_kiosk">kiosk</a>: _k, item: _i } = purchase_cap;
    <a href="object.md#0x2_object_delete">object::delete</a>(cap_uid);
    <a href="kiosk.md#0x2_kiosk_settle">settle</a>&lt;T, Asset&gt;(
        <a href="kiosk.md#0x2_kiosk">kiosk</a>,
        policy,
        handle,
        listing,
        payment,
        from,
        ctx
    )
}
</code></pre>



</details>

<a name="0x2_kiosk_is_exclusive"></a>

## Function `is_exclusive`

True iff this listing is reserved and so closed to <code>purchase</code>.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_is_exclusive">is_exclusive</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_is_exclusive">is_exclusive</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>): bool {
    <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, handle).purchase_cap != <a href="object.md#0x2_object_id_from_address">object::id_from_address</a>(@0x0)
}
</code></pre>



</details>

<a name="0x2_kiosk_buy_unregulated"></a>

## Function `buy_unregulated`

Buy the item at <code>item_id</code> and send it to <code>recipient</code>, requiring that the
policy has **no** rules.

The plain <code>purchase</code> returns a <code>TransferRequest&lt;T&gt;</code> hot potato, and an
entry function can neither return one nor leave it unresolved. This
wrapper exists for the only case where that is honestly possible: a
policy with zero rules, so <code>confirm_request</code> has nothing to wait for.

**This is not a general sale path.** A type with a royalty rule cannot be
sold through here — deliberately. Resolving the request means paying
that type's rules, and those belong to the type owner, not to the
marketplace. A type with rules must be sold through <code>kanari_pet::trade</code>
or another module that knows how to discharge the hot potato.


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_buy_unregulated">buy_unregulated</a>&lt;T: store, key, Asset: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, item_id: <a href="object.md#0x2_object_ID">object::ID</a>, payment: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;Asset&gt;, recipient: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kiosk.md#0x2_kiosk_buy_unregulated">buy_unregulated</a>&lt;T: key + store, Asset: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    policy: &TransferPolicy&lt;T&gt;,
    item_id: ID,
    payment: &<b>mut</b> Coin&lt;Asset&gt;,
    recipient: <b>address</b>,
    ctx: &<b>mut</b> TxContext,
) {
    <b>assert</b>!(<a href="transfer_policy.md#0x2_transfer_policy_rule_count">transfer_policy::rule_count</a>(policy) == 0, <a href="kiosk.md#0x2_kiosk_E_POLICY_HAS_RULES">E_POLICY_HAS_RULES</a>);
    <b>let</b> handle = <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a> {
        <a href="kiosk.md#0x2_kiosk">kiosk</a>: <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id),
        item: item_id
    };
    <b>let</b> from = <a href="object.md#0x2_object_id_from_address">object::id_from_address</a>(<a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx));
    <b>let</b> (item, request) = <a href="kiosk.md#0x2_kiosk_purchase">purchase</a>&lt;T, Asset&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>, policy, handle, payment, from, ctx);
    <b>let</b> (_i, _p, _f) = <a href="transfer_policy.md#0x2_transfer_policy_confirm_request">transfer_policy::confirm_request</a>&lt;T&gt;(policy, request);
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(item, recipient);
}
</code></pre>



</details>

<a name="0x2_kiosk_purchase_cap_item"></a>

## Function `purchase_cap_item`

The item a purchase cap is reserved for.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_purchase_cap_item">purchase_cap_item</a>(cap: &<a href="kiosk.md#0x2_kiosk_PurchaseCap">kiosk::PurchaseCap</a>): <a href="object.md#0x2_object_ID">object::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_purchase_cap_item">purchase_cap_item</a>(cap: &<a href="kiosk.md#0x2_kiosk_PurchaseCap">PurchaseCap</a>): ID {
    cap.item
}
</code></pre>



</details>

<a name="0x2_kiosk_settle"></a>

## Function `settle`

Shared settlement tail for both purchase paths.

<code>listing</code> is the already-validated state, so the assertions that decide
*whether* a sale may happen live in the callers and cannot drift between
the public and reserved routes. What remains — clear the listing, pay the
seller, hand over the item, mint the request — is identical for both.


<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_settle">settle</a>&lt;T: store, key, Asset: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, handle: <a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>, listing: <a href="kiosk.md#0x2_kiosk_Listing">kiosk::Listing</a>, payment: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;Asset&gt;, from: <a href="object.md#0x2_object_ID">object::ID</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): (T, <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_settle">settle</a>&lt;T: key + store, Asset: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    policy: &TransferPolicy&lt;T&gt;,
    handle: <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>,
    listing: <a href="kiosk.md#0x2_kiosk_Listing">Listing</a>,
    payment: &<b>mut</b> Coin&lt;Asset&gt;,
    from: ID,
    ctx: &<b>mut</b> TxContext,
): (T, TransferRequest&lt;T&gt;) {
    <b>let</b> price = listing.price;
    <b>assert</b>!(<a href="coin.md#0x2_coin_value">coin::value</a>(payment) &gt;= price, <a href="kiosk.md#0x2_kiosk_E_INSUFFICIENT_PAYMENT">E_INSUFFICIENT_PAYMENT</a>);

    // Clear the listing before handing the item over, so a second purchase
    // of the same handle finds nothing listed. The reservation goes <b>with</b>
    // it: the cap is spent, so leaving it set would block every later
    // listing of the same handle forever.
    <b>let</b> cleared = <a href="kiosk.md#0x2_kiosk_Listing">Listing</a> {
        <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>: listing.<a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>,
        locked: listing.locked,
        price: 0,
        seller: @0x0,
        purchase_cap: <a href="object.md#0x2_object_id_from_address">object::id_from_address</a>(@0x0)
    };
    <a href="kiosk.md#0x2_kiosk_set_listing">set_listing</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, handle.item, cleared);

    <b>let</b> seller = listing.seller;
    <b>let</b> payout = <a href="coin.md#0x2_coin_split">coin::split</a>(payment, price, ctx);
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(payout, seller);

    <b>let</b> item = <a href="kiosk.md#0x2_kiosk_remove_item">remove_item</a>&lt;T&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>, &handle);
    <b>let</b> request = <a href="transfer_policy.md#0x2_transfer_policy_new_request">transfer_policy::new_request</a>(policy, handle.item, price, from);
    (item, request)
}
</code></pre>



</details>

<a name="0x2_kiosk_record_profit"></a>

## Function `record_profit`

Note that <code>amount</code> of <code>currency</code> is owed, for marketplaces that batch
payouts rather than paying inline.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_record_profit">record_profit</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, currency: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_record_profit">record_profit</a>(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    currency: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    amount: u64,
) {
    <b>let</b> current = <a href="kiosk.md#0x2_kiosk_profit_of">profit_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, &currency);
    <a href="kiosk.md#0x2_kiosk_set_profit">set_profit</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, currency, current + amount);
}
</code></pre>



</details>

<a name="0x2_kiosk_profit_of"></a>

## Function `profit_of`

Total owed in <code>currency</code>.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_profit_of">profit_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, currency: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_profit_of">profit_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, currency: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64 {
    <b>if</b> (<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.profits, currency)) {
        *<a href="vec_map.md#0x2_vec_map_get">vec_map::get</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.profits, currency)
    } <b>else</b> { 0 }
}
</code></pre>



</details>

<a name="0x2_kiosk_has_item"></a>

## Function `has_item`

True iff <code>handle</code> names a live item in <code><a href="kiosk.md#0x2_kiosk">kiosk</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_has_item">has_item</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_has_item">has_item</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>): bool {
    <a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.items, &handle.item)
}
</code></pre>



</details>

<a name="0x2_kiosk_is_locked"></a>

## Function `is_locked`

True iff the item is locked (the owner cannot take it back).


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_is_locked">is_locked</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_is_locked">is_locked</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>): bool {
    <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, handle).locked
}
</code></pre>



</details>

<a name="0x2_kiosk_is_listed"></a>

## Function `is_listed`

True iff the item is currently for sale.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_is_listed">is_listed</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_is_listed">is_listed</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>): bool {
    <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, handle).price &gt; 0
}
</code></pre>



</details>

<a name="0x2_kiosk_price"></a>

## Function `price`

The asking price, or 0.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_price">price</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_price">price</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>): u64 {
    <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, handle).price
}
</code></pre>



</details>

<a name="0x2_kiosk_seller"></a>

## Function `seller`

The address that receives payment for this listing.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_seller">seller</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_seller">seller</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>): <b>address</b> {
    <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, handle).seller
}
</code></pre>



</details>

<a name="0x2_kiosk_item_count"></a>

## Function `item_count`

How many items the kiosk holds.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_item_count">item_count</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_item_count">item_count</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>): u64 {
    <a href="vec_map.md#0x2_vec_map_size">vec_map::size</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.items)
}
</code></pre>



</details>

<a name="0x2_kiosk_item_ids"></a>

## Function `item_ids`

Ids of every item in the kiosk, for indexing.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_item_ids">item_ids</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="object.md#0x2_object_ID">object::ID</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk.md#0x2_kiosk_item_ids">item_ids</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;ID&gt; {
    <a href="vec_map.md#0x2_vec_map_keys">vec_map::keys</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.items)
}
</code></pre>



</details>

<a name="0x2_kiosk_assert_owner"></a>

## Function `assert_owner`



<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_assert_owner">assert_owner</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">kiosk::KioskOwnerCap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_assert_owner">assert_owner</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, cap: &<a href="kiosk.md#0x2_kiosk_KioskOwnerCap">KioskOwnerCap</a>) {
    <b>assert</b>!(cap.<a href="kiosk.md#0x2_kiosk">kiosk</a> == <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id), <a href="kiosk.md#0x2_kiosk_E_NOT_OWNER">E_NOT_OWNER</a>);
}
</code></pre>



</details>

<a name="0x2_kiosk_insert_item"></a>

## Function `insert_item`

Store <code>item</code> under <code>item_id</code> and record its listing.

The caller supplies <code>item_id</code> because Move cannot read a generic <code>T</code>'s
fields to extract its own id. That is also what makes the check
meaningful: the caller must name the id it actually holds, and <code>dof</code>
derives the child address from it.


<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_insert_item">insert_item</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, item_id: <a href="object.md#0x2_object_ID">object::ID</a>, item: T, locked: bool, price: u64, seller: <b>address</b>): <a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_insert_item">insert_item</a>&lt;T: key + store&gt;(
    <a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>,
    item_id: ID,
    item: T,
    locked: bool,
    price: u64,
    seller: <b>address</b>,
): <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a> {
    <b>let</b> count = <a href="vec_map.md#0x2_vec_map_size">vec_map::size</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.items);
    <b>assert</b>!(count &lt; <a href="kiosk.md#0x2_kiosk_MAX_ITEMS">MAX_ITEMS</a>, <a href="kiosk.md#0x2_kiosk_E_KIOSK_FULL">E_KIOSK_FULL</a>);
    <b>assert</b>!(
        !dof::exists_with_type&lt;ID, T&gt;(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id, item_id),
        <a href="kiosk.md#0x2_kiosk_E_ITEM_EXISTS">E_ITEM_EXISTS</a>
    );

    <b>let</b> listing = <a href="kiosk.md#0x2_kiosk_Listing">Listing</a> {
        <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>: <a href="kiosk.md#0x2_kiosk_type_name_bytes">type_name_bytes</a>&lt;T&gt;(),
        locked,
        price,
        seller,
        purchase_cap: <a href="object.md#0x2_object_id_from_address">object::id_from_address</a>(@0x0)
    };
    dof::add&lt;ID, T&gt;(&<b>mut</b> <a href="kiosk.md#0x2_kiosk">kiosk</a>.id, item_id, item);
    <a href="kiosk.md#0x2_kiosk_set_listing">set_listing</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>, item_id, listing);
    <a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a> {
        <a href="kiosk.md#0x2_kiosk">kiosk</a>: <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id),
        item: item_id
    }
}
</code></pre>



</details>

<a name="0x2_kiosk_remove_item"></a>

## Function `remove_item`

Pull the item back out and drop its listing entry.


<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_remove_item">remove_item</a>&lt;T: store, key&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_remove_item">remove_item</a>&lt;T: key + store&gt;(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>): T {
    <b>assert</b>!(
        dof::exists_with_type&lt;ID, T&gt;(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.id, handle.item),
        <a href="kiosk.md#0x2_kiosk_E_ITEM_TYPE_MISMATCH">E_ITEM_TYPE_MISMATCH</a>
    );
    <b>let</b> (removed_key, removed) = <a href="vec_map.md#0x2_vec_map_remove">vec_map::remove</a>(&<b>mut</b> <a href="kiosk.md#0x2_kiosk">kiosk</a>.items, &handle.item);
    <b>let</b> _ = removed_key;
    <b>let</b> _ = removed;
    dof::remove&lt;ID, T&gt;(&<b>mut</b> <a href="kiosk.md#0x2_kiosk">kiosk</a>.id, handle.item)
}
</code></pre>



</details>

<a name="0x2_kiosk_listing_of"></a>

## Function `listing_of`



<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">kiosk::ItemId</a>): <a href="kiosk.md#0x2_kiosk_Listing">kiosk::Listing</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_listing_of">listing_of</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, handle: &<a href="kiosk.md#0x2_kiosk_ItemId">ItemId</a>): <a href="kiosk.md#0x2_kiosk_Listing">Listing</a> {
    <b>assert</b>!(<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.items, &handle.item), <a href="kiosk.md#0x2_kiosk_E_ITEM_MISSING">E_ITEM_MISSING</a>);
    *<a href="vec_map.md#0x2_vec_map_get">vec_map::get</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.items, &handle.item)
}
</code></pre>



</details>

<a name="0x2_kiosk_set_listing"></a>

## Function `set_listing`

<code><a href="vec_map.md#0x2_vec_map_insert">vec_map::insert</a></code> aborts on a duplicate key, so a replace is
remove-then-insert.


<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_set_listing">set_listing</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, item_id: <a href="object.md#0x2_object_ID">object::ID</a>, listing: <a href="kiosk.md#0x2_kiosk_Listing">kiosk::Listing</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_set_listing">set_listing</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, item_id: ID, listing: <a href="kiosk.md#0x2_kiosk_Listing">Listing</a>) {
    <b>if</b> (<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.items, &item_id)) {
        <b>let</b> (removed_key, removed) = <a href="vec_map.md#0x2_vec_map_remove">vec_map::remove</a>(&<b>mut</b> <a href="kiosk.md#0x2_kiosk">kiosk</a>.items, &item_id);
        <b>let</b> _ = removed_key;
        <b>let</b> _ = removed;
    };
    <a href="vec_map.md#0x2_vec_map_insert">vec_map::insert</a>(&<b>mut</b> <a href="kiosk.md#0x2_kiosk">kiosk</a>.items, item_id, listing);
}
</code></pre>



</details>

<a name="0x2_kiosk_set_profit"></a>

## Function `set_profit`



<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_set_profit">set_profit</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">kiosk::Kiosk</a>, currency: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_set_profit">set_profit</a>(<a href="kiosk.md#0x2_kiosk">kiosk</a>: &<b>mut</b> <a href="kiosk.md#0x2_kiosk_Kiosk">Kiosk</a>, currency: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, amount: u64) {
    <b>if</b> (<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&<a href="kiosk.md#0x2_kiosk">kiosk</a>.profits, &currency)) {
        <b>let</b> (removed_key, removed) = <a href="vec_map.md#0x2_vec_map_remove">vec_map::remove</a>(&<b>mut</b> <a href="kiosk.md#0x2_kiosk">kiosk</a>.profits, &currency);
        <b>let</b> _ = removed_key;
        <b>let</b> _ = removed;
    };
    <a href="vec_map.md#0x2_vec_map_insert">vec_map::insert</a>(&<b>mut</b> <a href="kiosk.md#0x2_kiosk">kiosk</a>.profits, currency, amount);
}
</code></pre>



</details>

<a name="0x2_kiosk_type_name_bytes"></a>

## Function `type_name_bytes`

<code>T</code>'s canonical type-name bytes.


<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_type_name_bytes">type_name_bytes</a>&lt;T: store, key&gt;(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="kiosk.md#0x2_kiosk_type_name_bytes">type_name_bytes</a>&lt;T: key + store&gt;(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> name = std::type_name::get&lt;T&gt;();
    *std::ascii::as_bytes(std::type_name::borrow_string(&name))
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
