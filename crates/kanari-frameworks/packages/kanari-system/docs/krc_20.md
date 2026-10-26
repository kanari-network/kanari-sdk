
<a name="0x2_krc_20"></a>

# Module `0x2::krc_20`

KRC-20 is Kanari's fungible-token standard.

A token is identified by its Move witness type <code>T</code>; its value lives in
<code><a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;</code>, and mint authority lives exclusively in
<code><a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;</code>. This module deliberately adds no parallel balance
or supply ledger: <code><a href="coin.md#0x2_coin">coin</a></code> is the canonical implementation, including its
one-currency-per-witness check, overflow checks, and object ownership.


-  [Constants](#@Constants_0)
-  [Function `create`](#0x2_krc_20_create)
-  [Function `create_regulated`](#0x2_krc_20_create_regulated)
-  [Function `mint`](#0x2_krc_20_mint)
-  [Function `mint_to`](#0x2_krc_20_mint_to)
-  [Function `burn`](#0x2_krc_20_burn)
-  [Function `transfer`](#0x2_krc_20_transfer)
-  [Function `transfer_checked`](#0x2_krc_20_transfer_checked)
-  [Function `join`](#0x2_krc_20_join)
-  [Function `value`](#0x2_krc_20_value)
-  [Function `total_supply`](#0x2_krc_20_total_supply)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="coin.md#0x2_coin">0x2::coin</a>;
<b>use</b> <a href="deny_list.md#0x2_deny_list">0x2::deny_list</a>;
<b>use</b> <a href="pay.md#0x2_pay">0x2::pay</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
<b>use</b> <a href="url.md#0x2_url">0x2::url</a>;
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x2_krc_20_E_DENIED_OR_PAUSED"></a>

Sender, recipient, or token type is blocked by its deny-list policy.


<pre><code><b>const</b> <a href="krc_20.md#0x2_krc_20_E_DENIED_OR_PAUSED">E_DENIED_OR_PAUSED</a>: u64 = 0;
</code></pre>



<a name="0x2_krc_20_create"></a>

## Function `create`

Create the canonical KRC-20 currency for witness <code>T</code>.
<code><a href="coin.md#0x2_coin_create_currency">coin::create_currency</a></code> claims the witness before returning, so no
second treasury cap can be created for the same token type.


<pre><code><b>public</b> <b>fun</b> <a href="krc_20.md#0x2_krc_20_create">create</a>&lt;T: drop&gt;(witness: T, decimals: u8, symbol: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, description: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, icon_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): (<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, <a href="coin.md#0x2_coin_CoinMetadata">coin::CoinMetadata</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="krc_20.md#0x2_krc_20_create">create</a>&lt;T: drop&gt;(
    witness: T,
    decimals: u8,
    symbol: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    description: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    icon_url: Option&lt;Url&gt;,
    ctx: &<b>mut</b> TxContext
): (TreasuryCap&lt;T&gt;, CoinMetadata&lt;T&gt;) {
    <a href="coin.md#0x2_coin_create_currency">coin::create_currency</a>(
        witness,
        decimals,
        symbol,
        name,
        description,
        icon_url,
        ctx
    )
}
</code></pre>



</details>

<a name="0x2_krc_20_create_regulated"></a>

## Function `create_regulated`

Create a regulated KRC-20 token with an immediate deny list and a
global-pause capability. Transfers use <code>transfer_checked</code> for enforcement.


<pre><code><b>public</b> <b>fun</b> <a href="krc_20.md#0x2_krc_20_create_regulated">create_regulated</a>&lt;T: drop&gt;(witness: T, decimals: u8, symbol: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, description: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, icon_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;, allow_global_pause: bool, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): (<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, <a href="coin.md#0x2_coin_DenyCapV2">coin::DenyCapV2</a>&lt;T&gt;, <a href="coin.md#0x2_coin_CoinMetadata">coin::CoinMetadata</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="krc_20.md#0x2_krc_20_create_regulated">create_regulated</a>&lt;T: drop&gt;(
    witness: T,
    decimals: u8,
    symbol: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    description: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    icon_url: Option&lt;Url&gt;,
    allow_global_pause: bool,
    ctx: &<b>mut</b> TxContext
): (TreasuryCap&lt;T&gt;, DenyCapV2&lt;T&gt;, CoinMetadata&lt;T&gt;) {
    <a href="coin.md#0x2_coin_create_regulated_currency_v2">coin::create_regulated_currency_v2</a>(
        witness,
        decimals,
        symbol,
        name,
        description,
        icon_url,
        allow_global_pause,
        ctx
    )
}
</code></pre>



</details>

<a name="0x2_krc_20_mint"></a>

## Function `mint`

Mint a coin object. Only a holder of the token's treasury cap can mint.


<pre><code><b>public</b> <b>fun</b> <a href="krc_20.md#0x2_krc_20_mint">mint</a>&lt;T&gt;(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, amount: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="krc_20.md#0x2_krc_20_mint">mint</a>&lt;T&gt;(
    cap: &<b>mut</b> TreasuryCap&lt;T&gt;, amount: u64, ctx: &<b>mut</b> TxContext
): Coin&lt;T&gt; {
    <a href="coin.md#0x2_coin_mint">coin::mint</a>(cap, amount, ctx)
}
</code></pre>



</details>

<a name="0x2_krc_20_mint_to"></a>

## Function `mint_to`

Mint directly to an address. This is the issuer's airdrop primitive.


<pre><code><b>public</b> entry <b>fun</b> <a href="krc_20.md#0x2_krc_20_mint_to">mint_to</a>&lt;T&gt;(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, amount: u64, recipient: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="krc_20.md#0x2_krc_20_mint_to">mint_to</a>&lt;T&gt;(
    cap: &<b>mut</b> TreasuryCap&lt;T&gt;,
    amount: u64,
    recipient: <b>address</b>,
    ctx: &<b>mut</b> TxContext
) {
    <a href="coin.md#0x2_coin_mint_and_transfer">coin::mint_and_transfer</a>(cap, amount, recipient, ctx)
}
</code></pre>



</details>

<a name="0x2_krc_20_burn"></a>

## Function `burn`

Burn a complete coin object and reduce total supply.


<pre><code><b>public</b> <b>fun</b> <a href="krc_20.md#0x2_krc_20_burn">burn</a>&lt;T&gt;(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;, token: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="krc_20.md#0x2_krc_20_burn">burn</a>&lt;T&gt;(cap: &<b>mut</b> TreasuryCap&lt;T&gt;, token: Coin&lt;T&gt;): u64 {
    <a href="coin.md#0x2_coin_burn">coin::burn</a>(cap, token)
}
</code></pre>



</details>

<a name="0x2_krc_20_transfer"></a>

## Function `transfer`

Send an amount by splitting the sender-owned coin object.

The canonical KRC-20 path always consults the singleton deny list. For
ordinary tokens the lookup is empty and has no effect; regulated tokens
reject a denied sender, a denied recipient, or a global pause.


<pre><code><b>public</b> entry <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>&lt;T&gt;(token: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;, amount: u64, recipient: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>&lt;T&gt;(
    token: &<b>mut</b> Coin&lt;T&gt;, amount: u64, recipient: <b>address</b>, ctx: &<b>mut</b> TxContext
) {
    <b>let</b> sender = <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);
    <b>assert</b>!(!<a href="coin.md#0x2_coin_is_denied">coin::is_denied</a>&lt;T&gt;(sender), <a href="krc_20.md#0x2_krc_20_E_DENIED_OR_PAUSED">E_DENIED_OR_PAUSED</a>);
    <b>assert</b>!(!<a href="coin.md#0x2_coin_is_denied">coin::is_denied</a>&lt;T&gt;(recipient), <a href="krc_20.md#0x2_krc_20_E_DENIED_OR_PAUSED">E_DENIED_OR_PAUSED</a>);
    <b>assert</b>!(!<a href="coin.md#0x2_coin_is_paused">coin::is_paused</a>&lt;T&gt;(), <a href="krc_20.md#0x2_krc_20_E_DENIED_OR_PAUSED">E_DENIED_OR_PAUSED</a>);
    <a href="pay.md#0x2_pay_split_and_transfer">pay::split_and_transfer</a>(token, amount, recipient, ctx)
}
</code></pre>



</details>

<a name="0x2_krc_20_transfer_checked"></a>

## Function `transfer_checked`

Regulated KRC-20 transfer. Both sender and recipient are checked
against <code>deny</code>; a globally paused token also aborts.


<pre><code><b>public</b> entry <b>fun</b> <a href="krc_20.md#0x2_krc_20_transfer_checked">transfer_checked</a>&lt;T&gt;(token: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;, amount: u64, recipient: <b>address</b>, deny: &<a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="krc_20.md#0x2_krc_20_transfer_checked">transfer_checked</a>&lt;T&gt;(
    token: &<b>mut</b> Coin&lt;T&gt;,
    amount: u64,
    recipient: <b>address</b>,
    deny: &DenyList,
    ctx: &<b>mut</b> TxContext
) {
    <a href="pay.md#0x2_pay_split_and_transfer_checked">pay::split_and_transfer_checked</a>(token, amount, recipient, deny, ctx)
}
</code></pre>



</details>

<a name="0x2_krc_20_join"></a>

## Function `join`

Merge <code>other</code> into <code>token</code>, reducing coin-object fragmentation.


<pre><code><b>public</b> entry <b>fun</b> <a href="krc_20.md#0x2_krc_20_join">join</a>&lt;T&gt;(token: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;, other: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="krc_20.md#0x2_krc_20_join">join</a>&lt;T&gt;(token: &<b>mut</b> Coin&lt;T&gt;, other: Coin&lt;T&gt;) {
    <a href="coin.md#0x2_coin_join">coin::join</a>(token, other)
}
</code></pre>



</details>

<a name="0x2_krc_20_value"></a>

## Function `value`

Current base-unit balance of one coin object.


<pre><code><b>public</b> <b>fun</b> <a href="krc_20.md#0x2_krc_20_value">value</a>&lt;T&gt;(token: &<a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="krc_20.md#0x2_krc_20_value">value</a>&lt;T&gt;(token: &Coin&lt;T&gt;): u64 {
    <a href="coin.md#0x2_coin_value">coin::value</a>(token)
}
</code></pre>



</details>

<a name="0x2_krc_20_total_supply"></a>

## Function `total_supply`

Current circulating supply in base units.


<pre><code><b>public</b> <b>fun</b> <a href="krc_20.md#0x2_krc_20_total_supply">total_supply</a>&lt;T&gt;(cap: &<a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="krc_20.md#0x2_krc_20_total_supply">total_supply</a>&lt;T&gt;(cap: &TreasuryCap&lt;T&gt;): u64 {
    <a href="coin.md#0x2_coin_total_supply">coin::total_supply</a>(cap)
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
