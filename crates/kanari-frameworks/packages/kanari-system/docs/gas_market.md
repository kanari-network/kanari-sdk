
<a name="0x2_gas_market"></a>

# Module `0x2::gas_market`

USD-numeraire gas market with multi-coin settlement.

Design (Phase 1):
- Fees are *quoted* in USD (micro-USD, 6 decimals) so wallets can show a
stable price: <code>usd_fee = gas_units * usd_per_gas_unit</code>.
- Fees are *settled* in any whitelisted <code>Coin&lt;T&gt;</code> the sender owns, including
the native <code><a href="kanari.md#0x2_kanari_KANARI">0x2::kanari::KANARI</a></code>. Conversion uses integer math only and
always rounds up so validators never under-collect.
- <code><a href="gas_market.md#0x2_gas_market_PriceTable">PriceTable</a></code> is an on-chain shared object. Every validator quotes with the
same committed version, keeping checkpoints deterministic. Transactions
pin <code>price_version</code> and a <code>max_price_per_unit</code> slippage cap; the backend
(see <code><a href="kanari.md#0x2_kanari">kanari</a>-types::gas_market</code>) mirrors this math in Rust.

Price encoding: <code>price_usd_micros</code> = USD value of ONE whole token, scaled by
1e6. Example: KANARI at $2.00 -> 2_000_000.


-  [Resource `AdminCap`](#0x2_gas_market_AdminCap)
-  [Resource `FeederCap`](#0x2_gas_market_FeederCap)
-  [Struct `GasCoinConfig`](#0x2_gas_market_GasCoinConfig)
-  [Struct `PriceTick`](#0x2_gas_market_PriceTick)
-  [Resource `PriceTable`](#0x2_gas_market_PriceTable)
-  [Constants](#@Constants_0)
-  [Function `init`](#0x2_gas_market_init)
-  [Function `quote_usd_fee_micros`](#0x2_gas_market_quote_usd_fee_micros)
-  [Function `usd_to_token_amount`](#0x2_gas_market_usd_to_token_amount)
-  [Function `quote_in_token`](#0x2_gas_market_quote_in_token)
-  [Function `pow10`](#0x2_gas_market_pow10)
-  [Function `register_gas_coin`](#0x2_gas_market_register_gas_coin)
-  [Function `set_coin_active`](#0x2_gas_market_set_coin_active)
-  [Function `set_usd_per_gas_unit`](#0x2_gas_market_set_usd_per_gas_unit)
-  [Function `publish_prices`](#0x2_gas_market_publish_prices)
-  [Function `quote_for_coin`](#0x2_gas_market_quote_for_coin)
-  [Function `find_config`](#0x2_gas_market_find_config)
-  [Function `is_supported_gas_coin`](#0x2_gas_market_is_supported_gas_coin)
-  [Function `version`](#0x2_gas_market_version)
-  [Function `usd_per_gas_unit`](#0x2_gas_market_usd_per_gas_unit)
-  [Function `num_coins`](#0x2_gas_market_num_coins)
-  [Function `usd_scale`](#0x2_gas_market_usd_scale)
-  [Function `assert_authorized`](#0x2_gas_market_assert_authorized)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="math.md#0x2_math">0x2::math</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
</code></pre>



<a name="0x2_gas_market_AdminCap"></a>

## Resource `AdminCap`

Capability for whitelist + base-fee governance (held by DAO timelock).


<pre><code><b>struct</b> <a href="gas_market.md#0x2_gas_market_AdminCap">AdminCap</a> <b>has</b> store, key
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

<a name="0x2_gas_market_FeederCap"></a>

## Resource `FeederCap`

Capability for publishing price ticks (held by feeder multisig;
Phase 2 migrates to a validator-median vote).


<pre><code><b>struct</b> <a href="gas_market.md#0x2_gas_market_FeederCap">FeederCap</a> <b>has</b> store, key
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

<a name="0x2_gas_market_GasCoinConfig"></a>

## Struct `GasCoinConfig`

One whitelisted gas coin.


<pre><code><b>struct</b> <a href="gas_market.md#0x2_gas_market_GasCoinConfig">GasCoinConfig</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>coin_type: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>decimals: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>active: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_gas_market_PriceTick"></a>

## Struct `PriceTick`

USD price tick for the coin at the same index in <code>configs</code>.


<pre><code><b>struct</b> <a href="gas_market.md#0x2_gas_market_PriceTick">PriceTick</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>price_usd_micros: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>updated_at_ms: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>checkpoint_seq: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_gas_market_PriceTable"></a>

## Resource `PriceTable`

Shared oracle + fee-policy object.


<pre><code><b>struct</b> <a href="gas_market.md#0x2_gas_market_PriceTable">PriceTable</a> <b>has</b> store, key
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
<code>version: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>usd_per_gas_unit_micros: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>configs: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="gas_market.md#0x2_gas_market_GasCoinConfig">gas_market::GasCoinConfig</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>prices: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="gas_market.md#0x2_gas_market_PriceTick">gas_market::PriceTick</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>max_staleness_checkpoints: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_gas_market_EINVALID_DECIMALS"></a>

Coin decimals exceed the 9-decimal ceiling.


<pre><code><b>const</b> <a href="gas_market.md#0x2_gas_market_EINVALID_DECIMALS">EINVALID_DECIMALS</a>: u64 = 8;
</code></pre>



<a name="0x2_gas_market_DEFAULT_USD_PER_GAS_UNIT"></a>

Default USD price per gas unit: $0.000001 (1 micro-USD).


<pre><code><b>const</b> <a href="gas_market.md#0x2_gas_market_DEFAULT_USD_PER_GAS_UNIT">DEFAULT_USD_PER_GAS_UNIT</a>: u64 = 1;
</code></pre>



<a name="0x2_gas_market_ELENGTH_MISMATCH"></a>

Batched vector arguments have different lengths.


<pre><code><b>const</b> <a href="gas_market.md#0x2_gas_market_ELENGTH_MISMATCH">ELENGTH_MISMATCH</a>: u64 = 7;
</code></pre>



<a name="0x2_gas_market_ENOT_AUTHORIZED"></a>

Caller is not the admin/feeder.


<pre><code><b>const</b> <a href="gas_market.md#0x2_gas_market_ENOT_AUTHORIZED">ENOT_AUTHORIZED</a>: u64 = 5;
</code></pre>



<a name="0x2_gas_market_EOVERFLOW_QUOTE"></a>

Quote arithmetic overflowed u64.


<pre><code><b>const</b> <a href="gas_market.md#0x2_gas_market_EOVERFLOW_QUOTE">EOVERFLOW_QUOTE</a>: u64 = 6;
</code></pre>



<a name="0x2_gas_market_ESLIPPAGE_EXCEEDED"></a>

Quoted cost exceeds the sender's slippage cap.


<pre><code><b>const</b> <a href="gas_market.md#0x2_gas_market_ESLIPPAGE_EXCEEDED">ESLIPPAGE_EXCEEDED</a>: u64 = 3;
</code></pre>



<a name="0x2_gas_market_ESTALE_PRICE"></a>

Price feed is stale (older than max staleness window).


<pre><code><b>const</b> <a href="gas_market.md#0x2_gas_market_ESTALE_PRICE">ESTALE_PRICE</a>: u64 = 2;
</code></pre>



<a name="0x2_gas_market_EUNSUPPORTED_GAS_COIN"></a>

Coin type is not whitelisted for gas payment.


<pre><code><b>const</b> <a href="gas_market.md#0x2_gas_market_EUNSUPPORTED_GAS_COIN">EUNSUPPORTED_GAS_COIN</a>: u64 = 1;
</code></pre>



<a name="0x2_gas_market_EZERO_INPUT"></a>

Zero gas units or zero price.


<pre><code><b>const</b> <a href="gas_market.md#0x2_gas_market_EZERO_INPUT">EZERO_INPUT</a>: u64 = 4;
</code></pre>



<a name="0x2_gas_market_USD_SCALE"></a>

Micro-USD scale (6 decimals, USDC-style).


<pre><code><b>const</b> <a href="gas_market.md#0x2_gas_market_USD_SCALE">USD_SCALE</a>: u64 = 1000000;
</code></pre>



<a name="0x2_gas_market_init"></a>

## Function `init`



<pre><code><b>fun</b> <a href="gas_market.md#0x2_gas_market_init">init</a>(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="gas_market.md#0x2_gas_market_init">init</a>(ctx: &<b>mut</b> TxContext) {
    <b>let</b> admin = <a href="gas_market.md#0x2_gas_market_AdminCap">AdminCap</a> { id: <a href="object.md#0x2_object_new">object::new</a>(ctx) };
    <b>let</b> feeder = <a href="gas_market.md#0x2_gas_market_FeederCap">FeederCap</a> { id: <a href="object.md#0x2_object_new">object::new</a>(ctx) };
    <b>let</b> <a href="table.md#0x2_table">table</a> = <a href="gas_market.md#0x2_gas_market_PriceTable">PriceTable</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        version: 1,
        usd_per_gas_unit_micros: <a href="gas_market.md#0x2_gas_market_DEFAULT_USD_PER_GAS_UNIT">DEFAULT_USD_PER_GAS_UNIT</a>,
        configs: <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>(),
        prices: <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>(),
        max_staleness_checkpoints: 600
    };
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(admin, <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx));
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(feeder, <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx));
    <a href="transfer.md#0x2_transfer_share_object">transfer::share_object</a>(<a href="table.md#0x2_table">table</a>);
}
</code></pre>



</details>

<a name="0x2_gas_market_quote_usd_fee_micros"></a>

## Function `quote_usd_fee_micros`

USD fee for <code>gas_units</code>, in micro-USD. Pure function (no state read).


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_quote_usd_fee_micros">quote_usd_fee_micros</a>(gas_units: u64, usd_per_gas_unit_micros: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_quote_usd_fee_micros">quote_usd_fee_micros</a>(
    gas_units: u64,
    usd_per_gas_unit_micros: u64,
): u64 {
    <b>let</b> fee = (gas_units <b>as</b> u128) * (usd_per_gas_unit_micros <b>as</b> u128);
    <b>assert</b>!(fee &lt;= 18446744073709551615, <a href="gas_market.md#0x2_gas_market_EOVERFLOW_QUOTE">EOVERFLOW_QUOTE</a>);
    (fee <b>as</b> u64)
}
</code></pre>



</details>

<a name="0x2_gas_market_usd_to_token_amount"></a>

## Function `usd_to_token_amount`

Convert a USD fee (micro-USD) into base units of a token with
<code>decimals</code> and <code>price_usd_micros</code> per whole token. Rounds UP so the
protocol never under-collects; floors non-zero fees to at least 1 unit.


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_usd_to_token_amount">usd_to_token_amount</a>(usd_fee_micros: u64, price_usd_micros: u64, decimals: u8): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_usd_to_token_amount">usd_to_token_amount</a>(
    usd_fee_micros: u64,
    price_usd_micros: u64,
    decimals: u8,
): u64 {
    <b>assert</b>!(price_usd_micros &gt; 0, <a href="gas_market.md#0x2_gas_market_EZERO_INPUT">EZERO_INPUT</a>);
    <b>if</b> (usd_fee_micros == 0) { <b>return</b> 0 };
    <b>let</b> scale = <a href="gas_market.md#0x2_gas_market_pow10">pow10</a>(decimals);
    <b>let</b> quoted =
        <a href="math.md#0x2_math_mul_div_round_u128">math::mul_div_round_u128</a>(
            (usd_fee_micros <b>as</b> u128),
            (scale <b>as</b> u128),
            (price_usd_micros <b>as</b> u128),
            <b>true</b>
        );
    <b>if</b> (quoted == 0) { 1 }
    <b>else</b> {
        <b>assert</b>!(quoted &lt;= 18446744073709551615, <a href="gas_market.md#0x2_gas_market_EOVERFLOW_QUOTE">EOVERFLOW_QUOTE</a>);
        (quoted <b>as</b> u64)
    }
}
</code></pre>



</details>

<a name="0x2_gas_market_quote_in_token"></a>

## Function `quote_in_token`

Full quote: gas units -> token base units. Pure function.


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_quote_in_token">quote_in_token</a>(gas_units: u64, usd_per_gas_unit_micros: u64, price_usd_micros: u64, decimals: u8): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_quote_in_token">quote_in_token</a>(
    gas_units: u64,
    usd_per_gas_unit_micros: u64,
    price_usd_micros: u64,
    decimals: u8,
): u64 {
    <b>let</b> fee = <a href="gas_market.md#0x2_gas_market_quote_usd_fee_micros">quote_usd_fee_micros</a>(gas_units, usd_per_gas_unit_micros);
    <a href="gas_market.md#0x2_gas_market_usd_to_token_amount">usd_to_token_amount</a>(fee, price_usd_micros, decimals)
}
</code></pre>



</details>

<a name="0x2_gas_market_pow10"></a>

## Function `pow10`

10^decimals as u64 (decimals <= 9 enforced by coin::create_currency).


<pre><code><b>fun</b> <a href="gas_market.md#0x2_gas_market_pow10">pow10</a>(decimals: u8): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="gas_market.md#0x2_gas_market_pow10">pow10</a>(decimals: u8): u64 {
    <b>let</b> (result, i) = (1u64, 0u8);
    <b>while</b> (i &lt; decimals) {
        result = result * 10;
        i = i + 1;
    };
    result
}
</code></pre>



</details>

<a name="0x2_gas_market_register_gas_coin"></a>

## Function `register_gas_coin`

Register (or re-enable) a gas coin. Admin only.


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_register_gas_coin">register_gas_coin</a>(_: &<a href="gas_market.md#0x2_gas_market_AdminCap">gas_market::AdminCap</a>, <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="gas_market.md#0x2_gas_market_PriceTable">gas_market::PriceTable</a>, coin_type: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, decimals: u8, price_usd_micros: u64, now_ms: u64, checkpoint_seq: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_register_gas_coin">register_gas_coin</a>(
    _: &<a href="gas_market.md#0x2_gas_market_AdminCap">AdminCap</a>,
    <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="gas_market.md#0x2_gas_market_PriceTable">PriceTable</a>,
    coin_type: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    decimals: u8,
    price_usd_micros: u64,
    now_ms: u64,
    checkpoint_seq: u64,
) {
    <b>let</b> name = <a href="dependencies/move-stdlib/string.md#0x1_string_utf8">string::utf8</a>(coin_type);
    <b>assert</b>!(<a href="gas_market.md#0x2_gas_market_is_supported_gas_coin">is_supported_gas_coin</a>(&name), <a href="gas_market.md#0x2_gas_market_EUNSUPPORTED_GAS_COIN">EUNSUPPORTED_GAS_COIN</a>);
    <b>assert</b>!(decimals &lt;= 9, <a href="gas_market.md#0x2_gas_market_EINVALID_DECIMALS">EINVALID_DECIMALS</a>);
    <b>let</b> i = <a href="gas_market.md#0x2_gas_market_find_config">find_config</a>(<a href="table.md#0x2_table">table</a>, &name);
    <b>if</b> (i &lt; <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&<a href="table.md#0x2_table">table</a>.configs)) {
        <b>let</b> cfg = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.configs, i);
        cfg.decimals = decimals;
        cfg.active = <b>true</b>;
        <b>let</b> tick = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.prices, i);
        tick.price_usd_micros = price_usd_micros;
        tick.updated_at_ms = now_ms;
        tick.checkpoint_seq = checkpoint_seq;
    } <b>else</b> {
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(
            &<b>mut</b> <a href="table.md#0x2_table">table</a>.configs,
            <a href="gas_market.md#0x2_gas_market_GasCoinConfig">GasCoinConfig</a> { coin_type: name, decimals, active: <b>true</b> }
        );
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(
            &<b>mut</b> <a href="table.md#0x2_table">table</a>.prices,
            <a href="gas_market.md#0x2_gas_market_PriceTick">PriceTick</a> {
                price_usd_micros,
                updated_at_ms: now_ms,
                checkpoint_seq
            }
        );
    };
    <a href="table.md#0x2_table">table</a>.version = <a href="table.md#0x2_table">table</a>.version + 1;
}
</code></pre>



</details>

<a name="0x2_gas_market_set_coin_active"></a>

## Function `set_coin_active`

Enable/disable a listed coin without touching its price. Admin only.


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_set_coin_active">set_coin_active</a>(_: &<a href="gas_market.md#0x2_gas_market_AdminCap">gas_market::AdminCap</a>, <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="gas_market.md#0x2_gas_market_PriceTable">gas_market::PriceTable</a>, coin_type: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, active: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_set_coin_active">set_coin_active</a>(
    _: &<a href="gas_market.md#0x2_gas_market_AdminCap">AdminCap</a>,
    <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="gas_market.md#0x2_gas_market_PriceTable">PriceTable</a>,
    coin_type: String,
    active: bool,
) {
    <b>let</b> i = <a href="gas_market.md#0x2_gas_market_find_config">find_config</a>(<a href="table.md#0x2_table">table</a>, &coin_type);
    <b>assert</b>!(i &lt; <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&<a href="table.md#0x2_table">table</a>.configs), <a href="gas_market.md#0x2_gas_market_EUNSUPPORTED_GAS_COIN">EUNSUPPORTED_GAS_COIN</a>);
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.configs, i).active = active;
    <a href="table.md#0x2_table">table</a>.version = <a href="table.md#0x2_table">table</a>.version + 1;
}
</code></pre>



</details>

<a name="0x2_gas_market_set_usd_per_gas_unit"></a>

## Function `set_usd_per_gas_unit`

Update the USD base fee. Admin (DAO vote) only. Zero would make every
quote free, so it is rejected.


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_set_usd_per_gas_unit">set_usd_per_gas_unit</a>(_: &<a href="gas_market.md#0x2_gas_market_AdminCap">gas_market::AdminCap</a>, <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="gas_market.md#0x2_gas_market_PriceTable">gas_market::PriceTable</a>, v: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_set_usd_per_gas_unit">set_usd_per_gas_unit</a>(
    _: &<a href="gas_market.md#0x2_gas_market_AdminCap">AdminCap</a>,
    <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="gas_market.md#0x2_gas_market_PriceTable">PriceTable</a>,
    v: u64,
) {
    <b>assert</b>!(v &gt; 0, <a href="gas_market.md#0x2_gas_market_EZERO_INPUT">EZERO_INPUT</a>);
    <a href="table.md#0x2_table">table</a>.usd_per_gas_unit_micros = v;
    <a href="table.md#0x2_table">table</a>.version = <a href="table.md#0x2_table">table</a>.version + 1;
}
</code></pre>



</details>

<a name="0x2_gas_market_publish_prices"></a>

## Function `publish_prices`

Publish fresh price ticks. Feeder only; bumps version once per batch.


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_publish_prices">publish_prices</a>(_: &<a href="gas_market.md#0x2_gas_market_FeederCap">gas_market::FeederCap</a>, <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="gas_market.md#0x2_gas_market_PriceTable">gas_market::PriceTable</a>, coin_types: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;, prices_usd_micros: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u64&gt;, now_ms: u64, checkpoint_seq: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_publish_prices">publish_prices</a>(
    _: &<a href="gas_market.md#0x2_gas_market_FeederCap">FeederCap</a>,
    <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="gas_market.md#0x2_gas_market_PriceTable">PriceTable</a>,
    coin_types: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    prices_usd_micros: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    now_ms: u64,
    checkpoint_seq: u64,
) {
    <b>let</b> len = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&coin_types);
    <b>assert</b>!(len == <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&prices_usd_micros), <a href="gas_market.md#0x2_gas_market_ELENGTH_MISMATCH">ELENGTH_MISMATCH</a>);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; len) {
        <b>let</b> name = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&coin_types, i);
        <b>assert</b>!(<a href="gas_market.md#0x2_gas_market_is_supported_gas_coin">is_supported_gas_coin</a>(name), <a href="gas_market.md#0x2_gas_market_EUNSUPPORTED_GAS_COIN">EUNSUPPORTED_GAS_COIN</a>);
        <b>let</b> j = <a href="gas_market.md#0x2_gas_market_find_config">find_config</a>(<a href="table.md#0x2_table">table</a>, name);
        <b>assert</b>!(j &lt; <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&<a href="table.md#0x2_table">table</a>.configs), <a href="gas_market.md#0x2_gas_market_EUNSUPPORTED_GAS_COIN">EUNSUPPORTED_GAS_COIN</a>);
        <b>let</b> tick = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.prices, j);
        tick.price_usd_micros = *<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&prices_usd_micros, i);
        tick.updated_at_ms = now_ms;
        tick.checkpoint_seq = checkpoint_seq;
        i = i + 1;
    };
    <a href="table.md#0x2_table">table</a>.version = <a href="table.md#0x2_table">table</a>.version + 1;
}
</code></pre>



</details>

<a name="0x2_gas_market_quote_for_coin"></a>

## Function `quote_for_coin`

Quote gas for <code>coin_type</code> against the live table, enforcing whitelist,
staleness, and the sender's slippage cap. Aborts on violation so the
failure is explicit in Move execution rather than silent under-charge.


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_quote_for_coin">quote_for_coin</a>(<a href="table.md#0x2_table">table</a>: &<a href="gas_market.md#0x2_gas_market_PriceTable">gas_market::PriceTable</a>, coin_type: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, gas_units: u64, current_checkpoint: u64, max_price_per_unit: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_quote_for_coin">quote_for_coin</a>(
    <a href="table.md#0x2_table">table</a>: &<a href="gas_market.md#0x2_gas_market_PriceTable">PriceTable</a>,
    coin_type: String,
    gas_units: u64,
    current_checkpoint: u64,
    max_price_per_unit: u64,
): u64 {
    <b>assert</b>!(<a href="gas_market.md#0x2_gas_market_is_supported_gas_coin">is_supported_gas_coin</a>(&coin_type), <a href="gas_market.md#0x2_gas_market_EUNSUPPORTED_GAS_COIN">EUNSUPPORTED_GAS_COIN</a>);
    <b>let</b> i = <a href="gas_market.md#0x2_gas_market_find_config">find_config</a>(<a href="table.md#0x2_table">table</a>, &coin_type);
    <b>assert</b>!(i &lt; <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&<a href="table.md#0x2_table">table</a>.configs), <a href="gas_market.md#0x2_gas_market_EUNSUPPORTED_GAS_COIN">EUNSUPPORTED_GAS_COIN</a>);
    <b>let</b> cfg = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="table.md#0x2_table">table</a>.configs, i);
    <b>assert</b>!(cfg.active, <a href="gas_market.md#0x2_gas_market_EUNSUPPORTED_GAS_COIN">EUNSUPPORTED_GAS_COIN</a>);
    <b>let</b> tick = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="table.md#0x2_table">table</a>.prices, i);
    <b>assert</b>!(tick.price_usd_micros &gt; 0, <a href="gas_market.md#0x2_gas_market_EZERO_INPUT">EZERO_INPUT</a>);
    <b>assert</b>!(
        current_checkpoint &lt;= tick.checkpoint_seq + <a href="table.md#0x2_table">table</a>.max_staleness_checkpoints,
        <a href="gas_market.md#0x2_gas_market_ESTALE_PRICE">ESTALE_PRICE</a>
    );
    <b>let</b> amount =
        <a href="gas_market.md#0x2_gas_market_quote_in_token">quote_in_token</a>(
            gas_units,
            <a href="table.md#0x2_table">table</a>.usd_per_gas_unit_micros,
            tick.price_usd_micros,
            cfg.decimals
        );
    // Slippage cap is expressed per gas unit in token base units.
    // Compared in u128 so a huge cap cannot overflow-<b>abort</b> the check.
    <b>assert</b>!(
        (amount <b>as</b> u128) &lt;= (max_price_per_unit <b>as</b> u128) * (gas_units <b>as</b> u128),
        <a href="gas_market.md#0x2_gas_market_ESLIPPAGE_EXCEEDED">ESLIPPAGE_EXCEEDED</a>
    );
    amount
}
</code></pre>



</details>

<a name="0x2_gas_market_find_config"></a>

## Function `find_config`



<pre><code><b>fun</b> <a href="gas_market.md#0x2_gas_market_find_config">find_config</a>(<a href="table.md#0x2_table">table</a>: &<a href="gas_market.md#0x2_gas_market_PriceTable">gas_market::PriceTable</a>, coin_type: &<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="gas_market.md#0x2_gas_market_find_config">find_config</a>(<a href="table.md#0x2_table">table</a>: &<a href="gas_market.md#0x2_gas_market_PriceTable">PriceTable</a>, coin_type: &String): u64 {
    <b>let</b> (i, len) = (0, <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&<a href="table.md#0x2_table">table</a>.configs));
    <b>while</b> (i &lt; len) {
        <b>if</b> (<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="table.md#0x2_table">table</a>.configs, i).coin_type == *coin_type) { <b>return</b> i };
        i = i + 1;
    };
    len
}
</code></pre>



</details>

<a name="0x2_gas_market_is_supported_gas_coin"></a>

## Function `is_supported_gas_coin`



<pre><code><b>fun</b> <a href="gas_market.md#0x2_gas_market_is_supported_gas_coin">is_supported_gas_coin</a>(coin_type: &<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="gas_market.md#0x2_gas_market_is_supported_gas_coin">is_supported_gas_coin</a>(coin_type: &String): bool {
    *coin_type == <a href="dependencies/move-stdlib/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="kanari.md#0x2_kanari_KANARI">0x2::kanari::KANARI</a>")
        || *coin_type == <a href="dependencies/move-stdlib/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="usd.md#0x2_usd_USD">0x2::usd::USD</a>")
}
</code></pre>



</details>

<a name="0x2_gas_market_version"></a>

## Function `version`



<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_version">version</a>(<a href="table.md#0x2_table">table</a>: &<a href="gas_market.md#0x2_gas_market_PriceTable">gas_market::PriceTable</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_version">version</a>(<a href="table.md#0x2_table">table</a>: &<a href="gas_market.md#0x2_gas_market_PriceTable">PriceTable</a>): u64 {
    <a href="table.md#0x2_table">table</a>.version
}
</code></pre>



</details>

<a name="0x2_gas_market_usd_per_gas_unit"></a>

## Function `usd_per_gas_unit`



<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_usd_per_gas_unit">usd_per_gas_unit</a>(<a href="table.md#0x2_table">table</a>: &<a href="gas_market.md#0x2_gas_market_PriceTable">gas_market::PriceTable</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_usd_per_gas_unit">usd_per_gas_unit</a>(<a href="table.md#0x2_table">table</a>: &<a href="gas_market.md#0x2_gas_market_PriceTable">PriceTable</a>): u64 {
    <a href="table.md#0x2_table">table</a>.usd_per_gas_unit_micros
}
</code></pre>



</details>

<a name="0x2_gas_market_num_coins"></a>

## Function `num_coins`



<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_num_coins">num_coins</a>(<a href="table.md#0x2_table">table</a>: &<a href="gas_market.md#0x2_gas_market_PriceTable">gas_market::PriceTable</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_num_coins">num_coins</a>(<a href="table.md#0x2_table">table</a>: &<a href="gas_market.md#0x2_gas_market_PriceTable">PriceTable</a>): u64 {
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&<a href="table.md#0x2_table">table</a>.configs)
}
</code></pre>



</details>

<a name="0x2_gas_market_usd_scale"></a>

## Function `usd_scale`



<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_usd_scale">usd_scale</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_usd_scale">usd_scale</a>(): u64 {
    <a href="gas_market.md#0x2_gas_market_USD_SCALE">USD_SCALE</a>
}
</code></pre>



</details>

<a name="0x2_gas_market_assert_authorized"></a>

## Function `assert_authorized`



<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_assert_authorized">assert_authorized</a>(ok: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_market.md#0x2_gas_market_assert_authorized">assert_authorized</a>(ok: bool) {
    <b>assert</b>!(ok, <a href="gas_market.md#0x2_gas_market_ENOT_AUTHORIZED">ENOT_AUTHORIZED</a>);
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
