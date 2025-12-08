
<a name="0x2_kanari_laps_coin"></a>

# Module `0x2::kanari_laps_coin`



-  [Struct `COIN`](#0x2_kanari_laps_coin_COIN)
-  [Struct `Borrower`](#0x2_kanari_laps_coin_Borrower)
-  [Struct `Mint`](#0x2_kanari_laps_coin_Mint)
-  [Resource `CoinMetadata`](#0x2_kanari_laps_coin_CoinMetadata)
-  [Function `init`](#0x2_kanari_laps_coin_init)
-  [Function `total_supply`](#0x2_kanari_laps_coin_total_supply)
-  [Function `borrow`](#0x2_kanari_laps_coin_borrow)
-  [Function `mint`](#0x2_kanari_laps_coin_mint)
-  [Function `burn`](#0x2_kanari_laps_coin_burn)
-  [Function `deny_list_add_admin`](#0x2_kanari_laps_coin_deny_list_add_admin)
-  [Function `deny_list_remove_admin`](#0x2_kanari_laps_coin_deny_list_remove_admin)
-  [Function `destroy_zero`](#0x2_kanari_laps_coin_destroy_zero)
-  [Function `transfer`](#0x2_kanari_laps_coin_transfer)
-  [Function `update_name`](#0x2_kanari_laps_coin_update_name)
-  [Function `update_symbol`](#0x2_kanari_laps_coin_update_symbol)
-  [Function `update_description`](#0x2_kanari_laps_coin_update_description)
-  [Function `update_icon_url`](#0x2_kanari_laps_coin_update_icon_url)
-  [Function `get_decimals`](#0x2_kanari_laps_coin_get_decimals)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/ascii.md#0x1_ascii">0x1::ascii</a>;
<b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="dependencies/move-stdlib/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="balance.md#0x2_balance">0x2::balance</a>;
<b>use</b> <a href="coin.md#0x2_coin">0x2::coin</a>;
<b>use</b> <a href="deny_list.md#0x2_deny_list">0x2::deny_list</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
<b>use</b> <a href="url.md#0x2_url">0x2::url</a>;
</code></pre>



<a name="0x2_kanari_laps_coin_COIN"></a>

## Struct `COIN`



<pre><code><b>struct</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_kanari_laps_coin_Borrower"></a>

## Struct `Borrower`



<pre><code><b>struct</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_Borrower">Borrower</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>sender: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_kanari_laps_coin_Mint"></a>

## Struct `Mint`



<pre><code><b>struct</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_Mint">Mint</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>sender: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_kanari_laps_coin_CoinMetadata"></a>

## Resource `CoinMetadata`



<pre><code><b>struct</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_CoinMetadata">CoinMetadata</a> <b>has</b> store, key
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

<a name="0x2_kanari_laps_coin_init"></a>

## Function `init`



<pre><code><b>public</b> <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_init">init</a>(witness: <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_init">init</a>(witness: <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>, ctx: &<b>mut</b> TxContext) {
    // Create the <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a> governance token <b>with</b> 9 decimals
    <b>let</b> (treasury, denycap, metadata) = <a href="coin.md#0x2_coin_create_regulated_currency">coin::create_regulated_currency</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;(
        witness,
        9,
        b"KARI",
        b"Kanari Token",
        b"The governance token of Kanari Network",
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(<a href="url.md#0x2_url_new_unsafe_from_bytes">url::new_unsafe_from_bytes</a>(b"https://magenta-able-pheasant-388.mypinata.cloud/ipfs/QmNVQ3LQSbLC8bJDnXrbuftf2dC7LWJp4oXVkXxVRrDRfk")),
        ctx
    );
    // Get the sender of the transaction
    <b>let</b> sender = <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);

    // Transfer the treasury and denycap objects <b>to</b> the sender
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(treasury, sender);
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(denycap, sender);


    // Freeze the metadata <a href="object.md#0x2_object">object</a>
    <a href="transfer.md#0x2_transfer_public_freeze_object">transfer::public_freeze_object</a>(metadata);
}
</code></pre>



</details>

<a name="0x2_kanari_laps_coin_total_supply"></a>

## Function `total_supply`



<pre><code><b>public</b> <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_total_supply">total_supply</a>(cap: &<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_total_supply">total_supply</a>(cap: &TreasuryCap&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;) : u64 {
    <a href="coin.md#0x2_coin_total_supply">coin::total_supply</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;(cap)
}
</code></pre>



</details>

<a name="0x2_kanari_laps_coin_borrow"></a>

## Function `borrow`



<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_borrow">borrow</a>(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>&gt;, amount: u64, sender: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_borrow">borrow</a>(
    cap: &<b>mut</b> TreasuryCap&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;,
    amount: u64,
    sender: <b>address</b>,
    ctx: &<b>mut</b> TxContext,
) {
    <b>let</b> borrow = <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_Borrower">Borrower</a> {
        amount,
        sender,
    };
    // <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_Mint">Mint</a> and <a href="transfer.md#0x2_transfer">transfer</a> the borrowed <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a> tokens <b>to</b> the borrower
    <a href="coin.md#0x2_coin_mint_and_transfer">coin::mint_and_transfer</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;(cap, borrow.amount, borrow.sender, ctx);
}
</code></pre>



</details>

<a name="0x2_kanari_laps_coin_mint"></a>

## Function `mint`



<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_mint">mint</a>(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>&gt;, amount: u64, sender: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_mint">mint</a>(
    cap: &<b>mut</b> TreasuryCap&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;,
    amount: u64,
    sender: <b>address</b>,
    ctx: &<b>mut</b> TxContext,
) {
    <b>let</b> mint = <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_Mint">Mint</a> {
        amount,
        sender,
    };
    // <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_Mint">Mint</a> and <a href="transfer.md#0x2_transfer">transfer</a> the minted <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a> tokens <b>to</b> the sender
    <a href="coin.md#0x2_coin_mint_and_transfer">coin::mint_and_transfer</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;(cap, mint.amount, mint.sender, ctx);
}
</code></pre>



</details>

<a name="0x2_kanari_laps_coin_burn"></a>

## Function `burn`



<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_burn">burn</a>(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>&gt;, c: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_burn">burn</a>(
    cap: &<b>mut</b> TreasuryCap&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;,
    c: Coin&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;,
) {
    <a href="coin.md#0x2_coin_burn">coin::burn</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;(cap, c);
}
</code></pre>



</details>

<a name="0x2_kanari_laps_coin_deny_list_add_admin"></a>

## Function `deny_list_add_admin`



<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_deny_list_add_admin">deny_list_add_admin</a>(denylist: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, denycap: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyCap">deny_list::DenyCap</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>&gt;, sender: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_deny_list_add_admin">deny_list_add_admin</a>(
    denylist: &<b>mut</b> DenyList,
    denycap: &<b>mut</b> DenyCap&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;,
    sender: <b>address</b>,
    ctx: &<b>mut</b> TxContext,
) {
    kanari_system::deny_list::deny_list_add&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;(denylist, denycap, sender, ctx);
}
</code></pre>



</details>

<a name="0x2_kanari_laps_coin_deny_list_remove_admin"></a>

## Function `deny_list_remove_admin`



<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_deny_list_remove_admin">deny_list_remove_admin</a>(denylist: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, denycap: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyCap">deny_list::DenyCap</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>&gt;, sender: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_deny_list_remove_admin">deny_list_remove_admin</a>(
    denylist: &<b>mut</b> DenyList,
    denycap: &<b>mut</b> DenyCap&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;,
    sender: <b>address</b>,
    ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>,
) {
    kanari_system::deny_list::deny_list_remove&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;(denylist, denycap, sender, ctx);
}
</code></pre>



</details>

<a name="0x2_kanari_laps_coin_destroy_zero"></a>

## Function `destroy_zero`



<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_destroy_zero">destroy_zero</a>(c: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>  entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_destroy_zero">destroy_zero</a>(
    c: Coin&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;
) {
    // Only allow destroying coins whose value is zero
    <b>let</b> v = <a href="coin.md#0x2_coin_value">coin::value</a>(&c);
    <b>assert</b>!(v == 0, 1);
    <b>let</b> _ = <a href="coin.md#0x2_coin_into_balance">coin::into_balance</a>(c);
}
</code></pre>



</details>

<a name="0x2_kanari_laps_coin_transfer"></a>

## Function `transfer`



<pre><code><b>public</b> entry <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>(c: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>&gt;, recipient: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>(c: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;, recipient: <b>address</b>) {
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(c, recipient);
}
</code></pre>



</details>

<a name="0x2_kanari_laps_coin_update_name"></a>

## Function `update_name`



<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_update_name">update_name</a>(_treasury: &<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>&gt;, metadata: &<b>mut</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_CoinMetadata">kanari_laps_coin::CoinMetadata</a>, name: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_update_name">update_name</a>(
    _treasury: &TreasuryCap&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;,
    metadata: &<b>mut</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_CoinMetadata">CoinMetadata</a>,
    name: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>
) {
    metadata.name = name;
}
</code></pre>



</details>

<a name="0x2_kanari_laps_coin_update_symbol"></a>

## Function `update_symbol`



<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_update_symbol">update_symbol</a>(_treasury: &<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>&gt;, metadata: &<b>mut</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_CoinMetadata">kanari_laps_coin::CoinMetadata</a>, symbol: <a href="dependencies/move-stdlib/ascii.md#0x1_ascii_String">ascii::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_update_symbol">update_symbol</a>(
    _treasury: &TreasuryCap&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;,
    metadata: &<b>mut</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_CoinMetadata">CoinMetadata</a>,
    symbol: <a href="dependencies/move-stdlib/ascii.md#0x1_ascii_String">ascii::String</a>
) {
    metadata.symbol = symbol;
}
</code></pre>



</details>

<a name="0x2_kanari_laps_coin_update_description"></a>

## Function `update_description`



<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_update_description">update_description</a>(_treasury: &<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>&gt;, metadata: &<b>mut</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_CoinMetadata">kanari_laps_coin::CoinMetadata</a>, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_update_description">update_description</a>(
    _treasury: &TreasuryCap&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;,
    metadata: &<b>mut</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_CoinMetadata">CoinMetadata</a>,
    description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>
) {
    metadata.description = description;
}
</code></pre>



</details>

<a name="0x2_kanari_laps_coin_update_icon_url"></a>

## Function `update_icon_url`



<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_update_icon_url">update_icon_url</a>(_treasury: &<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">kanari_laps_coin::COIN</a>&gt;, metadata: &<b>mut</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_CoinMetadata">kanari_laps_coin::CoinMetadata</a>, <a href="url.md#0x2_url">url</a>: <a href="dependencies/move-stdlib/ascii.md#0x1_ascii_String">ascii::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_update_icon_url">update_icon_url</a>(
    _treasury: &TreasuryCap&lt;<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_COIN">COIN</a>&gt;,
    metadata: &<b>mut</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_CoinMetadata">CoinMetadata</a>,
    <a href="url.md#0x2_url">url</a>: <a href="dependencies/move-stdlib/ascii.md#0x1_ascii_String">ascii::String</a>
) {
    metadata.icon_url = <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(<a href="url.md#0x2_url_new_unsafe">url::new_unsafe</a>(<a href="url.md#0x2_url">url</a>));
}
</code></pre>



</details>

<a name="0x2_kanari_laps_coin_get_decimals"></a>

## Function `get_decimals`



<pre><code><b>public</b> <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_get_decimals">get_decimals</a>(metadata: &<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_CoinMetadata">kanari_laps_coin::CoinMetadata</a>): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kanari_laps_coin.md#0x2_kanari_laps_coin_get_decimals">get_decimals</a>(metadata: &<a href="kanari_laps_coin.md#0x2_kanari_laps_coin_CoinMetadata">CoinMetadata</a>): u8 {
    metadata.decimals
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
