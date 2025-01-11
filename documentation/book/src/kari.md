
<a name="0x2_kari"></a>

 Module `0x2::kari`

Coin<KARI> is the token used to pay for gas in Kari.
It has 9 decimals, and the smallest unit (10^-9) is called "mist".


-  [Struct `KARI`](#0x2_kari_KARI)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x2_kari_new)
-  [Function `transfer`](#0x2_kari_transfer)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="balance.md#0x2_balance">0x2::balance</a>;
<b>use</b> <a href="coin.md#0x2_coin">0x2::coin</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
<b>use</b> <a href="url.md#0x2_url">0x2::url</a>;
</code></pre>



<a name="0x2_kari_KARI"></a>

# Struct `KARI`

Name of the coin


<pre><code><b>struct</b> <a href="kari.md#0x2_kari_KARI">KARI</a> <b>has</b> drop
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

# Constants


<a name="0x2_kari_ENotSystemAddress"></a>

Sender is not @0x0 the system address.


<pre><code><b>const</b> <a href="kari.md#0x2_kari_ENotSystemAddress">ENotSystemAddress</a>: u64 = 1;
</code></pre>



<a name="0x2_kari_EAlreadyMinted"></a>



<pre><code><b>const</b> <a href="kari.md#0x2_kari_EAlreadyMinted">EAlreadyMinted</a>: u64 = 0;
</code></pre>



<a name="0x2_kari_MIST_PER_KARI"></a>

The amount of Mist per Kari token based on the the fact that mist is
10^-9 of a Kari token


<pre><code><b>const</b> <a href="kari.md#0x2_kari_MIST_PER_KARI">MIST_PER_KARI</a>: u64 = 1000000000;
</code></pre>



<a name="0x2_kari_TOTAL_SUPPLY_KARI"></a>

The total supply of Kari denominated in whole Kari tokens (10 Billion)


<pre><code><b>const</b> <a href="kari.md#0x2_kari_TOTAL_SUPPLY_KARI">TOTAL_SUPPLY_KARI</a>: u64 = 10000000000;
</code></pre>



<a name="0x2_kari_TOTAL_SUPPLY_MIST"></a>

The total supply of Kari denominated in Mist (10 Billion * 10^9)


<pre><code><b>const</b> <a href="kari.md#0x2_kari_TOTAL_SUPPLY_MIST">TOTAL_SUPPLY_MIST</a>: u64 = 200000000000000000;
</code></pre>



<a name="0x2_kari_new"></a>

# Function `new`



<pre><code><b>fun</b> <a href="kari.md#0x2_kari_new">new</a>(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;<a href="kari.md#0x2_kari_KARI">kari::KARI</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="kari.md#0x2_kari_new">new</a>(ctx: &<b>mut</b> TxContext): Balance&lt;<a href="kari.md#0x2_kari_KARI">KARI</a>&gt; {
    <b>assert</b>!(<a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx) == @0x0, <a href="kari.md#0x2_kari_ENotSystemAddress">ENotSystemAddress</a>);
    <b>assert</b>!(<a href="tx_context.md#0x2_tx_context_epoch">tx_context::epoch</a>(ctx) == 0, <a href="kari.md#0x2_kari_EAlreadyMinted">EAlreadyMinted</a>);

    <b>let</b> (treasury, metadata) = <a href="coin.md#0x2_coin_create_currency">coin::create_currency</a>(
        <a href="kari.md#0x2_kari_KARI">KARI</a> {},
        9,
        b"<a href="kari.md#0x2_kari_KARI">KARI</a>",
        b"Karura Network Coin",
        // TODO: add appropriate description and logo <a href="url.md#0x2_url">url</a>
        b"",
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>(),
        ctx
    );
    <a href="transfer.md#0x2_transfer_public_freeze_object">transfer::public_freeze_object</a>(metadata);
    <b>let</b> supply = <a href="coin.md#0x2_coin_treasury_into_supply">coin::treasury_into_supply</a>(treasury);
    <b>let</b> total_kari = <a href="balance.md#0x2_balance_increase_supply">balance::increase_supply</a>(&<b>mut</b> supply, <a href="kari.md#0x2_kari_TOTAL_SUPPLY_MIST">TOTAL_SUPPLY_MIST</a>);
    <a href="balance.md#0x2_balance_destroy_supply">balance::destroy_supply</a>(supply);
    total_kari
}
</code></pre>



</details>

<a name="0x2_kari_transfer"></a>

# Function `transfer`



<pre><code><b>public</b> entry <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>(c: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="kari.md#0x2_kari_KARI">kari::KARI</a>&gt;, recipient: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>(c: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="kari.md#0x2_kari_KARI">KARI</a>&gt;, recipient: <b>address</b>) {
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(c, recipient)
}
</code></pre>



</details>
