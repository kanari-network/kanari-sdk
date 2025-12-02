
<a name="0x2_kanari"></a>

# Module `0x2::kanari`

Coin<KANARI> is the token used to pay for gas in KANARI.
It has 9 decimals, and the smallest unit (10^-9) is called "MIST".


-  [Struct `KANARI`](#0x2_kanari_KANARI)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x2_kanari_new)
-  [Function `transfer`](#0x2_kanari_transfer)
-  [Function `burn`](#0x2_kanari_burn)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/ascii.md#0x1_ascii">0x1::ascii</a>;
<b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="dependencies/move-stdlib/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="coin.md#0x2_coin">0x2::coin</a>;
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


<a name="0x2_kanari_EAlreadyMinted"></a>



<pre><code><b>const</b> <a href="kanari.md#0x2_kanari_EAlreadyMinted">EAlreadyMinted</a>: u64 = 0;
</code></pre>



<a name="0x2_kanari_ENotSystemAddress"></a>

Sender is not @0x0 the system address.


<pre><code><b>const</b> <a href="kanari.md#0x2_kanari_ENotSystemAddress">ENotSystemAddress</a>: u64 = 1;
</code></pre>



<a name="0x2_kanari_MIST_PER_KANARI"></a>

The amount of Mist per Kanari token based on the fact that mist is
10^-9 of a Kanari token


<pre><code><b>const</b> <a href="kanari.md#0x2_kanari_MIST_PER_KANARI">MIST_PER_KANARI</a>: u64 = 1000000000;
</code></pre>



<a name="0x2_kanari_TOTAL_SUPPLY_KANARI"></a>

The total supply of Kanari denominated in whole Kanari tokens (100 Million)


<pre><code><b>const</b> <a href="kanari.md#0x2_kanari_TOTAL_SUPPLY_KANARI">TOTAL_SUPPLY_KANARI</a>: u64 = 100000000;
</code></pre>



<a name="0x2_kanari_TOTAL_SUPPLY_MIST"></a>

The total supply of Kanari denominated in Mist (100 Million * 10^9)


<pre><code><b>const</b> <a href="kanari.md#0x2_kanari_TOTAL_SUPPLY_MIST">TOTAL_SUPPLY_MIST</a>: u64 = 100000000000000000;
</code></pre>



<a name="0x2_kanari_new"></a>

## Function `new`



<pre><code><b>fun</b> <a href="kanari.md#0x2_kanari_new">new</a>(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;<a href="kanari.md#0x2_kanari_KANARI">kanari::KANARI</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="kanari.md#0x2_kanari_new">new</a>(ctx: &<b>mut</b> TxContext): TreasuryCap&lt;<a href="kanari.md#0x2_kanari_KANARI">KANARI</a>&gt; {
    <b>assert</b>!(<a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx) == @0x0, <a href="kanari.md#0x2_kanari_ENotSystemAddress">ENotSystemAddress</a>);
    <b>assert</b>!(<a href="tx_context.md#0x2_tx_context_epoch">tx_context::epoch</a>(ctx) == 0, <a href="kanari.md#0x2_kanari_EAlreadyMinted">EAlreadyMinted</a>);

    <b>let</b> (treasury, metadata) = <a href="coin.md#0x2_coin_create_currency">coin::create_currency</a>(
        <a href="kanari.md#0x2_kanari_KANARI">KANARI</a> {},
        9,
        <a href="dependencies/move-stdlib/ascii.md#0x1_ascii_string">ascii::string</a>(b"<a href="kanari.md#0x2_kanari_KANARI">KANARI</a>"),
        <a href="dependencies/move-stdlib/string.md#0x1_string_utf8">string::utf8</a>(b"Kanari Network Coin"),
        <a href="dependencies/move-stdlib/string.md#0x1_string_utf8">string::utf8</a>(b""),
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>(),
        ctx
    );
    <a href="transfer.md#0x2_transfer_public_freeze_object">transfer::public_freeze_object</a>(metadata);

    // make a mutable binding for minting (<b>use</b> a different name than the original)
    <b>let</b> treasury_cap = treasury;

    // Mint the entire supply (in Mist) and <a href="transfer.md#0x2_transfer">transfer</a> <b>to</b> dev @0x9
    <b>let</b> dev_address: <b>address</b> = @0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea;
    <b>let</b> minted_coin: Coin&lt;<a href="kanari.md#0x2_kanari_KANARI">KANARI</a>&gt; = <a href="coin.md#0x2_coin_mint">coin::mint</a>(&<b>mut</b> treasury_cap, <a href="kanari.md#0x2_kanari_TOTAL_SUPPLY_MIST">TOTAL_SUPPLY_MIST</a>, ctx);
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(minted_coin, dev_address);

    // Return the treasury cap for further authorized minting <b>if</b> needed
    treasury_cap
}
</code></pre>



</details>

<a name="0x2_kanari_transfer"></a>

## Function `transfer`

KANARI tokens to the treasury


<pre><code><b>public</b> entry <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>(c: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="kanari.md#0x2_kanari_KANARI">kanari::KANARI</a>&gt;, recipient: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>(c: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="kanari.md#0x2_kanari_KANARI">KANARI</a>&gt;, recipient: <b>address</b>) {
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(c, recipient)
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
