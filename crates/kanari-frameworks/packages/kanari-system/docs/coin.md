
<a name="0x2_coin"></a>

# Module `0x2::coin`



-  [Struct `Coin`](#0x2_coin_Coin)
-  [Struct `TreasuryCap`](#0x2_coin_TreasuryCap)
-  [Struct `Treasury`](#0x2_coin_Treasury)
-  [Resource `CoinMetadata`](#0x2_coin_CoinMetadata)
-  [Constants](#@Constants_0)
-  [Function `create_currency`](#0x2_coin_create_currency)
-  [Function `mint`](#0x2_coin_mint)
-  [Function `mint_and_transfer`](#0x2_coin_mint_and_transfer)
-  [Function `burn`](#0x2_coin_burn)
-  [Function `into_balance`](#0x2_coin_into_balance)
-  [Function `from_balance`](#0x2_coin_from_balance)
-  [Function `total_supply`](#0x2_coin_total_supply)
-  [Function `value`](#0x2_coin_value)
-  [Function `split`](#0x2_coin_split)
-  [Function `join`](#0x2_coin_join)
-  [Function `treasury_into_supply`](#0x2_coin_treasury_into_supply)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/ascii.md#0x1_ascii">0x1::ascii</a>;
<b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="dependencies/move-stdlib/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="balance.md#0x2_balance">0x2::balance</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
<b>use</b> <a href="url.md#0x2_url">0x2::url</a>;
</code></pre>



<a name="0x2_coin_Coin"></a>

## Struct `Coin`

Coin resource wrapper with balance


<pre><code><b>struct</b> <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt; <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="balance.md#0x2_balance">balance</a>: <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_coin_TreasuryCap"></a>

## Struct `TreasuryCap`

Capability allowing the bearer to mint and burn coins


<pre><code><b>struct</b> <a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt; <b>has</b> drop, store
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

<a name="0x2_coin_Treasury"></a>

## Struct `Treasury`

Treasury: holds authority to mint into a Supply (deprecated, use TreasuryCap)


<pre><code><b>struct</b> <a href="coin.md#0x2_coin_Treasury">Treasury</a>&lt;T&gt; <b>has</b> drop, store
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

<a name="0x2_coin_CoinMetadata"></a>

## Resource `CoinMetadata`

Metadata resource for a currency (stored as an object with UID)


<pre><code><b>struct</b> <a href="coin.md#0x2_coin_CoinMetadata">CoinMetadata</a>&lt;T&gt; <b>has</b> drop, store, key
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



<pre><code><b>const</b> <a href="coin.md#0x2_coin_EINVALID_DECIMALS">EINVALID_DECIMALS</a>: u64 = 5;
</code></pre>



<a name="0x2_coin_EOVERFLOW"></a>



<pre><code><b>const</b> <a href="coin.md#0x2_coin_EOVERFLOW">EOVERFLOW</a>: u64 = 2;
</code></pre>



<a name="0x2_coin_EUNDERFLOW"></a>



<pre><code><b>const</b> <a href="coin.md#0x2_coin_EUNDERFLOW">EUNDERFLOW</a>: u64 = 3;
</code></pre>



<a name="0x2_coin_EZERO_AMOUNT"></a>



<pre><code><b>const</b> <a href="coin.md#0x2_coin_EZERO_AMOUNT">EZERO_AMOUNT</a>: u64 = 1;
</code></pre>



<a name="0x2_coin_create_currency"></a>

## Function `create_currency`

Create a new currency with TreasuryCap for minting control and return the
TreasuryCap and the Metadata object. Callers may transfer/freeze the
returned objects as appropriate for their use-case.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_create_currency">create_currency</a>&lt;T: drop&gt;(witness: T, decimals: u8, symbol: <a href="dependencies/move-stdlib/ascii.md#0x1_ascii_String">ascii::String</a>, name: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, icon_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): (<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, <a href="coin.md#0x2_coin_CoinMetadata">coin::CoinMetadata</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_create_currency">create_currency</a>&lt;T: drop&gt;(
    witness: T,
    decimals: u8,
    symbol: <a href="dependencies/move-stdlib/ascii.md#0x1_ascii_String">ascii::String</a>,
    name: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
    description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
    icon_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;,
    ctx: &<b>mut</b> TxContext,
): (<a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt;, <a href="coin.md#0x2_coin_CoinMetadata">CoinMetadata</a>&lt;T&gt;) {
    // 1. Consume the witness type
    <b>let</b> _ = witness;

    // Basic safety checks for decimals (Move's u8 can hold max 255, but typically &lt; 27 for real-world)
    // We'll set a soft limit based on common standards.
    <b>assert</b>!(decimals &lt;= 27, <a href="coin.md#0x2_coin_EINVALID_DECIMALS">EINVALID_DECIMALS</a>);

    // 2. Create the Capability and Metadata, explicitly specifying the generic type T
    <b>let</b> treasury_cap = <a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt; { id: <a href="object.md#0x2_object_new">object::new</a>(ctx), total_supply: 0 };
    <b>let</b> metadata = <a href="coin.md#0x2_coin_CoinMetadata">CoinMetadata</a>&lt;T&gt; {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        decimals,
        name,
        symbol,
        description,
        icon_url
    };

    // Return the newly-created capability and metadata. Callers decide how
    // <b>to</b> distribute or <b>freeze</b> them (e.g., <a href="transfer.md#0x2_transfer">transfer</a> the cap <b>to</b> an <b>address</b>
    // or <b>freeze</b> the metadata for <b>public</b> visibility).
    (treasury_cap, metadata)
}
</code></pre>



</details>

<a name="0x2_coin_mint"></a>

## Function `mint`

Mint new coins using TreasuryCap
Returns the newly minted Coin<T>.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_mint">mint</a>&lt;T&gt;(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, amount: u64, _ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_mint">mint</a>&lt;T&gt;(
    cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt;,
    amount: u64,
    _ctx: &<b>mut</b> TxContext,
): <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt; {
    <b>assert</b>!(amount &gt; 0, <a href="coin.md#0x2_coin_EZERO_AMOUNT">EZERO_AMOUNT</a>);
    <b>let</b> new_total = cap.total_supply + amount;
    <b>assert</b>!(new_total &gt;= cap.total_supply, <a href="coin.md#0x2_coin_EOVERFLOW">EOVERFLOW</a>); // Check for overflow

    cap.total_supply = new_total;

    <a href="coin.md#0x2_coin_Coin">Coin</a> {
        <a href="balance.md#0x2_balance">balance</a>: <a href="balance.md#0x2_balance_create">balance::create</a>(amount),
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
    <b>let</b> <a href="coin.md#0x2_coin_Coin">Coin</a> { <a href="balance.md#0x2_balance">balance</a> } = <a href="coin.md#0x2_coin">coin</a>;
    <b>let</b> value = <a href="balance.md#0x2_balance_destroy">balance::destroy</a>(<a href="balance.md#0x2_balance">balance</a>);

    <b>assert</b>!(cap.total_supply &gt;= value, <a href="coin.md#0x2_coin_EUNDERFLOW">EUNDERFLOW</a>); // Check for underflow

    cap.total_supply = cap.total_supply - value;
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
    <b>let</b> <a href="coin.md#0x2_coin_Coin">Coin</a> { <a href="balance.md#0x2_balance">balance</a> } = <a href="coin.md#0x2_coin">coin</a>;
    <a href="balance.md#0x2_balance">balance</a>
}
</code></pre>



</details>

<a name="0x2_coin_from_balance"></a>

## Function `from_balance`

Construct a <code><a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt;</code> from a <code>Balance&lt;T&gt;</code>.
This helper allows other modules to wrap balances into Coin objects
when they take custody of raw balances (e.g., DEX pools).


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_from_balance">from_balance</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;): <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_from_balance">from_balance</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: Balance&lt;T&gt;): <a href="coin.md#0x2_coin_Coin">Coin</a>&lt;T&gt; {
    <a href="coin.md#0x2_coin_Coin">Coin</a> { <a href="balance.md#0x2_balance">balance</a> }
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
    <a href="balance.md#0x2_balance_value">balance::value</a>(&<a href="coin.md#0x2_coin">coin</a>.<a href="balance.md#0x2_balance">balance</a>)
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
    // Assert for sufficient <a href="balance.md#0x2_balance">balance</a> is implicitly handled by <a href="balance.md#0x2_balance_split">balance::split</a>
    <b>let</b> _ = ctx;
    <a href="coin.md#0x2_coin_Coin">Coin</a> {
        <a href="balance.md#0x2_balance">balance</a>: <a href="balance.md#0x2_balance_split">balance::split</a>(&<b>mut</b> <a href="coin.md#0x2_coin">coin</a>.<a href="balance.md#0x2_balance">balance</a>, amount),
    }
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
    <b>let</b> <a href="coin.md#0x2_coin_Coin">Coin</a> { <a href="balance.md#0x2_balance">balance</a> } = other;
    <a href="balance.md#0x2_balance_merge">balance::merge</a>(&<b>mut</b> <a href="coin.md#0x2_coin">coin</a>.<a href="balance.md#0x2_balance">balance</a>, <a href="balance.md#0x2_balance">balance</a>);
}
</code></pre>



</details>

<a name="0x2_coin_treasury_into_supply"></a>

## Function `treasury_into_supply`

Deprecated: Convert a treasury (or treasury cap) into a supply handle.
In modern Kanari/Move systems, the total supply is tracked either in the TreasuryCap
or in a separate Supply object that is shared upon creation.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_treasury_into_supply">treasury_into_supply</a>&lt;T&gt;(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;): <a href="balance.md#0x2_balance_Supply">balance::Supply</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x2_coin_treasury_into_supply">treasury_into_supply</a>&lt;T&gt;(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">TreasuryCap</a>&lt;T&gt;): Supply&lt;T&gt; {
    <b>let</b> _ = cap;
    <a href="balance.md#0x2_balance_new_supply">balance::new_supply</a>&lt;T&gt;() // Assumes new_supply is still needed for compatibility
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
