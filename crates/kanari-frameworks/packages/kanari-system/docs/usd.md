
<a name="0x2_usd"></a>

# Module `0x2::usd`

USD stablecoin for the Kanari network (<code><a href="usd.md#0x2_usd_USD">0x2::usd::USD</a></code>).

- 6 decimals (USDC-style); the smallest unit is 10^-6 USD.
- Whitelisted for gas settlement (see <code>kanari_system::gas_market</code> and the
<code>GasPriceTable</code> client default of $1.00).
- Permissionless mint: the <code>TreasuryCap</code> is a shared object, so anyone
can call <code>mint</code> with their own wallet (paying their own gas). No fixed
supply, no dev allocation, no custodian key or password.

Anyone with a funded wallet mints with one call, e.g.:
<code><a href="kanari.md#0x2_kanari">kanari</a> <b>move</b> call --package 0x2 --<b>module</b> <a href="usd.md#0x2_usd">usd</a> --function mint \
--args &lt;treasury_cap_object_id&gt; 100000000 &lt;recipient&gt;</code>


-  [Struct `USD`](#0x2_usd_USD)
-  [Constants](#@Constants_0)
-  [Function `init`](#0x2_usd_init)
-  [Function `mint`](#0x2_usd_mint)
-  [Function `transfer_amount`](#0x2_usd_transfer_amount)
-  [Function `burn_amount`](#0x2_usd_burn_amount)
-  [Function `units_per_usd`](#0x2_usd_units_per_usd)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="coin.md#0x2_coin">0x2::coin</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
<b>use</b> <a href="url.md#0x2_url">0x2::url</a>;
</code></pre>



<a name="0x2_usd_USD"></a>

## Struct `USD`

Name of the coin


<pre><code><b>struct</b> <a href="usd.md#0x2_usd_USD">USD</a> <b>has</b> drop
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

<a name="@Constants_0"></a>

## Constants


<a name="0x2_usd_UNITS_PER_USD"></a>

Base units per whole USD (6 decimals).


<pre><code><b>const</b> <a href="usd.md#0x2_usd_UNITS_PER_USD">UNITS_PER_USD</a>: u64 = 1000000;
</code></pre>



<a name="0x2_usd_init"></a>

## Function `init`



<pre><code><b>fun</b> <a href="usd.md#0x2_usd_init">init</a>(witness: <a href="usd.md#0x2_usd_USD">usd::USD</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="usd.md#0x2_usd_init">init</a>(witness: <a href="usd.md#0x2_usd_USD">USD</a>, ctx: &<b>mut</b> TxContext) {
    <b>let</b> (treasury, metadata) =
        <a href="coin.md#0x2_coin_create_currency">coin::create_currency</a>(
            witness,
            6,
            b"<a href="usd.md#0x2_usd_USD">USD</a>",
            b"Kanari <a href="usd.md#0x2_usd_USD">USD</a> Stablecoin",
            b"Permissionless <a href="usd.md#0x2_usd_USD">USD</a>: anyone can mint via <a href="usd.md#0x2_usd_mint">usd::mint</a> <b>with</b> the shared TreasuryCap",
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(
                <a href="url.md#0x2_url_new_unsafe_from_bytes">url::new_unsafe_from_bytes</a>(
                    b"https://avatars.githubusercontent.com/u/127471673?s=200&v=4"
                )
            ),
            ctx
        );
    <a href="transfer.md#0x2_transfer_public_freeze_object">transfer::public_freeze_object</a>(metadata);

    // The mint authority is shared, not held: no key, no password.
    <a href="transfer.md#0x2_transfer_share_object">transfer::share_object</a>(treasury);
}
</code></pre>



</details>

<a name="0x2_usd_mint"></a>

## Function `mint`

Mint new USD to <code>recipient</code>. Permissionless: pass the shared
<code>TreasuryCap&lt;<a href="usd.md#0x2_usd_USD">USD</a>&gt;</code> (discoverable via object query by type).


<pre><code><b>public</b> entry <b>fun</b> <a href="usd.md#0x2_usd_mint">mint</a>(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;<a href="usd.md#0x2_usd_USD">usd::USD</a>&gt;, amount: u64, recipient: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="usd.md#0x2_usd_mint">mint</a>(
    cap: &<b>mut</b> TreasuryCap&lt;<a href="usd.md#0x2_usd_USD">USD</a>&gt;,
    amount: u64,
    recipient: <b>address</b>,
    ctx: &<b>mut</b> TxContext,
) {
    <b>let</b> minted = <a href="coin.md#0x2_coin_mint">coin::mint</a>(cap, amount, ctx);
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(minted, recipient);
}
</code></pre>



</details>

<a name="0x2_usd_transfer_amount"></a>

## Function `transfer_amount`

Transfer a specific <code>amount</code> of USD from a mutable Coin held by the caller.


<pre><code><b>public</b> entry <b>fun</b> <a href="usd.md#0x2_usd_transfer_amount">transfer_amount</a>(c: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="usd.md#0x2_usd_USD">usd::USD</a>&gt;, amount: u64, recipient: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="usd.md#0x2_usd_transfer_amount">transfer_amount</a>(
    c: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="usd.md#0x2_usd_USD">USD</a>&gt;,
    amount: u64,
    recipient: <b>address</b>,
    ctx: &<b>mut</b> TxContext,
) {
    <b>let</b> sender = <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);
    <b>if</b> (sender == recipient) { <b>return</b> };
    <b>let</b> split_coin = <a href="coin.md#0x2_coin_split">coin::split</a>(c, amount, ctx);
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(split_coin, recipient);
}
</code></pre>



</details>

<a name="0x2_usd_burn_amount"></a>

## Function `burn_amount`

Burn a specific <code>amount</code> of USD from a mutable Coin held by the caller.


<pre><code><b>public</b> entry <b>fun</b> <a href="usd.md#0x2_usd_burn_amount">burn_amount</a>(cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;<a href="usd.md#0x2_usd_USD">usd::USD</a>&gt;, c: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="usd.md#0x2_usd_USD">usd::USD</a>&gt;, amount: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="usd.md#0x2_usd_burn_amount">burn_amount</a>(
    cap: &<b>mut</b> TreasuryCap&lt;<a href="usd.md#0x2_usd_USD">USD</a>&gt;,
    c: &<b>mut</b> Coin&lt;<a href="usd.md#0x2_usd_USD">USD</a>&gt;,
    amount: u64,
    ctx: &<b>mut</b> TxContext,
) {
    <b>let</b> to_burn = <a href="coin.md#0x2_coin_split">coin::split</a>(c, amount, ctx);
    <b>let</b> _burned = <a href="coin.md#0x2_coin_burn">coin::burn</a>(cap, to_burn);
}
</code></pre>



</details>

<a name="0x2_usd_units_per_usd"></a>

## Function `units_per_usd`



<pre><code><b>public</b> <b>fun</b> <a href="usd.md#0x2_usd_units_per_usd">units_per_usd</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="usd.md#0x2_usd_units_per_usd">units_per_usd</a>(): u64 {
    <a href="usd.md#0x2_usd_UNITS_PER_USD">UNITS_PER_USD</a>
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
