
<a name="0x2_kiosk_rule"></a>

# Module `0x2::kiosk_rule`

The kiosk rule: an item whose policy registers this rule can only be
traded through <code>kanari_system::kiosk</code>.

Mirrors <code>sui::kiosk_rule</code>. The point is to turn a documented convention —
"policy sales should go through the seller's kiosk" — into a mechanism: if
the rule is registered, <code><a href="transfer_policy.md#0x2_transfer_policy_confirm_request">transfer_policy::confirm_request</a></code> aborts unless
the rule's receipt is on the request, and the receipt can only ever be
minted inside <code><a href="kiosk.md#0x2_kiosk_settle">kiosk::settle</a></code>, which is <code><b>friend</b></code>-gated to the kiosk module.
A trade that bypasses the kiosk cannot produce the receipt, so it cannot
confirm.


<a name="@Why_the_kiosk_can_be_trusted_here_0"></a>

### Why the kiosk can be trusted here


A kiosk rule receipt is not issued on a credential the caller might
forge. <code><a href="kiosk.md#0x2_kiosk_purchase">kiosk::purchase</a></code> and <code><a href="kiosk.md#0x2_kiosk_purchase_with_cap">kiosk::purchase_with_cap</a></code> take the real
<code>Kiosk</code> object as a <code>&<b>mut</b></code> argument — live object identity enforced by the
runtime — and <code>settle</code> marks the request only in that context. Nothing
else can reach this module's receipt function: it is <code><b>public</b>(<b>friend</b>)</code> and
the module's only friend is the kiosk itself.


<a name="@Deliberate_scope_1"></a>

### Deliberate scope


The rule carries no config: it is either registered or not. Availability
(<code>is_enabled</code>) is exposed so a seller UI can show where the item can be
sold before the rules are applied.


    -  [Why the kiosk can be trusted here](#@Why_the_kiosk_can_be_trusted_here_0)
    -  [Deliberate scope](#@Deliberate_scope_1)
-  [Struct `Rule`](#0x2_kiosk_rule_Rule)
-  [Function `add_rule`](#0x2_kiosk_rule_add_rule)
-  [Function `remove_rule`](#0x2_kiosk_rule_remove_rule)
-  [Function `is_enabled`](#0x2_kiosk_rule_is_enabled)
-  [Function `kiosk_receipt`](#0x2_kiosk_rule_kiosk_receipt)


<pre><code><b>use</b> <a href="transfer_policy.md#0x2_transfer_policy">0x2::transfer_policy</a>;
</code></pre>



<a name="0x2_kiosk_rule_Rule"></a>

## Struct `Rule`

The rule witness. Zero-sized and <code>drop</code>-able like every transfer-policy
rule; authority lives in the friend gate, not in the value.


<pre><code><b>struct</b> <a href="kiosk_rule.md#0x2_kiosk_rule_Rule">Rule</a> <b>has</b> drop
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

<a name="0x2_kiosk_rule_add_rule"></a>

## Function `add_rule`



<pre><code><b>public</b> <b>fun</b> <a href="kiosk_rule.md#0x2_kiosk_rule_add_rule">add_rule</a>&lt;T: store, key&gt;(policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk_rule.md#0x2_kiosk_rule_add_rule">add_rule</a>&lt;T: key + store&gt;(
    policy: &<b>mut</b> TransferPolicy&lt;T&gt;,
    cap: &TransferPolicyCap&lt;T&gt;,
) {
    <a href="transfer_policy.md#0x2_transfer_policy_add_rule">transfer_policy::add_rule</a>&lt;<a href="kiosk_rule.md#0x2_kiosk_rule_Rule">Rule</a>, T&gt;(<a href="kiosk_rule.md#0x2_kiosk_rule_Rule">Rule</a> {}, policy, cap, b"");
}
</code></pre>



</details>

<a name="0x2_kiosk_rule_remove_rule"></a>

## Function `remove_rule`



<pre><code><b>public</b> <b>fun</b> <a href="kiosk_rule.md#0x2_kiosk_rule_remove_rule">remove_rule</a>&lt;T: store, key&gt;(policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk_rule.md#0x2_kiosk_rule_remove_rule">remove_rule</a>&lt;T: key + store&gt;(
    policy: &<b>mut</b> TransferPolicy&lt;T&gt;,
    cap: &TransferPolicyCap&lt;T&gt;,
) {
    <a href="transfer_policy.md#0x2_transfer_policy_remove_rule">transfer_policy::remove_rule</a>&lt;<a href="kiosk_rule.md#0x2_kiosk_rule_Rule">Rule</a>, T&gt;(<a href="kiosk_rule.md#0x2_kiosk_rule_Rule">Rule</a> {}, policy, cap);
}
</code></pre>



</details>

<a name="0x2_kiosk_rule_is_enabled"></a>

## Function `is_enabled`

True iff <code>T</code>'s policy requires kiosk-only trading.


<pre><code><b>public</b> <b>fun</b> <a href="kiosk_rule.md#0x2_kiosk_rule_is_enabled">is_enabled</a>&lt;T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="kiosk_rule.md#0x2_kiosk_rule_is_enabled">is_enabled</a>&lt;T: key + store&gt;(policy: &TransferPolicy&lt;T&gt;): bool {
    <a href="transfer_policy.md#0x2_transfer_policy_has_rule">transfer_policy::has_rule</a>&lt;<a href="kiosk_rule.md#0x2_kiosk_rule_Rule">Rule</a>, T&gt;(policy)
}
</code></pre>



</details>

<a name="0x2_kiosk_rule_kiosk_receipt"></a>

## Function `kiosk_receipt`

Mint the kiosk-rule receipt for <code>request</code>.

<code><b>public</b>(<b>friend</b>)</code>: reachable only from the kiosk module, which calls it
from inside <code>settle</code> — i.e. after the sale was validated against the
live kiosk object.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="kiosk_rule.md#0x2_kiosk_rule_kiosk_receipt">kiosk_receipt</a>&lt;T: store, key&gt;(policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, request: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="kiosk_rule.md#0x2_kiosk_rule_kiosk_receipt">kiosk_receipt</a>&lt;T: key + store&gt;(
    policy: &<b>mut</b> TransferPolicy&lt;T&gt;,
    request: &<b>mut</b> TransferRequest&lt;T&gt;,
) {
    <a href="transfer_policy.md#0x2_transfer_policy_add_receipt">transfer_policy::add_receipt</a>&lt;<a href="kiosk_rule.md#0x2_kiosk_rule_Rule">Rule</a>, T&gt;(<a href="kiosk_rule.md#0x2_kiosk_rule_Rule">Rule</a> {}, policy, request);
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
