
<a name="0x2_coin"></a>

# Module `0x2::coin`



-  [Resource `Coin`](#0x2_coin_Coin)
-  [Resource `TreasuryCap`](#0x2_coin_TreasuryCap)
-  [Resource `CoinMetadata`](#0x2_coin_CoinMetadata)
-  [Constants](#@Constants_0)
-  [Function `max_split_parts`](#0x2_coin_max_split_parts)
-  [Function `create_currency`](#0x2_coin_create_currency)
-  [Function `create_regulated_currency`](#0x2_coin_create_regulated_currency)
-  [Function `deny_type_key`](#0x2_coin_deny_type_key)
-  [Function `deny_list_add`](#0x2_coin_deny_list_add)
-  [Function `deny_list_remove`](#0x2_coin_deny_list_remove)
-  [Function `deny_list_contains`](#0x2_coin_deny_list_contains)
-  [Function `deny_list_pause`](#0x2_coin_deny_list_pause)
-  [Function `deny_list_unpause`](#0x2_coin_deny_list_unpause)
-  [Function `deny_list_paused`](#0x2_coin_deny_list_paused)
-  [Function `mint`](#0x2_coin_mint)
-  [Function `mint_and_transfer`](#0x2_coin_mint_and_transfer)
-  [Function `burn`](#0x2_coin_burn)
-  [Function `into_balance`](#0x2_coin_into_balance)
-  [Function `from_balance`](#0x2_coin_from_balance)
-  [Function `total_supply`](#0x2_coin_total_supply)
-  [Function `value`](#0x2_coin_value)
-  [Function `zero`](#0x2_coin_zero)
-  [Function `split`](#0x2_coin_split)
-  [Function `join`](#0x2_coin_join)
-  [Function `join_entry`](#0x2_coin_join_entry)
-  [Function `destroy_zero`](#0x2_coin_destroy_zero)
-  [Function `divide_into_n`](#0x2_coin_divide_into_n)
-  [Function `update_icon_url`](#0x2_coin_update_icon_url)
-  [Function `update_name`](#0x2_coin_update_name)
-  [Function `update_symbol`](#0x2_coin_update_symbol)
-  [Function `update_description`](#0x2_coin_update_description)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/ascii.md#0x1_ascii">0x1::ascii</a>;
<b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="dependencies/move-stdlib/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">0x1::type_name</a>;
<b>use</b> <a href="balance.md#0x2_balance">0x2::balance</a>;
<b>use</b> <a href="deny_list.md#0x2_deny_list">0x2::deny_list</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
<b>use</b> <a href="types.md#0x2_types">0x2::types</a>;
<b>use</b> <a href="url.md#0x2_url">0x2::url</a>;
</code></pre>



<a name="0x2_coin_Coin"></a>

## Resource `Coin`

Coin resource wrapper with balance (Removed <code>drop</code> for asset safety)


<pre><code><b>struct</b> <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt; <b>has</b> store, key
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
<code><a href="balance.md#0x2_balance">balance</a>: <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_coin_TreasuryCap"></a>

## Resource `TreasuryCap`

Capability allowing the bearer to mint and burn coins (Removed <code>drop</code>)


<pre><code><b>struct</b> <a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt; <b>has</b> store, key
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
<code>total_supply: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_coin_CoinMetadata"></a>

## Resource `CoinMetadata`

Metadata resource for a currency (Removed <code>drop</code>)


<pre><code><b>struct</b> <a href="coin.md#0x2_coin_CoinMetadata">CoinMetadata</a>&lt;T&gt; <b>has</b> store, key
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
<code>decimals: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>name: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>symbol: <a href="dependencies/move-stdlib/ascii.md#0x1_ascii_String">ascii::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>icon_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_coin_EINVALID_DECIMALS"></a>

Invalid decimal count.


<pre><code><b>const</b> <a href="coin.md#0x2_coin_EINVALID_DECIMALS">EINVALID_DECIMALS</a>: u64 = 6;
</code></pre>



<a name="0x2_coin_EInvalidArg"></a>

Invalid arguments are passed to a function.


<pre><code><b>const</b> <a href="coin.md#0x2_coin_EInvalidArg">EInvalidArg</a>: u64 = 1;
</code></pre>



<a name="0x2_coin_ENotEnough"></a>

Trying to split a coin more times than its balance allows.


<pre><code><b>const</b> <a href="coin.md#0x2_coin_ENotEnough">ENotEnough</a>: u64 = 2;
</code></pre>



<a name="0x2_coin_EOVERFLOW"></a>

Arithmetic overflow.


<pre><code><b>const</b> <a href="coin.md#0x2_coin_EOVERFLOW">EOVERFLOW</a>: u64 = 4;
</code></pre>



<a name="0x2_coin_ETOO_MANY_PARTS"></a>

Split fan-out exceeds the per-call object cap.


<pre><code><b>const</b> <a href="coin.md#0x2_coin_ETOO_MANY_PARTS">ETOO_MANY_PARTS</a>: u64 = 7;
</code></pre>



<a name="0x2_coin_EUNDERFLOW"></a>

Arithmetic underflow.


<pre><code><b>const</b> <a href="coin.md#0x2_coin_EUNDERFLOW">EUNDERFLOW</a>: u64 = 5;
</code></pre>



<a name="0x2_coin_EZERO_AMOUNT"></a>

Amount must be greater than zero.


<pre><code><b>const</b> <a href="coin.md#0x2_coin_EZERO_AMOUNT">EZERO_AMOUNT</a>: u64 = 3;
</code></pre>



<a name="0x2_coin_MAX_SPLIT_PARTS"></a>

Maximum coins one <code>divide_into_n</code> call may create. Bounds object
creation per call even when transaction gas accounting is permissive;
large airdrops should batch across transactions.


<pre><code><b>const</b> <a href="coin.md#0x2_coin_MAX_SPLIT_PARTS">MAX_SPLIT_PARTS</a>: u64 = 1024;
</code></pre>



<a name="0x2_coin_max_split_parts"></a>

## Function `max_split_parts`

The per-call split fan-out cap (see <code><a href="coin.md#0x2_coin_MAX_SPLIT_PARTS">MAX_SPLIT_PARTS</a></code>).


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_max_split_parts">max_split_parts</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_max_split_parts">max_split_parts</a>(): u64 {
    <a href="coin.md#0x2_coin_MAX_SPLIT_PARTS">MAX_SPLIT_PARTS</a>
}
</code></pre>



</details>

<a name="0x2_coin_create_currency"></a>

## Function `create_currency`

Create a new currency with TreasuryCap for minting control and return the
TreasuryCap and the Metadata object.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_create_currency">create_currency</a>&lt;T: drop&gt;(witness: T, decimals: u8, symbol_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, name_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, description_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, icon_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): (<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, <a href="coin.md#0x2_coin_CoinMetadata">coin::CoinMetadata</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_create_currency">create_currency</a>&lt;T: drop&gt;(
    witness: T,
    decimals: u8,
    symbol_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    name_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    description_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    icon_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;,
    ctx: &<b>mut</b> TxContext,
): (<a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt;, <a href="coin.md#0x2_coin_CoinMetadata">CoinMetadata</a>&lt;T&gt;) {
    // One canonical currency per <a href="coin.md#0x2_coin">coin</a> type: the host rejects a second
    // claim for `T`, so mint authority cannot fragment across caps.
    kanari_system::types::claim_witness(&witness);
    <b>let</b> _ = witness;

    <b>assert</b>!(decimals &lt;= 9, <a href="coin.md#0x2_coin_EINVALID_DECIMALS">EINVALID_DECIMALS</a>);

    <b>let</b> symbol = <a href="dependencies/move-stdlib/ascii.md#0x1_ascii_string">ascii::string</a>(symbol_bytes);
    <b>let</b> name = <a href="dependencies/move-stdlib/string.md#0x1_string_utf8">string::utf8</a>(name_bytes);
    <b>let</b> description = <a href="dependencies/move-stdlib/string.md#0x1_string_utf8">string::utf8</a>(description_bytes);

    <b>let</b> treasury_cap = <a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt; { id: <a href="object.md#0x2_object_new">object::new</a>(ctx), total_supply: 0 };
    <b>let</b> metadata = <a href="coin.md#0x2_coin_CoinMetadata">CoinMetadata</a>&lt;T&gt; {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        decimals,
        name,
        symbol,
        description,
        icon_url
    };

    (treasury_cap, metadata)
}
</code></pre>



</details>

<a name="0x2_coin_create_regulated_currency"></a>

## Function `create_regulated_currency`

Create a regulated currency, returning the mint <code><a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a></code>, a
<code>DenyCap</code> for managing a per-currency deny list, and the metadata.

Enforcement model: this creates the *capability* to maintain deny
entries in the shared <code><a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a></code> singleton; it does not by
itself block transfers. Entries are managed with the returned
<code>DenyCap&lt;T&gt;</code> via <code>deny_list_add</code>/<code>deny_list_remove</code> and enforced by
the <code>*_checked</code> transfer paths (<code><a href="pay.md#0x2_pay_split_and_transfer_checked">pay::split_and_transfer_checked</a></code>,
<code><a href="pay.md#0x2_pay_join_vec_and_transfer_checked">pay::join_vec_and_transfer_checked</a></code>,
<code><a href="multisig.md#0x2_multisig_execute_transfer_checked">multisig::execute_transfer_checked</a></code>), which consult the shared list
through <code>deny_list_contains</code>. The plain <code><a href="coin.md#0x2_coin">coin</a>::*</code> and
<code><a href="pay.md#0x2_pay_split_and_transfer">pay::split_and_transfer</a></code>/<code>join_vec_and_transfer</code> paths stay
permissionless and skip the check, so a regulated coin must be moved
through the checked paths to be enforceable.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_create_regulated_currency">create_regulated_currency</a>&lt;T: drop&gt;(witness: T, decimals: u8, symbol_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, name_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, description_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, icon_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): (<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, <a href="deny_list.md#0x2_deny_list_DenyCap">deny_list::DenyCap</a>&lt;T&gt;, <a href="coin.md#0x2_coin_CoinMetadata">coin::CoinMetadata</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_create_regulated_currency">create_regulated_currency</a>&lt;T: drop&gt;(
    witness: T,
    decimals: u8,
    symbol_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    name_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    description_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    icon_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;,
    ctx: &<b>mut</b> TxContext,
): (
    <a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt;, kanari_system::deny_list::DenyCap&lt;T&gt;, <a href="coin.md#0x2_coin_CoinMetadata">CoinMetadata</a>&lt;T&gt;
) {
    // One canonical currency per <a href="coin.md#0x2_coin">coin</a> type (see `create_currency`).
    kanari_system::types::claim_witness(&witness);
    <b>let</b> _ = witness;
    <b>assert</b>!(decimals &lt;= 9, <a href="coin.md#0x2_coin_EINVALID_DECIMALS">EINVALID_DECIMALS</a>);

    <b>let</b> symbol = <a href="dependencies/move-stdlib/ascii.md#0x1_ascii_string">ascii::string</a>(symbol_bytes);
    <b>let</b> name = <a href="dependencies/move-stdlib/string.md#0x1_string_utf8">string::utf8</a>(name_bytes);
    <b>let</b> description = <a href="dependencies/move-stdlib/string.md#0x1_string_utf8">string::utf8</a>(description_bytes);

    <b>let</b> treasury_cap = <a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt; { id: <a href="object.md#0x2_object_new">object::new</a>(ctx), total_supply: 0 };
    <b>let</b> denycap = kanari_system::deny_list::new_denycap&lt;T&gt;(ctx);
    <b>let</b> metadata = <a href="coin.md#0x2_coin_CoinMetadata">CoinMetadata</a>&lt;T&gt; {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        decimals,
        name,
        symbol,
        description,
        icon_url
    };
    (treasury_cap, denycap, metadata)
}
</code></pre>



</details>

<a name="0x2_coin_deny_type_key"></a>

## Function `deny_type_key`

Canonical deny-list key for <code>T</code>: its <code><a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a></code> bytes
(e.g. <code>0x2::my_coin::MY_COIN</code>, never the <code><a href="coin.md#0x2_coin_Coin">Coin</a>&lt;...&gt;</code> wrapper).


<pre><code><b>fun</b> <a href="coin.md#0x2_coin_deny_type_key">deny_type_key</a>&lt;T&gt;(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="coin.md#0x2_coin_deny_type_key">deny_type_key</a>&lt;T&gt;(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> name = <a href="dependencies/move-stdlib/type_name.md#0x1_type_name_get">type_name::get</a>&lt;T&gt;();
    *<a href="dependencies/move-stdlib/ascii.md#0x1_ascii_as_bytes">ascii::as_bytes</a>(<a href="dependencies/move-stdlib/type_name.md#0x1_type_name_borrow_string">type_name::borrow_string</a>(&name))
}
</code></pre>



</details>

<a name="0x2_coin_deny_list_add"></a>

## Function `deny_list_add`

Add <code>addr</code> to <code>T</code>'s deny entries in the shared deny list. Requires
<code>&DenyCap&lt;T&gt;</code> minted by <code>create_regulated_currency</code>, so a capability
for one coin can never touch another coin's entries.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_deny_list_add">deny_list_add</a>&lt;T&gt;(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, _cap: &<a href="deny_list.md#0x2_deny_list_DenyCap">deny_list::DenyCap</a>&lt;T&gt;, addr: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_deny_list_add">deny_list_add</a>&lt;T&gt;(
    <a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> DenyList,
    _cap: &DenyCap&lt;T&gt;,
    addr: <b>address</b>,
    ctx: &<b>mut</b> TxContext,
) {
    <a href="deny_list.md#0x2_deny_list_v1_add">deny_list::v1_add</a>(<a href="deny_list.md#0x2_deny_list">deny_list</a>, <a href="deny_list.md#0x2_deny_list_coin_index">deny_list::coin_index</a>(), <a href="coin.md#0x2_coin_deny_type_key">deny_type_key</a>&lt;T&gt;(), addr, ctx);
}
</code></pre>



</details>

<a name="0x2_coin_deny_list_remove"></a>

## Function `deny_list_remove`

Remove <code>addr</code> from <code>T</code>'s deny entries. Aborts when not denied.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_deny_list_remove">deny_list_remove</a>&lt;T&gt;(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, _cap: &<a href="deny_list.md#0x2_deny_list_DenyCap">deny_list::DenyCap</a>&lt;T&gt;, addr: <b>address</b>, _ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_deny_list_remove">deny_list_remove</a>&lt;T&gt;(
    <a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> DenyList,
    _cap: &DenyCap&lt;T&gt;,
    addr: <b>address</b>,
    _ctx: &<b>mut</b> TxContext,
) {
    <a href="deny_list.md#0x2_deny_list_v1_remove">deny_list::v1_remove</a>(<a href="deny_list.md#0x2_deny_list">deny_list</a>, <a href="deny_list.md#0x2_deny_list_coin_index">deny_list::coin_index</a>(), <a href="coin.md#0x2_coin_deny_type_key">deny_type_key</a>&lt;T&gt;(), addr);
}
</code></pre>



</details>

<a name="0x2_coin_deny_list_contains"></a>

## Function `deny_list_contains`

Open membership read used by the <code>*_checked</code> transfer paths.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_deny_list_contains">deny_list_contains</a>&lt;T&gt;(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &<a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_deny_list_contains">deny_list_contains</a>&lt;T&gt;(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &DenyList, addr: <b>address</b>): bool {
    <a href="deny_list.md#0x2_deny_list_v1_contains">deny_list::v1_contains</a>(<a href="deny_list.md#0x2_deny_list">deny_list</a>, <a href="deny_list.md#0x2_deny_list_coin_index">deny_list::coin_index</a>(), <a href="coin.md#0x2_coin_deny_type_key">deny_type_key</a>&lt;T&gt;(), addr)
}
</code></pre>



</details>

<a name="0x2_coin_deny_list_pause"></a>

## Function `deny_list_pause`

Pause all regulated flows for <code>T</code> (fail-closed kill-switch).
Requires <code>&DenyCap&lt;T&gt;</code>, like deny entries.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_deny_list_pause">deny_list_pause</a>&lt;T&gt;(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, _cap: &<a href="deny_list.md#0x2_deny_list_DenyCap">deny_list::DenyCap</a>&lt;T&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_deny_list_pause">deny_list_pause</a>&lt;T&gt;(
    <a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> DenyList,
    _cap: &DenyCap&lt;T&gt;,
    ctx: &<b>mut</b> TxContext,
) {
    <a href="deny_list.md#0x2_deny_list_v2_enable_global_pause">deny_list::v2_enable_global_pause</a>(
        <a href="deny_list.md#0x2_deny_list">deny_list</a>,
        <a href="deny_list.md#0x2_deny_list_coin_index">deny_list::coin_index</a>(),
        <a href="coin.md#0x2_coin_deny_type_key">deny_type_key</a>&lt;T&gt;(),
        ctx
    );
}
</code></pre>



</details>

<a name="0x2_coin_deny_list_unpause"></a>

## Function `deny_list_unpause`

Lift the pause for <code>T</code>. Requires <code>&DenyCap&lt;T&gt;</code>.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_deny_list_unpause">deny_list_unpause</a>&lt;T&gt;(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, _cap: &<a href="deny_list.md#0x2_deny_list_DenyCap">deny_list::DenyCap</a>&lt;T&gt;, _ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_deny_list_unpause">deny_list_unpause</a>&lt;T&gt;(
    <a href="deny_list.md#0x2_deny_list">deny_list</a>: &<b>mut</b> DenyList,
    _cap: &DenyCap&lt;T&gt;,
    _ctx: &<b>mut</b> TxContext,
) {
    <a href="deny_list.md#0x2_deny_list_v2_disable_global_pause">deny_list::v2_disable_global_pause</a>(<a href="deny_list.md#0x2_deny_list">deny_list</a>, <a href="deny_list.md#0x2_deny_list_coin_index">deny_list::coin_index</a>(), <a href="coin.md#0x2_coin_deny_type_key">deny_type_key</a>&lt;T&gt;());
}
</code></pre>



</details>

<a name="0x2_coin_deny_list_paused"></a>

## Function `deny_list_paused`

Open read: true iff <code>T</code> is currently paused. The <code>*_checked</code>
transfer paths reject paused types alongside denied addresses.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_deny_list_paused">deny_list_paused</a>&lt;T&gt;(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &<a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_deny_list_paused">deny_list_paused</a>&lt;T&gt;(<a href="deny_list.md#0x2_deny_list">deny_list</a>: &DenyList): bool {
    <a href="deny_list.md#0x2_deny_list_v2_is_global_pause_enabled">deny_list::v2_is_global_pause_enabled</a>(<a href="deny_list.md#0x2_deny_list">deny_list</a>, <a href="deny_list.md#0x2_deny_list_coin_index">deny_list::coin_index</a>(), <a href="coin.md#0x2_coin_deny_type_key">deny_type_key</a>&lt;T&gt;())
}
</code></pre>



</details>

<a name="0x2_coin_mint"></a>

## Function `mint`

Mint new coins using TreasuryCap


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_mint">mint</a>&lt;T&gt;(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, amount: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_mint">mint</a>&lt;T&gt;(
    cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt;,
    amount: u64,
    ctx: &<b>mut</b> TxContext,
): <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt; {
    <b>assert</b>!(amount &gt; 0, <a href="coin.md#0x2_coin_EZERO_AMOUNT">EZERO_AMOUNT</a>);
    <b>let</b> new_total = cap.total_supply + amount;
    <b>assert</b>!(new_total &gt;= cap.total_supply, <a href="coin.md#0x2_coin_EOVERFLOW">EOVERFLOW</a>);
    cap.total_supply = new_total;
    <a href="object.md#0x2_object_save_object">object::save_object</a>(cap);
    <a href="coin.md#0x2_coin_Coin">Coin</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        <a href="balance.md#0x2_balance">balance</a>: kanari_system::balance::create&lt;T&gt;(amount)
    }
}
</code></pre>



</details>

<a name="0x2_coin_mint_and_transfer"></a>

## Function `mint_and_transfer`

Mint and transfer to recipient


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_mint_and_transfer">mint_and_transfer</a>&lt;T&gt;(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, amount: u64, recipient: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_mint_and_transfer">mint_and_transfer</a>&lt;T&gt;(
    cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt;,
    amount: u64,
    recipient: <b>address</b>,
    ctx: &<b>mut</b> TxContext,
) {
    <b>let</b> <a href="coin.md#0x2_coin">coin</a> = <a href="coin.md#0x2_coin_mint">mint</a>(cap, amount, ctx);
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(<a href="coin.md#0x2_coin">coin</a>, recipient);
}
</code></pre>



</details>

<a name="0x2_coin_burn"></a>

## Function `burn`

Burn coins, decreasing total supply


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_burn">burn</a>&lt;T&gt;(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, <a href="coin.md#0x2_coin">coin</a>: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_burn">burn</a>&lt;T&gt;(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt;, <a href="coin.md#0x2_coin">coin</a>: <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;): u64 {
    <b>let</b> <a href="coin.md#0x2_coin_Coin">Coin</a> { id, <a href="balance.md#0x2_balance">balance</a> } = <a href="coin.md#0x2_coin">coin</a>;
    <b>let</b> value = kanari_system::balance::destroy&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>);
    <b>assert</b>!(value &gt; 0, <a href="coin.md#0x2_coin_EZERO_AMOUNT">EZERO_AMOUNT</a>);
    <b>assert</b>!(cap.total_supply &gt;= value, <a href="coin.md#0x2_coin_EUNDERFLOW">EUNDERFLOW</a>);
    cap.total_supply = cap.total_supply - value;
    <a href="object.md#0x2_object_save_object">object::save_object</a>(cap);
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
    value
}
</code></pre>



</details>

<a name="0x2_coin_into_balance"></a>

## Function `into_balance`

Convert a <code><a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;</code> into its inner <code>Balance&lt;T&gt;</code>.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_into_balance">into_balance</a>&lt;T&gt;(<a href="coin.md#0x2_coin">coin</a>: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;): <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_into_balance">into_balance</a>&lt;T&gt;(<a href="coin.md#0x2_coin">coin</a>: <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;): Balance&lt;T&gt; {
    <b>let</b> <a href="coin.md#0x2_coin_Coin">Coin</a> { id, <a href="balance.md#0x2_balance">balance</a> } = <a href="coin.md#0x2_coin">coin</a>;
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
    <a href="balance.md#0x2_balance">balance</a>
}
</code></pre>



</details>

<a name="0x2_coin_from_balance"></a>

## Function `from_balance`

Construct a <code><a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;</code> from a <code>Balance&lt;T&gt;</code>.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_from_balance">from_balance</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_from_balance">from_balance</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: Balance&lt;T&gt;, ctx: &<b>mut</b> TxContext): <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt; {
    <a href="coin.md#0x2_coin_Coin">Coin</a> { id: <a href="object.md#0x2_object_new">object::new</a>(ctx), <a href="balance.md#0x2_balance">balance</a> }
}
</code></pre>



</details>

<a name="0x2_coin_total_supply"></a>

## Function `total_supply`

Get total supply from TreasuryCap


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_total_supply">total_supply</a>&lt;T&gt;(cap: &<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_total_supply">total_supply</a>&lt;T&gt;(cap: &<a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt;): u64 {
    cap.total_supply
}
</code></pre>



</details>

<a name="0x2_coin_value"></a>

## Function `value`

Get coin value


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_value">value</a>&lt;T&gt;(<a href="coin.md#0x2_coin">coin</a>: &<a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_value">value</a>&lt;T&gt;(<a href="coin.md#0x2_coin">coin</a>: &<a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;): u64 {
    kanari_system::balance::value(&<a href="coin.md#0x2_coin">coin</a>.<a href="balance.md#0x2_balance">balance</a>)
}
</code></pre>



</details>

<a name="0x2_coin_zero"></a>

## Function `zero`

Create a zero-value Coin<T> (useful for testing and as a starting point)


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_zero">zero</a>&lt;T&gt;(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_zero">zero</a>&lt;T&gt;(ctx: &<b>mut</b> TxContext): <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt; {
    <a href="coin.md#0x2_coin_Coin">Coin</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        <a href="balance.md#0x2_balance">balance</a>: kanari_system::balance::create&lt;T&gt;(0)
    }
}
</code></pre>



</details>

<a name="0x2_coin_split"></a>

## Function `split`

Split a coin into two. Returns the new coin with the specified amount.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_split">split</a>&lt;T&gt;(<a href="coin.md#0x2_coin">coin</a>: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;, amount: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_split">split</a>&lt;T&gt;(<a href="coin.md#0x2_coin">coin</a>: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;, amount: u64, ctx: &<b>mut</b> TxContext): <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt; {
    <b>assert</b>!(amount &gt; 0, <a href="coin.md#0x2_coin_EZERO_AMOUNT">EZERO_AMOUNT</a>);
    <b>assert</b>!(<a href="coin.md#0x2_coin_value">value</a>(<a href="coin.md#0x2_coin">coin</a>) &gt;= amount, <a href="coin.md#0x2_coin_ENotEnough">ENotEnough</a>);

    <b>let</b> new_balance = kanari_system::balance::split(&<b>mut</b> <a href="coin.md#0x2_coin">coin</a>.<a href="balance.md#0x2_balance">balance</a>, amount);
    <a href="object.md#0x2_object_save_object">object::save_object</a>(<a href="coin.md#0x2_coin">coin</a>);
    <a href="coin.md#0x2_coin_Coin">Coin</a> { id: <a href="object.md#0x2_object_new">object::new</a>(ctx), <a href="balance.md#0x2_balance">balance</a>: new_balance }
}
</code></pre>



</details>

<a name="0x2_coin_join"></a>

## Function `join`

Join two coins together (adds the balance of 'other' into 'coin').


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_join">join</a>&lt;T&gt;(<a href="coin.md#0x2_coin">coin</a>: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;, other: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_join">join</a>&lt;T&gt;(<a href="coin.md#0x2_coin">coin</a>: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;, other: <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;) {
    <b>let</b> <a href="coin.md#0x2_coin_Coin">Coin</a> { id, <a href="balance.md#0x2_balance">balance</a> } = other;
    kanari_system::balance::merge(&<b>mut</b> <a href="coin.md#0x2_coin">coin</a>.<a href="balance.md#0x2_balance">balance</a>, <a href="balance.md#0x2_balance">balance</a>);
    <a href="object.md#0x2_object_save_object">object::save_object</a>(<a href="coin.md#0x2_coin">coin</a>);
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
}
</code></pre>



</details>

<a name="0x2_coin_join_entry"></a>

## Function `join_entry`

Entry wrapper for joining two coin objects owned by the sender.


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x2_coin_join_entry">join_entry</a>&lt;T&gt;(<a href="coin.md#0x2_coin">coin</a>: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;, other: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x2_coin_join_entry">join_entry</a>&lt;T&gt;(<a href="coin.md#0x2_coin">coin</a>: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;, other: <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;) {
    <a href="coin.md#0x2_coin_join">join</a>(<a href="coin.md#0x2_coin">coin</a>, other);
}
</code></pre>



</details>

<a name="0x2_coin_destroy_zero"></a>

## Function `destroy_zero`

Destroy a zero-balance coin.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_destroy_zero">destroy_zero</a>&lt;T&gt;(<a href="coin.md#0x2_coin">coin</a>: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_destroy_zero">destroy_zero</a>&lt;T&gt;(<a href="coin.md#0x2_coin">coin</a>: <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;) {
    <b>let</b> <a href="coin.md#0x2_coin_Coin">Coin</a> { id, <a href="balance.md#0x2_balance">balance</a> } = <a href="coin.md#0x2_coin">coin</a>;
    <b>assert</b>!(kanari_system::balance::value(&<a href="balance.md#0x2_balance">balance</a>) == 0, <a href="coin.md#0x2_coin_EInvalidArg">EInvalidArg</a>);
    kanari_system::balance::destroy&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>);
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
}
</code></pre>



</details>

<a name="0x2_coin_divide_into_n"></a>

## Function `divide_into_n`

Split coin <code>self</code> into <code>n - 1</code> coins with equal balances.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_divide_into_n">divide_into_n</a>&lt;T&gt;(self: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;, n: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_divide_into_n">divide_into_n</a>&lt;T&gt;(
    self: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;,
    n: u64,
    ctx: &<b>mut</b> TxContext,
): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;&gt; {
    <b>assert</b>!(n &gt; 0, <a href="coin.md#0x2_coin_EInvalidArg">EInvalidArg</a>);
    <b>assert</b>!(n &lt;= <a href="coin.md#0x2_coin_MAX_SPLIT_PARTS">MAX_SPLIT_PARTS</a>, <a href="coin.md#0x2_coin_ETOO_MANY_PARTS">ETOO_MANY_PARTS</a>);
    <b>assert</b>!(<a href="coin.md#0x2_coin_value">value</a>(self) &gt;= n, <a href="coin.md#0x2_coin_ENotEnough">ENotEnough</a>);

    <b>let</b> vec = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;&gt;();
    <b>let</b> i = 0;
    <b>let</b> split_amount = <a href="coin.md#0x2_coin_value">value</a>(self) / n;
    <b>while</b> (i &lt; n - 1) {
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> vec, <a href="coin.md#0x2_coin_split">split</a>(self, split_amount, ctx));
        i = i + 1;
    };
    vec
}
</code></pre>



</details>

<a name="0x2_coin_update_icon_url"></a>

## Function `update_icon_url`



<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_update_icon_url">update_icon_url</a>&lt;T&gt;(treasury: &<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, metadata: &<b>mut</b> <a href="coin.md#0x2_coin_CoinMetadata">coin::CoinMetadata</a>&lt;T&gt;, <a href="url.md#0x2_url">url</a>: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_update_icon_url">update_icon_url</a>&lt;T&gt;(
    treasury: &<a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt;,
    metadata: &<b>mut</b> <a href="coin.md#0x2_coin_CoinMetadata">CoinMetadata</a>&lt;T&gt;,
    <a href="url.md#0x2_url">url</a>: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;,
) {
    <b>let</b> _ = treasury;
    metadata.icon_url = <a href="url.md#0x2_url">url</a>;
}
</code></pre>



</details>

<a name="0x2_coin_update_name"></a>

## Function `update_name`



<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_update_name">update_name</a>&lt;T&gt;(treasury: &<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, metadata: &<b>mut</b> <a href="coin.md#0x2_coin_CoinMetadata">coin::CoinMetadata</a>&lt;T&gt;, name: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_update_name">update_name</a>&lt;T&gt;(
    treasury: &<a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt;,
    metadata: &<b>mut</b> <a href="coin.md#0x2_coin_CoinMetadata">CoinMetadata</a>&lt;T&gt;,
    name: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
) {
    <b>let</b> _ = treasury;
    metadata.name = name;
}
</code></pre>



</details>

<a name="0x2_coin_update_symbol"></a>

## Function `update_symbol`



<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_update_symbol">update_symbol</a>&lt;T&gt;(treasury: &<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, metadata: &<b>mut</b> <a href="coin.md#0x2_coin_CoinMetadata">coin::CoinMetadata</a>&lt;T&gt;, symbol: <a href="dependencies/move-stdlib/ascii.md#0x1_ascii_String">ascii::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_update_symbol">update_symbol</a>&lt;T&gt;(
    treasury: &<a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt;,
    metadata: &<b>mut</b> <a href="coin.md#0x2_coin_CoinMetadata">CoinMetadata</a>&lt;T&gt;,
    symbol: <a href="dependencies/move-stdlib/ascii.md#0x1_ascii_String">ascii::String</a>,
) {
    <b>let</b> _ = treasury;
    metadata.symbol = symbol;
}
</code></pre>



</details>

<a name="0x2_coin_update_description"></a>

## Function `update_description`



<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_update_description">update_description</a>&lt;T&gt;(treasury: &<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, metadata: &<b>mut</b> <a href="coin.md#0x2_coin_CoinMetadata">coin::CoinMetadata</a>&lt;T&gt;, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_update_description">update_description</a>&lt;T&gt;(
    treasury: &<a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt;,
    metadata: &<b>mut</b> <a href="coin.md#0x2_coin_CoinMetadata">CoinMetadata</a>&lt;T&gt;,
    description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
) {
    <b>let</b> _ = treasury;
    metadata.description = description;
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
