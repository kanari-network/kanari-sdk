
<a name="0x2_kanari"></a>

# Module `0x2::kanari`

Coin<KANARI> is the token used to pay for gas in KANARI.
It has 9 decimals, and the smallest unit (10^-9) is called "MIST".


-  [Struct `KANARI`](#0x2_kanari_KANARI)
-  [Constants](#@Constants_0)
-  [Function `init`](#0x2_kanari_init)
-  [Function `transfer`](#0x2_kanari_transfer)
-  [Function `burn`](#0x2_kanari_burn)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="coin.md#0x2_coin">0x2::coin</a>;
<b>use</b> <a href="pay.md#0x2_pay">0x2::pay</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
<b>use</b> <a href="url.md#0x2_url">0x2::url</a>;
</code></pre>



<a name="0x2_kanari_KANARI"></a>

## Struct `KANARI`

Name of the coin


<pre><code><b>struct</b> <a href="kanari.md#0x2_kanari_KANARI">KANARI</a> <b>has</b> drop
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


<a name="0x2_kanari_DEV_GAS_RESERVE_MIST"></a>



<pre><code><b>const</b> <a href="kanari.md#0x2_kanari_DEV_GAS_RESERVE_MIST">DEV_GAS_RESERVE_MIST</a>: u64 = 1000000000;
</code></pre>



<a name="0x2_kanari_MIST_PER_KANARI"></a>

The amount of Mist per Kanari token based on the fact that mist is
10^-9 of a Kanari token


<pre><code><b>const</b> <a href="kanari.md#0x2_kanari_MIST_PER_KANARI">MIST_PER_KANARI</a>: u64 = 1000000000;
</code></pre>



<a name="0x2_kanari_TOTAL_SUPPLY_KANARI"></a>

The total supply of Kanari denominated in whole Kanari tokens (11 Million)


<pre><code><b>const</b> <a href="kanari.md#0x2_kanari_TOTAL_SUPPLY_KANARI">TOTAL_SUPPLY_KANARI</a>: u64 = 11000000;
</code></pre>



<a name="0x2_kanari_TOTAL_SUPPLY_MIST"></a>

The total supply of Kanari denominated in Mist (11 Million * 10^9)


<pre><code><b>const</b> <a href="kanari.md#0x2_kanari_TOTAL_SUPPLY_MIST">TOTAL_SUPPLY_MIST</a>: u64 = 11000000000000000;
</code></pre>



<a name="0x2_kanari_init"></a>

## Function `init`



<pre><code><b>fun</b> <a href="kanari.md#0x2_kanari_init">init</a>(witness: <a href="kanari.md#0x2_kanari_KANARI">kanari::KANARI</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="kanari.md#0x2_kanari_init">init</a>(witness: <a href="kanari.md#0x2_kanari_KANARI">KANARI</a>, ctx: &<b>mut</b> TxContext) {
    // <b>assert</b>!(<a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx) == @0x0, ENotSystemAddress); // Sender check might be too strict for init
    // <b>assert</b>!(<a href="tx_context.md#0x2_tx_context_epoch">tx_context::epoch</a>(ctx) == 0, EAlreadyMinted); // Epoch check might be okay

    <b>let</b> (treasury, metadata) = <a href="coin.md#0x2_coin_create_currency">coin::create_currency</a>(
        witness,
        9,
        b"<a href="kanari.md#0x2_kanari_KANARI">KANARI</a>",
        b"Kanari Network Coin",
        b"",
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(<a href="url.md#0x2_url_new_unsafe_from_bytes">url::new_unsafe_from_bytes</a>(b"https://avatars.githubusercontent.com/u/127471673?s=200&v=4")),
        ctx
    );
    <a href="transfer.md#0x2_transfer_public_freeze_object">transfer::public_freeze_object</a>(metadata);

    // make a mutable binding for minting (<b>use</b> a different name than the original)
    <b>let</b> treasury_cap = treasury;

    // Mint the initial supply into two <a href="coin.md#0x2_coin">coin</a> objects so the dev wallet <b>has</b>
    // a dedicated gas <a href="coin.md#0x2_coin">coin</a> from genesis onward.
    <b>let</b> dev_address: <b>address</b> = @0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146;
    <b>let</b> primary_coin: Coin&lt;<a href="kanari.md#0x2_kanari_KANARI">KANARI</a>&gt; =
        <a href="coin.md#0x2_coin_mint">coin::mint</a>(&<b>mut</b> treasury_cap, <a href="kanari.md#0x2_kanari_TOTAL_SUPPLY_MIST">TOTAL_SUPPLY_MIST</a> - <a href="kanari.md#0x2_kanari_DEV_GAS_RESERVE_MIST">DEV_GAS_RESERVE_MIST</a>, ctx);
    <b>let</b> gas_coin: Coin&lt;<a href="kanari.md#0x2_kanari_KANARI">KANARI</a>&gt; = <a href="coin.md#0x2_coin_mint">coin::mint</a>(&<b>mut</b> treasury_cap, <a href="kanari.md#0x2_kanari_DEV_GAS_RESERVE_MIST">DEV_GAS_RESERVE_MIST</a>, ctx);
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(primary_coin, dev_address);
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(gas_coin, dev_address);

    // Transfer the treasury cap <b>to</b> the sender (deployer)
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(treasury_cap, <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx));
}
</code></pre>



</details>

<a name="0x2_kanari_transfer"></a>

## Function `transfer`

Transfer a specific amount of KANARI using the same Move coin path as other tokens.


<pre><code><b>public</b> entry <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>(c: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="kanari.md#0x2_kanari_KANARI">kanari::KANARI</a>&gt;, amount: u64, recipient: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>(
    c: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="kanari.md#0x2_kanari_KANARI">KANARI</a>&gt;,
    amount: u64,
    recipient: <b>address</b>,
    ctx: &<b>mut</b> TxContext
) {
    <a href="pay.md#0x2_pay_split_and_transfer">pay::split_and_transfer</a>&lt;<a href="kanari.md#0x2_kanari_KANARI">KANARI</a>&gt;(c, amount, recipient, ctx);
}
</code></pre>



</details>

<a name="0x2_kanari_burn"></a>

## Function `burn`

Burns KANARI tokens, decreasing total supply


<pre><code><b>public</b> entry <b>fun</b> <a href="kanari.md#0x2_kanari_burn">burn</a>(treasury_cap: &<b>mut</b> <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;<a href="kanari.md#0x2_kanari_KANARI">kanari::KANARI</a>&gt;, <a href="coin.md#0x2_coin">coin</a>: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="kanari.md#0x2_kanari_KANARI">kanari::KANARI</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="kanari.md#0x2_kanari_burn">burn</a>(treasury_cap: &<b>mut</b> TreasuryCap&lt;<a href="kanari.md#0x2_kanari_KANARI">KANARI</a>&gt;, <a href="coin.md#0x2_coin">coin</a>: Coin&lt;<a href="kanari.md#0x2_kanari_KANARI">KANARI</a>&gt;) {
    <a href="coin.md#0x2_coin_burn">coin::burn</a>(treasury_cap, <a href="coin.md#0x2_coin">coin</a>);
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
