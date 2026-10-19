
<a name="0x2_deny_list"></a>

# Module `0x2::deny_list`

Sui-style deny list: a shared registry object that restricts addresses
from using instances of regulated coin types.

Deny entries live in a single shared <code><a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a></code> object (one per network),
keyed by coin type, so they persist across transactions and are visible to
every participant. Writes are authorized by a <code><a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a>&lt;T&gt;</code> minted by
<code><a href="coin.md#0x2_coin_create_regulated_currency">coin::create_regulated_currency</a></code>; membership reads are open to everyone.

Regulated transfer paths (<code><a href="pay.md#0x2_pay_split_and_transfer_checked">pay::split_and_transfer_checked</a></code>,
<code><a href="pay.md#0x2_pay_join_vec_and_transfer_checked">pay::join_vec_and_transfer_checked</a></code>, <code><a href="multisig.md#0x2_multisig_execute_transfer_checked">multisig::execute_transfer_checked</a></code>)
consult this list via <code><a href="coin.md#0x2_coin_deny_list_contains">coin::deny_list_contains</a></code>.


-  [Resource `DenyList`](#0x2_deny_list_DenyList)
-  [Resource `PerTypeList`](#0x2_deny_list_PerTypeList)
-  [Resource `TypeDenySet`](#0x2_deny_list_TypeDenySet)
-  [Struct `DenyKey`](#0x2_deny_list_DenyKey)
-  [Resource `DenyCap`](#0x2_deny_list_DenyCap)
-  [Constants](#@Constants_0)
-  [Function `deny_list_object_id`](#0x2_deny_list_deny_list_object_id)
-  [Function `borrow_singleton`](#0x2_deny_list_borrow_singleton)
-  [Function `coin_index`](#0x2_deny_list_coin_index)
-  [Function `is_reserved`](#0x2_deny_list_is_reserved)
-  [Function `create`](#0x2_deny_list_create)
-  [Function `per_type_list`](#0x2_deny_list_per_type_list)
-  [Function `new_denycap`](#0x2_deny_list_new_denycap)
-  [Function `v1_add`](#0x2_deny_list_v1_add)
-  [Function `v1_remove`](#0x2_deny_list_v1_remove)
-  [Function `v1_contains`](#0x2_deny_list_v1_contains)
-  [Function `v2_enable_global_pause`](#0x2_deny_list_v2_enable_global_pause)
-  [Function `v2_disable_global_pause`](#0x2_deny_list_v2_disable_global_pause)
-  [Function `v2_is_global_pause_enabled`](#0x2_deny_list_v2_is_global_pause_enabled)


<pre><code><b>use</b> <a href="bag.md#0x2_bag">0x2::bag</a>;
<b>use</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field">0x2::dynamic_object_field</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="table.md#0x2_table">0x2::table</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
<b>use</b> <a href="vec_set.md#0x2_vec_set">0x2::vec_set</a>;
</code></pre>



<a name="0x2_deny_list_DenyList"></a>

## Resource `DenyList`

A shared object holding every per-type deny list. Like Sui, it uses
both stores: the <code>lists</code> bag holds one <code><a href="deny_list.md#0x2_deny_list_PerTypeList">PerTypeList</a></code> per object
family (fast <code>denied_count</code> skip table), while each per-type address
set lives as a dynamic object field on this object's id, keyed by
<code>(per_type_index, per_type_key)</code>.


<pre><code><b>struct</b> <a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a> <b>has</b> store, key
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
<code>lists: <a href="bag.md#0x2_bag_Bag">bag::Bag</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_deny_list_PerTypeList"></a>

## Resource `PerTypeList`

Deny data for one object family: counts of how many types each
address is denied for. Lets <code>contains</code> skip the set lookup for the
overwhelmingly common undenied case.


<pre><code><b>struct</b> <a href="deny_list.md#0x2_deny_list_PerTypeList">PerTypeList</a> <b>has</b> store, key
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
<code>denied_count: <a href="table.md#0x2_table_Table">table::Table</a>&lt;<b>address</b>, u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_deny_list_TypeDenySet"></a>

## Resource `TypeDenySet`

The address set denied for one <code>(per_type_index, per_type_key)</code>,
e.g. <code>0x2::my_coin::MY_COIN</code>, plus that type's pause flag. Stored
as a dynamic object field so each type's set stays a live object in
storage. There is exactly one key type for these objects (<code><a href="deny_list.md#0x2_deny_list_DenyKey">DenyKey</a></code>):
this runtime keys dynamic fields by name *bytes*, so two key types
with the same layout would silently collide.


<pre><code><b>struct</b> <a href="deny_list.md#0x2_deny_list_TypeDenySet">TypeDenySet</a> <b>has</b> store, key
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
<code>addresses: <a href="vec_set.md#0x2_vec_set_VecSet">vec_set::VecSet</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>paused: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_deny_list_DenyKey"></a>

## Struct `DenyKey`

Dynamic-object-field key for a per-type set: essentially a
<code>(per_type_index, per_type_key)</code> pair.


<pre><code><b>struct</b> <a href="deny_list.md#0x2_deny_list_DenyKey">DenyKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>per_type_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>per_type_key: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_deny_list_DenyCap"></a>

## Resource `DenyCap`

Capability minted by <code><a href="coin.md#0x2_coin_create_regulated_currency">coin::create_regulated_currency</a></code>. Possession of
<code>&<a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a>&lt;T&gt;</code> authorizes writes to <code>T</code>'s deny entries.


<pre><code><b>struct</b> <a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a>&lt;T&gt; <b>has</b> drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="object.md#0x2_object_UID">object::UID</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_deny_list_COIN_INDEX"></a>

Index into the deny-list bag for <code><a href="coin.md#0x2_coin_Coin">coin::Coin</a></code> entries.


<pre><code><b>const</b> <a href="deny_list.md#0x2_deny_list_COIN_INDEX">COIN_INDEX</a>: u64 = 0;
</code></pre>



<a name="0x2_deny_list_EInvalidAddress"></a>

Reserved system addresses can never be denied.


<pre><code><b>const</b> <a href="deny_list.md#0x2_deny_list_EInvalidAddress">EInvalidAddress</a>: u64 = 2;
</code></pre>



<a name="0x2_deny_list_ENotDenied"></a>

The address being removed is not on the list.


<pre><code><b>const</b> <a href="deny_list.md#0x2_deny_list_ENotDenied">ENotDenied</a>: u64 = 1;
</code></pre>



<a name="0x2_deny_list_ENotSystemAddress"></a>

<code>create</code> was not called by the system address.


<pre><code><b>const</b> <a href="deny_list.md#0x2_deny_list_ENotSystemAddress">ENotSystemAddress</a>: u64 = 0;
</code></pre>



<a name="0x2_deny_list_deny_list_object_id"></a>

## Function `deny_list_object_id`

Well-known object id of the shared deny-list singleton, created at
genesis by the runtime (Sui uses <code>0x403</code> for the same purpose).
Transfer-time enforcement consults the list at this address.


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_deny_list_object_id">deny_list_object_id</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_deny_list_object_id">deny_list_object_id</a>(): <b>address</b> {
    @0x403
}
</code></pre>



</details>

<a name="0x2_deny_list_borrow_singleton"></a>

## Function `borrow_singleton`

Borrow the shared singleton without receiving it as a transaction
argument. The runtime preloads <code>@0x403</code> for every execution, so entry
functions stay free of deny-list parameters. Aborts when the
singleton is absent (e.g. the bare Move unit runner, which has no
genesis state): unit tests should build a local list with
<code>new_for_testing</code> and pass it explicitly instead.


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_borrow_singleton">borrow_singleton</a>(): &<a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_borrow_singleton">borrow_singleton</a>(): &<a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a> {
    <a href="object.md#0x2_object_borrow_global">object::borrow_global</a>&lt;<a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a>&gt;(<a href="deny_list.md#0x2_deny_list_deny_list_object_id">deny_list_object_id</a>())
}
</code></pre>



</details>

<a name="0x2_deny_list_coin_index"></a>

## Function `coin_index`

Bag index reserved for coin deny entries (used by <code><a href="coin.md#0x2_coin">coin</a>.<b>move</b></code>).


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_coin_index">coin_index</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_coin_index">coin_index</a>(): u64 {
    <a href="deny_list.md#0x2_deny_list_COIN_INDEX">COIN_INDEX</a>
}
</code></pre>



</details>

<a name="0x2_deny_list_is_reserved"></a>

## Function `is_reserved`

Framework/system addresses (<code>@0x0</code>–<code>@0xF</code>) can never be denied.


<pre><code><b>fun</b> <a href="deny_list.md#0x2_deny_list_is_reserved">is_reserved</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="deny_list.md#0x2_deny_list_is_reserved">is_reserved</a>(addr: <b>address</b>): bool {
    addr == @0x0
        || addr == @0x1
        || addr == @0x2
        || addr == @0x3
        || addr == @0x4
        || addr == @0x5
        || addr == @0x6
        || addr == @0x7
        || addr == @0x8
        || addr == @0x9
        || addr == @0xA
        || addr == @0xB
        || addr == @0xC
        || addr == @0xD
        || addr == @0xE
        || addr == @0xF
}
</code></pre>



</details>

<a name="0x2_deny_list_create"></a>

## Function `create`

Genesis-equivalent constructor for unit tests (dummy ctx sender is
<code>@0x0</code>, so the system-address gate passes). Production networks get
the singleton from runtime genesis at <code><a href="deny_list.md#0x2_deny_list_deny_list_object_id">deny_list_object_id</a>()</code>.


<pre><code><b>fun</b> <a href="deny_list.md#0x2_deny_list_create">create</a>(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="deny_list.md#0x2_deny_list_create">create</a>(ctx: &<b>mut</b> TxContext) {
    <b>assert</b>!(<a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx) == @0x0, <a href="deny_list.md#0x2_deny_list_ENotSystemAddress">ENotSystemAddress</a>);
    <b>let</b> lists = <a href="bag.md#0x2_bag_new">bag::new</a>(ctx);
    <a href="bag.md#0x2_bag_add">bag::add</a>(&<b>mut</b> lists, <a href="deny_list.md#0x2_deny_list_COIN_INDEX">COIN_INDEX</a>, <a href="deny_list.md#0x2_deny_list_per_type_list">per_type_list</a>(ctx));
    <a href="transfer.md#0x2_transfer_share_object">transfer::share_object</a>(<a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a> { id: <a href="object.md#0x2_object_new">object::new</a>(ctx), lists });
}
</code></pre>



</details>

<a name="0x2_deny_list_per_type_list"></a>

## Function `per_type_list`



<pre><code><b>fun</b> <a href="deny_list.md#0x2_deny_list_per_type_list">per_type_list</a>(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="deny_list.md#0x2_deny_list_PerTypeList">deny_list::PerTypeList</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="deny_list.md#0x2_deny_list_per_type_list">per_type_list</a>(ctx: &<b>mut</b> TxContext): <a href="deny_list.md#0x2_deny_list_PerTypeList">PerTypeList</a> {
    <a href="deny_list.md#0x2_deny_list_PerTypeList">PerTypeList</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        denied_count: <a href="table.md#0x2_table_new">table::new</a>(ctx),
    }
}
</code></pre>



</details>

<a name="0x2_deny_list_new_denycap"></a>

## Function `new_denycap`

Create a <code><a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a></code> while constructing a regulated currency.

Friend-only on purpose: every write path requires <code>&<a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a>&lt;T&gt;</code>, so a
publicly mintable capability would let any published package freeze
arbitrary addresses for arbitrary coin types and bypass every check in
<code><a href="coin.md#0x2_coin">coin</a></code>. <code><a href="coin.md#0x2_coin_create_regulated_currency">coin::create_regulated_currency</a></code> is the only minting path.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="deny_list.md#0x2_deny_list_new_denycap">new_denycap</a>&lt;T&gt;(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="deny_list.md#0x2_deny_list_DenyCap">deny_list::DenyCap</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="deny_list.md#0x2_deny_list_new_denycap">new_denycap</a>&lt;T&gt;(ctx: &<b>mut</b> TxContext): <a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a>&lt;T&gt; {
    <a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a>&lt;T&gt; { id: <a href="object.md#0x2_object_new">object::new</a>(ctx) }
}
</code></pre>



</details>

<a name="0x2_deny_list_v1_add"></a>

## Function `v1_add`

Add <code>addr</code> to the deny set for <code>type</code> under <code>per_type_index</code>. The
per-type set object is created on first use.

Friend-only: callers reach this through <code><a href="coin.md#0x2_coin_deny_list_add">coin::deny_list_add</a></code>, which
requires <code>&<a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a>&lt;T&gt;</code> and derives the key from <code>T</code>. Raw
<code>(per_type_index, type)</code> pairs are never accepted from outside.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="deny_list.md#0x2_deny_list_v1_add">v1_add</a>(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, per_type_index: u64, type: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, addr: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="deny_list.md#0x2_deny_list_v1_add">v1_add</a>(
    <a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a>,
    per_type_index: u64,
    `type`: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    addr: <b>address</b>,
    ctx: &<b>mut</b> TxContext,
) {
    <b>assert</b>!(!<a href="deny_list.md#0x2_deny_list_is_reserved">is_reserved</a>(addr), <a href="deny_list.md#0x2_deny_list_EInvalidAddress">EInvalidAddress</a>);
    // The genesis singleton is created <b>with</b> an empty `lists` <a href="bag.md#0x2_bag">bag</a> (its
    // <a href="object.md#0x2_object">object</a> id must be the well-known `<a href="deny_list.md#0x2_deny_list_deny_list_object_id">deny_list_object_id</a>()`, which
    // `create`'s `<a href="object.md#0x2_object_new">object::new</a>` cannot produce); materialize the per-type
    // list on first <b>use</b>.
    <b>if</b> (!<a href="bag.md#0x2_bag_contains">bag::contains</a>(&<a href="deny_list.md#0x2_deny_list">deny_list</a>.lists, per_type_index)) {
        <a href="bag.md#0x2_bag_add">bag::add</a>(&<b>mut</b> <a href="deny_list.md#0x2_deny_list">deny_list</a>.lists, per_type_index, <a href="deny_list.md#0x2_deny_list_per_type_list">per_type_list</a>(ctx));
    };
    <b>let</b> key = <a href="deny_list.md#0x2_deny_list_DenyKey">DenyKey</a> { per_type_index, per_type_key: `type` };
    <b>if</b> (ofield::exists_(&<a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key)) {
        <b>let</b> set: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_TypeDenySet">TypeDenySet</a> = ofield::borrow_mut(&<b>mut</b> <a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key);
        <b>if</b> (<a href="vec_set.md#0x2_vec_set_contains">vec_set::contains</a>(&set.addresses, &addr)) <b>return</b>;
        <a href="vec_set.md#0x2_vec_set_insert">vec_set::insert</a>(&<b>mut</b> set.addresses, addr);
    } <b>else</b> {
        <b>let</b> set = <a href="deny_list.md#0x2_deny_list_TypeDenySet">TypeDenySet</a> {
            id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
            addresses: <a href="vec_set.md#0x2_vec_set_singleton">vec_set::singleton</a>(addr),
            paused: <b>false</b>,
        };
        ofield::add(&<b>mut</b> <a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key, set);
    };
    <b>let</b> list: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_PerTypeList">PerTypeList</a> = <a href="bag.md#0x2_bag_borrow_mut">bag::borrow_mut</a>(&<b>mut</b> <a href="deny_list.md#0x2_deny_list">deny_list</a>.lists, per_type_index);
    <b>if</b> (!<a href="table.md#0x2_table_contains">table::contains</a>(&list.denied_count, addr)) {
        <a href="table.md#0x2_table_add">table::add</a>(&<b>mut</b> list.denied_count, addr, 0);
    };
    <b>let</b> count = <a href="table.md#0x2_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> list.denied_count, addr);
    *count = *count + 1;
}
</code></pre>



</details>

<a name="0x2_deny_list_v1_remove"></a>

## Function `v1_remove`

Remove a previously denied address. Aborts <code><a href="deny_list.md#0x2_deny_list_ENotDenied">ENotDenied</a></code> when the
address is not on the list for <code>type</code>.

Friend-only: callers reach this through <code><a href="coin.md#0x2_coin_deny_list_remove">coin::deny_list_remove</a></code>,
which requires <code>&<a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a>&lt;T&gt;</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="deny_list.md#0x2_deny_list_v1_remove">v1_remove</a>(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, per_type_index: u64, type: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="deny_list.md#0x2_deny_list_v1_remove">v1_remove</a>(
    <a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a>,
    per_type_index: u64,
    `type`: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    addr: <b>address</b>,
) {
    <b>assert</b>!(!<a href="deny_list.md#0x2_deny_list_is_reserved">is_reserved</a>(addr), <a href="deny_list.md#0x2_deny_list_EInvalidAddress">EInvalidAddress</a>);
    <b>let</b> key = <a href="deny_list.md#0x2_deny_list_DenyKey">DenyKey</a> { per_type_index, per_type_key: `type` };
    <b>assert</b>!(ofield::exists_(&<a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key), <a href="deny_list.md#0x2_deny_list_ENotDenied">ENotDenied</a>);
    <b>let</b> set: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_TypeDenySet">TypeDenySet</a> = ofield::borrow_mut(&<b>mut</b> <a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key);
    <b>assert</b>!(<a href="vec_set.md#0x2_vec_set_contains">vec_set::contains</a>(&set.addresses, &addr), <a href="deny_list.md#0x2_deny_list_ENotDenied">ENotDenied</a>);
    <a href="vec_set.md#0x2_vec_set_remove">vec_set::remove</a>(&<b>mut</b> set.addresses, &addr);
    <b>let</b> list: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_PerTypeList">PerTypeList</a> = <a href="bag.md#0x2_bag_borrow_mut">bag::borrow_mut</a>(&<b>mut</b> <a href="deny_list.md#0x2_deny_list">deny_list</a>.lists, per_type_index);
    <b>let</b> count = <a href="table.md#0x2_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> list.denied_count, addr);
    *count = *count - 1;
    <b>if</b> (*count == 0) {
        <a href="table.md#0x2_table_remove">table::remove</a>(&<b>mut</b> list.denied_count, addr);
    };
}
</code></pre>



</details>

<a name="0x2_deny_list_v1_contains"></a>

## Function `v1_contains`

Open membership read: anyone may check whether <code>addr</code> is denied for
<code>type</code>. Reserved addresses are never denied.


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_v1_contains">v1_contains</a>(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &<a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, per_type_index: u64, type: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_v1_contains">v1_contains</a>(
    <a href="deny_list.md#0x2_deny_list">deny_list</a>: &<a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a>,
    per_type_index: u64,
    `type`: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    addr: <b>address</b>,
): bool {
    <b>if</b> (<a href="deny_list.md#0x2_deny_list_is_reserved">is_reserved</a>(addr)) <b>return</b> <b>false</b>;
    <b>if</b> (!<a href="bag.md#0x2_bag_contains">bag::contains</a>(&<a href="deny_list.md#0x2_deny_list">deny_list</a>.lists, per_type_index)) <b>return</b> <b>false</b>;
    <b>let</b> list: &<a href="deny_list.md#0x2_deny_list_PerTypeList">PerTypeList</a> = <a href="bag.md#0x2_bag_borrow">bag::borrow</a>(&<a href="deny_list.md#0x2_deny_list">deny_list</a>.lists, per_type_index);
    <b>if</b> (!<a href="table.md#0x2_table_contains">table::contains</a>(&list.denied_count, addr)) <b>return</b> <b>false</b>;
    <b>if</b> (*<a href="table.md#0x2_table_borrow">table::borrow</a>(&list.denied_count, addr) == 0) <b>return</b> <b>false</b>;
    <b>let</b> key = <a href="deny_list.md#0x2_deny_list_DenyKey">DenyKey</a> { per_type_index, per_type_key: `type` };
    <b>if</b> (!ofield::exists_(&<a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key)) <b>return</b> <b>false</b>;
    <b>let</b> set: &<a href="deny_list.md#0x2_deny_list_TypeDenySet">TypeDenySet</a> = ofield::borrow(&<a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key);
    <a href="vec_set.md#0x2_vec_set_contains">vec_set::contains</a>(&set.addresses, &addr)
}
</code></pre>



</details>

<a name="0x2_deny_list_v2_enable_global_pause"></a>

## Function `v2_enable_global_pause`

Pause all regulated flows for <code>type</code>. Creates the type's set object
(unpaused-addresses, paused flag) on first use. Only <code><a href="coin.md#0x2_coin">coin</a></code>
(DenyCap-gated wrappers) may call this.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="deny_list.md#0x2_deny_list_v2_enable_global_pause">v2_enable_global_pause</a>(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, per_type_index: u64, type: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="deny_list.md#0x2_deny_list_v2_enable_global_pause">v2_enable_global_pause</a>(
    <a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a>,
    per_type_index: u64,
    `type`: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    ctx: &<b>mut</b> TxContext,
) {
    <b>let</b> key = <a href="deny_list.md#0x2_deny_list_DenyKey">DenyKey</a> { per_type_index, per_type_key: `type` };
    <b>if</b> (ofield::exists_(&<a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key)) {
        <b>let</b> set: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_TypeDenySet">TypeDenySet</a> = ofield::borrow_mut(&<b>mut</b> <a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key);
        set.paused = <b>true</b>;
    } <b>else</b> {
        <b>let</b> set = <a href="deny_list.md#0x2_deny_list_TypeDenySet">TypeDenySet</a> {
            id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
            addresses: <a href="vec_set.md#0x2_vec_set_empty">vec_set::empty</a>(),
            paused: <b>true</b>,
        };
        ofield::add(&<b>mut</b> <a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key, set);
    };
}
</code></pre>



</details>

<a name="0x2_deny_list_v2_disable_global_pause"></a>

## Function `v2_disable_global_pause`

Lift the pause for <code>type</code>. Missing set object means not paused:
a no-op rather than an abort. Friend-only, via <code><a href="coin.md#0x2_coin_deny_list_unpause">coin::deny_list_unpause</a></code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="deny_list.md#0x2_deny_list_v2_disable_global_pause">v2_disable_global_pause</a>(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, per_type_index: u64, type: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="deny_list.md#0x2_deny_list_v2_disable_global_pause">v2_disable_global_pause</a>(
    <a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a>,
    per_type_index: u64,
    `type`: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    <b>let</b> key = <a href="deny_list.md#0x2_deny_list_DenyKey">DenyKey</a> { per_type_index, per_type_key: `type` };
    <b>if</b> (!ofield::exists_(&<a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key)) <b>return</b>;
    <b>let</b> set: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_TypeDenySet">TypeDenySet</a> = ofield::borrow_mut(&<b>mut</b> <a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key);
    set.paused = <b>false</b>;
}
</code></pre>



</details>

<a name="0x2_deny_list_v2_is_global_pause_enabled"></a>

## Function `v2_is_global_pause_enabled`

Open read: true iff <code>type</code> is currently paused.


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_v2_is_global_pause_enabled">v2_is_global_pause_enabled</a>(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &<a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, per_type_index: u64, type: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_v2_is_global_pause_enabled">v2_is_global_pause_enabled</a>(
    <a href="deny_list.md#0x2_deny_list">deny_list</a>: &<a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a>,
    per_type_index: u64,
    `type`: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): bool {
    <b>let</b> key = <a href="deny_list.md#0x2_deny_list_DenyKey">DenyKey</a> { per_type_index, per_type_key: `type` };
    <b>if</b> (!ofield::exists_(&<a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key)) <b>return</b> <b>false</b>;
    <b>let</b> set: &<a href="deny_list.md#0x2_deny_list_TypeDenySet">TypeDenySet</a> = ofield::borrow(&<a href="deny_list.md#0x2_deny_list">deny_list</a>.id, key);
    set.paused
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
