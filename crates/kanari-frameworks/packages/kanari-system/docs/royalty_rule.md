
<a name="0x2_royalty_rule"></a>

# Module `0x2::royalty_rule`

Percentage-or-floor royalty rule for <code>kanari_system::transfer_policy</code>.

Royalty is a *rule*, not a field on the token. That is the whole point of
the policy design: the fee applies to every trade of the type, the type
owner can change it at any time, and the change takes effect immediately
across all instances — none of which is possible if the rate is frozen into
each minted object.


<a name="@Enforcement_0"></a>

### Enforcement


<code><a href="pay.md#0x2_pay">pay</a></code> is what makes this safe. It takes the policy mutably, the request
mutably, and the buyer's payment mutably; it splits the fee, sends it to the
beneficiary, and only then calls <code>add_receipt</code>. A caller who tries to skip
<code><a href="pay.md#0x2_pay">pay</a></code> cannot confirm the request, because <code>TransferRequest</code> has no <code>drop</code>
and <code>confirm_request</code> refuses incomplete receipts.


<a name="@Config_encoding_1"></a>

### Config encoding


<code><a href="transfer_policy.md#0x2_transfer_policy">transfer_policy</a></code> stores rule config as opaque BCS bytes and cannot
interpret them, so this module encodes and decodes its own. The layout is
<code>(u64 amount_bp, u64 min_amount, <b>address</b> beneficiary)</code> in that order, read
back with <code>peel_u64</code> / <code>peel_address</code>.


    -  [Enforcement](#@Enforcement_0)
    -  [Config encoding](#@Config_encoding_1)
-  [Struct `Rule`](#0x2_royalty_rule_Rule)
-  [Constants](#@Constants_2)
-  [Function `add`](#0x2_royalty_rule_add)
-  [Function `add_for_krc_721_collection`](#0x2_royalty_rule_add_for_krc_721_collection)
-  [Function `update_for_krc_721_collection`](#0x2_royalty_rule_update_for_krc_721_collection)
-  [Function `remove_for_krc_721_collection`](#0x2_royalty_rule_remove_for_krc_721_collection)
-  [Function `update`](#0x2_royalty_rule_update)
-  [Function `remove`](#0x2_royalty_rule_remove)
-  [Function `fee_amount`](#0x2_royalty_rule_fee_amount)
-  [Function `compute`](#0x2_royalty_rule_compute)
-  [Function `pay`](#0x2_royalty_rule_pay)
-  [Function `encode_config`](#0x2_royalty_rule_encode_config)
-  [Function `u64_le`](#0x2_royalty_rule_u64_le)
-  [Function `read_u64`](#0x2_royalty_rule_read_u64)
-  [Function `decode_bp`](#0x2_royalty_rule_decode_bp)
-  [Function `decode_min`](#0x2_royalty_rule_decode_min)
-  [Function `decode_beneficiary`](#0x2_royalty_rule_decode_beneficiary)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="address.md#0x2_address">0x2::address</a>;
<b>use</b> <a href="coin.md#0x2_coin">0x2::coin</a>;
<b>use</b> <a href="krc_721.md#0x2_krc_721">0x2::krc_721</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="transfer_policy.md#0x2_transfer_policy">0x2::transfer_policy</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
</code></pre>



<a name="0x2_royalty_rule_Rule"></a>

## Struct `Rule`

The rule witness. Its type is the registry key, so this rule can never
collide with another rule module.


<pre><code><b>struct</b> <a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a> <b>has</b> drop
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

<a name="@Constants_2"></a>

## Constants


<a name="0x2_royalty_rule_CONFIG_LEN"></a>

Encoded config length: 8 (<code>amount_bp</code>) + 8 (<code>min_amount</code>) + 32
(<code>beneficiary</code>).


<pre><code><b>const</b> <a href="royalty_rule.md#0x2_royalty_rule_CONFIG_LEN">CONFIG_LEN</a>: u64 = 48;
</code></pre>



<a name="0x2_royalty_rule_E_BAD_CONFIG"></a>

The stored config was not produced by <code>encode_config</code>.


<pre><code><b>const</b> <a href="royalty_rule.md#0x2_royalty_rule_E_BAD_CONFIG">E_BAD_CONFIG</a>: u64 = 3;
</code></pre>



<a name="0x2_royalty_rule_E_BPS_TOO_LARGE"></a>

<code>amount_bp</code> was above 100%.


<pre><code><b>const</b> <a href="royalty_rule.md#0x2_royalty_rule_E_BPS_TOO_LARGE">E_BPS_TOO_LARGE</a>: u64 = 0;
</code></pre>



<a name="0x2_royalty_rule_E_INSUFFICIENT"></a>

The buyer's payment does not cover the fee.


<pre><code><b>const</b> <a href="royalty_rule.md#0x2_royalty_rule_E_INSUFFICIENT">E_INSUFFICIENT</a>: u64 = 1;
</code></pre>



<a name="0x2_royalty_rule_E_INVALID_BENEFICIARY"></a>

The beneficiary is the reserved system address.


<pre><code><b>const</b> <a href="royalty_rule.md#0x2_royalty_rule_E_INVALID_BENEFICIARY">E_INVALID_BENEFICIARY</a>: u64 = 2;
</code></pre>



<a name="0x2_royalty_rule_MAX_BPS"></a>

100% expressed in basis points.


<pre><code><b>const</b> <a href="royalty_rule.md#0x2_royalty_rule_MAX_BPS">MAX_BPS</a>: u64 = 10000;
</code></pre>



<a name="0x2_royalty_rule_add"></a>

## Function `add`

Owner action: install the royalty rule on <code>T</code>'s policy.

<code>amount_bp</code> is the percentage of the sale price (500 = 5%). <code>min_amount</code>
is a floor so a very cheap sale still pays something; the higher of the
two applies. Setting <code>amount_bp</code> to 0 makes it a flat commission.


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_add">add</a>&lt;T: store, key&gt;(policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;T&gt;, amount_bp: u64, min_amount: u64, beneficiary: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_add">add</a>&lt;T: key + store&gt;(
    policy: &<b>mut</b> TransferPolicy&lt;T&gt;,
    cap: &TransferPolicyCap&lt;T&gt;,
    amount_bp: u64,
    min_amount: u64,
    beneficiary: <b>address</b>,
) {
    <b>assert</b>!(amount_bp &lt;= <a href="royalty_rule.md#0x2_royalty_rule_MAX_BPS">MAX_BPS</a>, <a href="royalty_rule.md#0x2_royalty_rule_E_BPS_TOO_LARGE">E_BPS_TOO_LARGE</a>);
    <b>assert</b>!(beneficiary != @0x0, <a href="royalty_rule.md#0x2_royalty_rule_E_INVALID_BENEFICIARY">E_INVALID_BENEFICIARY</a>);
    <b>let</b> config = <a href="royalty_rule.md#0x2_royalty_rule_encode_config">encode_config</a>(amount_bp, min_amount, beneficiary);
    <a href="transfer_policy.md#0x2_transfer_policy_add_rule">transfer_policy::add_rule</a>&lt;<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a>, T&gt;(<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a> {}, policy, cap, config);
}
</code></pre>



</details>

<a name="0x2_royalty_rule_add_for_krc_721_collection"></a>

## Function `add_for_krc_721_collection`

Install royalties on one KRC-721 collection's scoped policy.


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_add_for_krc_721_collection">add_for_krc_721_collection</a>(policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;<a href="krc_721.md#0x2_krc_721_Nft">krc_721::Nft</a>&gt;, cap: &<a href="krc_721.md#0x2_krc_721_CollectionPolicyCap">krc_721::CollectionPolicyCap</a>, amount_bp: u64, min_amount: u64, beneficiary: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_add_for_krc_721_collection">add_for_krc_721_collection</a>(
    policy: &<b>mut</b> TransferPolicy&lt;Nft&gt;,
    cap: &CollectionPolicyCap,
    amount_bp: u64,
    min_amount: u64,
    beneficiary: <b>address</b>,
) {
    <b>assert</b>!(amount_bp &lt;= <a href="royalty_rule.md#0x2_royalty_rule_MAX_BPS">MAX_BPS</a>, <a href="royalty_rule.md#0x2_royalty_rule_E_BPS_TOO_LARGE">E_BPS_TOO_LARGE</a>);
    <b>assert</b>!(beneficiary != @0x0, <a href="royalty_rule.md#0x2_royalty_rule_E_INVALID_BENEFICIARY">E_INVALID_BENEFICIARY</a>);
    <b>let</b> config = <a href="royalty_rule.md#0x2_royalty_rule_encode_config">encode_config</a>(amount_bp, min_amount, beneficiary);
    <a href="krc_721.md#0x2_krc_721_add_collection_policy_rule">krc_721::add_collection_policy_rule</a>&lt;<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a>&gt;(<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a> {}, policy, cap, config);
}
</code></pre>



</details>

<a name="0x2_royalty_rule_update_for_krc_721_collection"></a>

## Function `update_for_krc_721_collection`

Update royalties on one KRC-721 collection's scoped policy.


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_update_for_krc_721_collection">update_for_krc_721_collection</a>(policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;<a href="krc_721.md#0x2_krc_721_Nft">krc_721::Nft</a>&gt;, cap: &<a href="krc_721.md#0x2_krc_721_CollectionPolicyCap">krc_721::CollectionPolicyCap</a>, amount_bp: u64, min_amount: u64, beneficiary: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_update_for_krc_721_collection">update_for_krc_721_collection</a>(
    policy: &<b>mut</b> TransferPolicy&lt;Nft&gt;,
    cap: &CollectionPolicyCap,
    amount_bp: u64,
    min_amount: u64,
    beneficiary: <b>address</b>,
) {
    <b>assert</b>!(amount_bp &lt;= <a href="royalty_rule.md#0x2_royalty_rule_MAX_BPS">MAX_BPS</a>, <a href="royalty_rule.md#0x2_royalty_rule_E_BPS_TOO_LARGE">E_BPS_TOO_LARGE</a>);
    <b>assert</b>!(beneficiary != @0x0, <a href="royalty_rule.md#0x2_royalty_rule_E_INVALID_BENEFICIARY">E_INVALID_BENEFICIARY</a>);
    <b>let</b> config = <a href="royalty_rule.md#0x2_royalty_rule_encode_config">encode_config</a>(amount_bp, min_amount, beneficiary);
    <a href="krc_721.md#0x2_krc_721_replace_collection_policy_rule">krc_721::replace_collection_policy_rule</a>&lt;<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a>&gt;(<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a> {}, policy, cap, config);
}
</code></pre>



</details>

<a name="0x2_royalty_rule_remove_for_krc_721_collection"></a>

## Function `remove_for_krc_721_collection`

Remove royalties from one KRC-721 collection's scoped policy.


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_remove_for_krc_721_collection">remove_for_krc_721_collection</a>(policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;<a href="krc_721.md#0x2_krc_721_Nft">krc_721::Nft</a>&gt;, cap: &<a href="krc_721.md#0x2_krc_721_CollectionPolicyCap">krc_721::CollectionPolicyCap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_remove_for_krc_721_collection">remove_for_krc_721_collection</a>(
    policy: &<b>mut</b> TransferPolicy&lt;Nft&gt;,
    cap: &CollectionPolicyCap,
) {
    <a href="krc_721.md#0x2_krc_721_remove_collection_policy_rule">krc_721::remove_collection_policy_rule</a>&lt;<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a>&gt;(<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a> {}, policy, cap);
}
</code></pre>



</details>

<a name="0x2_royalty_rule_update"></a>

## Function `update`

Update the rule's terms in place.


<pre><code><b>public</b> <b>fun</b> <b>update</b>&lt;T: store, key&gt;(policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;T&gt;, amount_bp: u64, min_amount: u64, beneficiary: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>update</b>&lt;T: key + store&gt;(
    policy: &<b>mut</b> TransferPolicy&lt;T&gt;,
    cap: &TransferPolicyCap&lt;T&gt;,
    amount_bp: u64,
    min_amount: u64,
    beneficiary: <b>address</b>,
) {
    <b>assert</b>!(amount_bp &lt;= <a href="royalty_rule.md#0x2_royalty_rule_MAX_BPS">MAX_BPS</a>, <a href="royalty_rule.md#0x2_royalty_rule_E_BPS_TOO_LARGE">E_BPS_TOO_LARGE</a>);
    <b>assert</b>!(beneficiary != @0x0, <a href="royalty_rule.md#0x2_royalty_rule_E_INVALID_BENEFICIARY">E_INVALID_BENEFICIARY</a>);
    <b>let</b> config = <a href="royalty_rule.md#0x2_royalty_rule_encode_config">encode_config</a>(amount_bp, min_amount, beneficiary);
    <a href="transfer_policy.md#0x2_transfer_policy_replace_rule">transfer_policy::replace_rule</a>&lt;<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a>, T&gt;(<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a> {}, policy, cap, config);
}
</code></pre>



</details>

<a name="0x2_royalty_rule_remove"></a>

## Function `remove`

Remove the royalty rule entirely, making trades of <code>T</code> free again.


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_remove">remove</a>&lt;T: store, key&gt;(policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_remove">remove</a>&lt;T: key + store&gt;(
    policy: &<b>mut</b> TransferPolicy&lt;T&gt;,
    cap: &TransferPolicyCap&lt;T&gt;,
) {
    <a href="transfer_policy.md#0x2_transfer_policy_remove_rule">transfer_policy::remove_rule</a>&lt;<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a>, T&gt;(<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a> {}, policy, cap);
}
</code></pre>



</details>

<a name="0x2_royalty_rule_fee_amount"></a>

## Function `fee_amount`

The fee owed for a sale of <code>paid</code> base units.

Callable before a trade so a buyer can quote the price off-chain or in a
dry run without constructing a request.


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_fee_amount">fee_amount</a>&lt;T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, paid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_fee_amount">fee_amount</a>&lt;T: key + store&gt;(policy: &TransferPolicy&lt;T&gt;, paid: u64): u64 {
    <b>let</b> bytes = <a href="transfer_policy.md#0x2_transfer_policy_rule_config">transfer_policy::rule_config</a>&lt;<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a>, T&gt;(policy);
    <b>let</b> amount_bp = <a href="royalty_rule.md#0x2_royalty_rule_decode_bp">decode_bp</a>(bytes);
    <b>let</b> min_amount = <a href="royalty_rule.md#0x2_royalty_rule_decode_min">decode_min</a>(bytes);
    <a href="royalty_rule.md#0x2_royalty_rule_compute">compute</a>(paid, amount_bp, min_amount)
}
</code></pre>



</details>

<a name="0x2_royalty_rule_compute"></a>

## Function `compute`

The fee formula: percentage of <code>paid</code>, floored at <code>min_amount</code>.


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_compute">compute</a>(paid: u64, amount_bp: u64, min_amount: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_compute">compute</a>(paid: u64, amount_bp: u64, min_amount: u64): u64 {
    <b>let</b> share = (paid <b>as</b> u128) * (amount_bp <b>as</b> u128) / (<a href="royalty_rule.md#0x2_royalty_rule_MAX_BPS">MAX_BPS</a> <b>as</b> u128);
    <b>let</b> share = (share <b>as</b> u64);
    <b>if</b> (share &lt; min_amount) { min_amount } <b>else</b> { share }
}
</code></pre>



</details>

<a name="0x2_royalty_rule_pay"></a>

## Function `pay`

Buyer action: pay the royalty and unblock the request.

The fee is split off <code>payment</code> and sent straight to the beneficiary, so
the policy never holds value — nothing can be stranded inside it. The
receipt is added last: if the split or the transfer aborted, the receipt
is never written and the request stays unconfirmable.

<code>Asset</code> is the currency the fee settles in and <code>T</code> is the traded type.
They are separate because a royalty is almost never denominated in the
asset being traded — charging a NFT in itself would be circular. The
canonical shape is an NFT type <code>T</code> with a stablecoin <code>Asset</code>, which is
why this takes two type parameters rather than reusing one.


<pre><code><b>public</b> <b>fun</b> <a href="pay.md#0x2_pay">pay</a>&lt;T: store, key, Asset: store, key&gt;(policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, request: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;, payment: &<b>mut</b> <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;Asset&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pay.md#0x2_pay">pay</a>&lt;T: key + store, Asset: key + store&gt;(
    policy: &<b>mut</b> TransferPolicy&lt;T&gt;,
    request: &<b>mut</b> TransferRequest&lt;T&gt;,
    payment: &<b>mut</b> Coin&lt;Asset&gt;,
    ctx: &<b>mut</b> TxContext,
) {
    <b>let</b> bytes = <a href="transfer_policy.md#0x2_transfer_policy_rule_config">transfer_policy::rule_config</a>&lt;<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a>, T&gt;(policy);
    <b>let</b> amount_bp = <a href="royalty_rule.md#0x2_royalty_rule_decode_bp">decode_bp</a>(bytes);
    <b>let</b> min_amount = <a href="royalty_rule.md#0x2_royalty_rule_decode_min">decode_min</a>(bytes);
    <b>let</b> beneficiary = <a href="royalty_rule.md#0x2_royalty_rule_decode_beneficiary">decode_beneficiary</a>(bytes);

    <b>let</b> paid = <a href="transfer_policy.md#0x2_transfer_policy_paid">transfer_policy::paid</a>(request);
    <b>let</b> fee = <a href="royalty_rule.md#0x2_royalty_rule_compute">compute</a>(paid, amount_bp, min_amount);
    // A zero-price trade <b>with</b> no floor owes nothing, but the receipt is
    // still required so the trade can complete.
    <b>if</b> (fee &gt; 0) {
        <b>assert</b>!(<a href="coin.md#0x2_coin_value">coin::value</a>(payment) &gt;= fee, <a href="royalty_rule.md#0x2_royalty_rule_E_INSUFFICIENT">E_INSUFFICIENT</a>);
        <b>let</b> royalty = <a href="coin.md#0x2_coin_split">coin::split</a>(payment, fee, ctx);
        <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(royalty, beneficiary);
    };

    <a href="transfer_policy.md#0x2_transfer_policy_record_payout">transfer_policy::record_payout</a>&lt;T&gt;(policy, fee);
    <a href="transfer_policy.md#0x2_transfer_policy_add_receipt">transfer_policy::add_receipt</a>&lt;<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a>, T&gt;(<a href="royalty_rule.md#0x2_royalty_rule_Rule">Rule</a> {}, policy, request);
}
</code></pre>



</details>

<a name="0x2_royalty_rule_encode_config"></a>

## Function `encode_config`

Pack the config for storage in the policy's opaque config slot.

Hand-packed rather than <code><a href="dependencies/move-stdlib/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a></code> on a struct: the native
serializer's behaviour on a plain (non-<code>#[<a href="dependencies/move-stdlib/bcs.md#0x1_bcs">bcs</a>]</code>) struct is not something
to depend on for a value that has to round-trip between modules. The
layout is fixed-width and explicit, so decoding cannot drift.


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_encode_config">encode_config</a>(amount_bp: u64, min_amount: u64, beneficiary: <b>address</b>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_encode_config">encode_config</a>(amount_bp: u64, min_amount: u64, beneficiary: <b>address</b>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> bytes = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> bytes, <a href="royalty_rule.md#0x2_royalty_rule_u64_le">u64_le</a>(amount_bp));
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> bytes, <a href="royalty_rule.md#0x2_royalty_rule_u64_le">u64_le</a>(min_amount));
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> bytes, <a href="address.md#0x2_address_to_bytes">address::to_bytes</a>(beneficiary));
    bytes
}
</code></pre>



</details>

<a name="0x2_royalty_rule_u64_le"></a>

## Function `u64_le`

Little-endian u64, least-significant byte first.


<pre><code><b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_u64_le">u64_le</a>(value: u64): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_u64_le">u64_le</a>(value: u64): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> bytes = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <b>let</b> i = 0;
    <b>while</b> (i &lt; 8) {
        <b>let</b> shift = (i <b>as</b> u8) * 8;
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> bytes, (((value &gt;&gt; shift) & 0xFF) <b>as</b> u8));
        i = i + 1;
    };
    bytes
}
</code></pre>



</details>

<a name="0x2_royalty_rule_read_u64"></a>

## Function `read_u64`

Read a little-endian u64 out of <code>bytes</code> starting at <code>offset</code>.


<pre><code><b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_read_u64">read_u64</a>(bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, offset: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_read_u64">read_u64</a>(bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, offset: u64): u64 {
    <b>let</b> value = 0u64;
    <b>let</b> i = 0;
    <b>while</b> (i &lt; 8) {
        <b>let</b> byte = (*<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(bytes, offset + i) <b>as</b> u64);
        value = value | (byte &lt;&lt; ((i <b>as</b> u8) * 8));
        i = i + 1;
    };
    value
}
</code></pre>



</details>

<a name="0x2_royalty_rule_decode_bp"></a>

## Function `decode_bp`



<pre><code><b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_decode_bp">decode_bp</a>(bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_decode_bp">decode_bp</a>(bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64 {
    <a href="royalty_rule.md#0x2_royalty_rule_read_u64">read_u64</a>(&bytes, 0)
}
</code></pre>



</details>

<a name="0x2_royalty_rule_decode_min"></a>

## Function `decode_min`



<pre><code><b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_decode_min">decode_min</a>(bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_decode_min">decode_min</a>(bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64 {
    <a href="royalty_rule.md#0x2_royalty_rule_read_u64">read_u64</a>(&bytes, 8)
}
</code></pre>



</details>

<a name="0x2_royalty_rule_decode_beneficiary"></a>

## Function `decode_beneficiary`



<pre><code><b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_decode_beneficiary">decode_beneficiary</a>(bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="royalty_rule.md#0x2_royalty_rule_decode_beneficiary">decode_beneficiary</a>(bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b> {
    <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&bytes) &gt;= <a href="royalty_rule.md#0x2_royalty_rule_CONFIG_LEN">CONFIG_LEN</a>, <a href="royalty_rule.md#0x2_royalty_rule_E_BAD_CONFIG">E_BAD_CONFIG</a>);
    <b>let</b> raw = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <b>let</b> i = 16;
    <b>while</b> (i &lt; <a href="royalty_rule.md#0x2_royalty_rule_CONFIG_LEN">CONFIG_LEN</a>) {
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> raw, *<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&bytes, i));
        i = i + 1;
    };
    <a href="address.md#0x2_address_from_bytes">address::from_bytes</a>(raw)
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
