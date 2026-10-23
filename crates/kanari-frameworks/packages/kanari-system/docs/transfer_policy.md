
<a name="0x2_transfer_policy"></a>

# Module `0x2::transfer_policy`

Creator-controlled transfer rules.

This is <code>sui::transfer_policy</code> and <code>sui::policy</code> **merged into one module**.
Sui can split them because Move lets a module read another's fields; this
runtime's legacy edition does not, so <code><b>friend</b></code> access to private fields plus
a two-way <code><b>use</b></code> would be a dependency cycle. Fields are private to the
defining module in every edition, so keeping the rule table here and
exposing it through witness-gated functions is the only way to get the same
API surface without the cycle. <code><b>use</b> kanari_system::policy::*</code> still resolves
for rule modules.


<a name="@Enforcement_0"></a>

### Enforcement


The safety mechanism is the shape of <code><a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a></code>, not a runtime check.
It is declared with **no abilities at all** — no <code>store</code>, no <code>drop</code>, no
<code><b>copy</b></code>. A transaction cannot put one away and cannot abandon it: the only
way out is <code>confirm_request</code>, which refuses unless every registered rule
has submitted its receipt. A transaction that creates a request and forgets
it fails to execute.

Without this, "the buyer must pay a royalty" would be a convention. Here it
is a type-level constraint: there is no <code>drop</code> to fall back on.


<a name="@Rules_1"></a>

### Rules


A rule is a zero-sized witness type in the rule's own module
(<code><b>struct</b> Rule <b>has</b> drop {}</code>), which namespaces its state and means adding a
rule never changes <code><a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a></code>'s layout. Only a function taking that
witness can write that rule's receipt, so a rule's module is the sole
authority on whether it has been satisfied.

Collected value is tracked as plain <code>u64</code> amounts per rule plus a running
<code>Balance&lt;T&gt;</code>; <code><a href="transfer_policy.md#0x2_transfer_policy_AnyRule">AnyRule</a></code> must stay type-erased to live in the rule table.


    -  [Enforcement](#@Enforcement_0)
    -  [Rules](#@Rules_1)
-  [Struct `AnyRule`](#0x2_transfer_policy_AnyRule)
-  [Resource `TransferPolicy`](#0x2_transfer_policy_TransferPolicy)
-  [Resource `TransferPolicyCap`](#0x2_transfer_policy_TransferPolicyCap)
-  [Resource `Krc721CollectionScope`](#0x2_transfer_policy_Krc721CollectionScope)
-  [Resource `Krc721CollectionPolicyCap`](#0x2_transfer_policy_Krc721CollectionPolicyCap)
-  [Struct `TransferRequest`](#0x2_transfer_policy_TransferRequest)
-  [Constants](#@Constants_2)
-  [Function `new`](#0x2_transfer_policy_new)
-  [Function `new_for_krc_721_collection`](#0x2_transfer_policy_new_for_krc_721_collection)
-  [Function `share`](#0x2_transfer_policy_share)
-  [Function `destroy`](#0x2_transfer_policy_destroy)
-  [Function `transfer_cap`](#0x2_transfer_policy_transfer_cap)
-  [Function `freeze_cap`](#0x2_transfer_policy_freeze_cap)
-  [Function `share_krc_721_scope`](#0x2_transfer_policy_share_krc_721_scope)
-  [Function `transfer_krc_721_collection_cap`](#0x2_transfer_policy_transfer_krc_721_collection_cap)
-  [Function `krc_721_scope_collection_id`](#0x2_transfer_policy_krc_721_scope_collection_id)
-  [Function `add_krc_721_collection_rule`](#0x2_transfer_policy_add_krc_721_collection_rule)
-  [Function `replace_krc_721_collection_rule`](#0x2_transfer_policy_replace_krc_721_collection_rule)
-  [Function `remove_krc_721_collection_rule`](#0x2_transfer_policy_remove_krc_721_collection_rule)
-  [Function `assert_krc_721_policy_cap`](#0x2_transfer_policy_assert_krc_721_policy_cap)
-  [Function `id`](#0x2_transfer_policy_id)
-  [Function `cap_id`](#0x2_transfer_policy_cap_id)
-  [Function `new_request`](#0x2_transfer_policy_new_request)
-  [Function `is_krc_721_type`](#0x2_transfer_policy_is_krc_721_type)
-  [Function `new_krc_721_request`](#0x2_transfer_policy_new_krc_721_request)
-  [Function `confirm_request`](#0x2_transfer_policy_confirm_request)
-  [Function `are_receipts_complete`](#0x2_transfer_policy_are_receipts_complete)
-  [Function `assert_request_policy`](#0x2_transfer_policy_assert_request_policy)
-  [Function `item`](#0x2_transfer_policy_item)
-  [Function `paid`](#0x2_transfer_policy_paid)
-  [Function `from`](#0x2_transfer_policy_from)
-  [Function `add_rule`](#0x2_transfer_policy_add_rule)
-  [Function `replace_rule`](#0x2_transfer_policy_replace_rule)
-  [Function `remove_rule`](#0x2_transfer_policy_remove_rule)
-  [Function `has_rule`](#0x2_transfer_policy_has_rule)
-  [Function `rule_count`](#0x2_transfer_policy_rule_count)
-  [Function `rule_config`](#0x2_transfer_policy_rule_config)
-  [Function `new_rule_config`](#0x2_transfer_policy_new_rule_config)
-  [Function `encode_config`](#0x2_transfer_policy_encode_config)
-  [Function `add_receipt`](#0x2_transfer_policy_add_receipt)
-  [Function `receipts_count`](#0x2_transfer_policy_receipts_count)
-  [Function `are_receipts_done`](#0x2_transfer_policy_are_receipts_done)
-  [Function `collected`](#0x2_transfer_policy_collected)
-  [Function `record_collection`](#0x2_transfer_policy_record_collection)
-  [Function `take_collection`](#0x2_transfer_policy_take_collection)
-  [Function `record_payout`](#0x2_transfer_policy_record_payout)
-  [Function `owed`](#0x2_transfer_policy_owed)
-  [Function `rule_key`](#0x2_transfer_policy_rule_key)
-  [Function `amount_of`](#0x2_transfer_policy_amount_of)
-  [Function `set_amount`](#0x2_transfer_policy_set_amount)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/ascii.md#0x1_ascii">0x1::ascii</a>;
<b>use</b> <a href="dependencies/move-stdlib/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">0x1::type_name</a>;
<b>use</b> <a href="collection.md#0x2_collection">0x2::collection</a>;
<b>use</b> <a href="krc_721.md#0x2_krc_721">0x2::krc_721</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="package.md#0x2_package">0x2::package</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
<b>use</b> <a href="vec_map.md#0x2_vec_map">0x2::vec_map</a>;
<b>use</b> <a href="vec_set.md#0x2_vec_set">0x2::vec_set</a>;
</code></pre>



<a name="0x2_transfer_policy_AnyRule"></a>

## Struct `AnyRule`

Type-erased state for one rule. <code>config</code> holds BCS bytes the rule module
encoded; this module cannot interpret them, which is the point.


<pre><code><b>struct</b> <a href="transfer_policy.md#0x2_transfer_policy_AnyRule">AnyRule</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>receipts: <a href="vec_set.md#0x2_vec_set_VecSet">vec_set::VecSet</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>
 Receipt names collected so far.
</dd>
<dt>
<code>config: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 BCS-encoded rule config, written by the rule's own module.
</dd>
</dl>


</details>

<a name="0x2_transfer_policy_TransferPolicy"></a>

## Resource `TransferPolicy`

Registered rules, keyed by rule type name. Type-erased so adding a rule
never changes this struct's layout.


<pre><code><b>struct</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T: store, key&gt; <b>has</b> store, key
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
<code>rules: <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="transfer_policy.md#0x2_transfer_policy_AnyRule">transfer_policy::AnyRule</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>amounts: <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;</code>
</dt>
<dd>
 Per-rule collected totals, keyed the same way as <code>rules</code>. Plain
 amounts so the table above stays type-erased.
</dd>
<dt>
<code>owed: u64</code>
</dt>
<dd>
 Total the owner is owed across all rules, in base units.

 Tracked as a <code>u64</code> rather than a <code>Balance&lt;T&gt;</code>: <code>Balance&lt;T&gt;</code> is
 neither <code><b>copy</b></code> nor <code>drop</code>, so it cannot live in a struct that also
 needs to be shareable, destructible, and reassignable. Rules move
 the actual <code>Coin&lt;T&gt;</code> to the beneficiary directly and record the
 amount here for the owner's accounting, so no custody is needed and
 nothing can be stranded inside the policy.
</dd>
</dl>


</details>

<a name="0x2_transfer_policy_TransferPolicyCap"></a>

## Resource `TransferPolicyCap`

Owner authority over <code>T</code>'s policy. Whoever holds this can change the
rules, so it is the object that must never leak.


<pre><code><b>struct</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">TransferPolicyCap</a>&lt;T: store, key&gt; <b>has</b> store, key
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

<a name="0x2_transfer_policy_Krc721CollectionScope"></a>

## Resource `Krc721CollectionScope`

Shared collection binding for a KRC-721 transfer policy.


<pre><code><b>struct</b> <a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionScope">Krc721CollectionScope</a> <b>has</b> store, key
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
<code>policy_id: <a href="object.md#0x2_object_ID">object::ID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>collection_id: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_transfer_policy_Krc721CollectionPolicyCap"></a>

## Resource `Krc721CollectionPolicyCap`

Collection owner capability bound to one KRC-721 policy and collection.


<pre><code><b>struct</b> <a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">Krc721CollectionPolicyCap</a> <b>has</b> store, key
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
<code>policy_id: <a href="object.md#0x2_object_ID">object::ID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>collection_id: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>cap: <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;<a href="krc_721.md#0x2_krc_721_Nft">krc_721::Nft</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_transfer_policy_TransferRequest"></a>

## Struct `TransferRequest`

A pending transfer awaiting rule resolution. No abilities, on purpose.


<pre><code><b>struct</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;T: key&gt;
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>item: <a href="object.md#0x2_object_ID">object::ID</a></code>
</dt>
<dd>
 The item being traded.
</dd>
<dt>
<code>paid: u64</code>
</dt>
<dd>
 Amount the buyer paid, for percentage-based rules.
</dd>
<dt>
<code>from: <a href="object.md#0x2_object_ID">object::ID</a></code>
</dt>
<dd>
 Where the item came from (e.g. the kiosk id).
</dd>
<dt>
<code>policy_id: <a href="object.md#0x2_object_ID">object::ID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>receipts: <a href="vec_set.md#0x2_vec_set_VecSet">vec_set::VecSet</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>
 Receipt names collected so far.
</dd>
</dl>


</details>

<a name="@Constants_2"></a>

## Constants


<a name="0x2_transfer_policy_E_COLLECTION_MISMATCH"></a>



<pre><code><b>const</b> <a href="transfer_policy.md#0x2_transfer_policy_E_COLLECTION_MISMATCH">E_COLLECTION_MISMATCH</a>: u64 = 15;
</code></pre>



<a name="0x2_transfer_policy_E_COLLECTION_SCOPE_REQUIRED"></a>



<pre><code><b>const</b> <a href="transfer_policy.md#0x2_transfer_policy_E_COLLECTION_SCOPE_REQUIRED">E_COLLECTION_SCOPE_REQUIRED</a>: u64 = 14;
</code></pre>



<a name="0x2_transfer_policy_E_INCOMPLETE_RECEIPTS"></a>

Not every registered rule reported before confirmation.


<pre><code><b>const</b> <a href="transfer_policy.md#0x2_transfer_policy_E_INCOMPLETE_RECEIPTS">E_INCOMPLETE_RECEIPTS</a>: u64 = 0;
</code></pre>



<a name="0x2_transfer_policy_E_NO_RULE"></a>

The rule named by <code>R</code> is not registered on this policy.


<pre><code><b>const</b> <a href="transfer_policy.md#0x2_transfer_policy_E_NO_RULE">E_NO_RULE</a>: u64 = 10;
</code></pre>



<a name="0x2_transfer_policy_E_NO_SUCH_RULE"></a>

A receipt or collection was submitted for an unregistered rule.


<pre><code><b>const</b> <a href="transfer_policy.md#0x2_transfer_policy_E_NO_SUCH_RULE">E_NO_SUCH_RULE</a>: u64 = 12;
</code></pre>



<a name="0x2_transfer_policy_E_POLICY_MISMATCH"></a>



<pre><code><b>const</b> <a href="transfer_policy.md#0x2_transfer_policy_E_POLICY_MISMATCH">E_POLICY_MISMATCH</a>: u64 = 13;
</code></pre>



<a name="0x2_transfer_policy_E_RULE_ALREADY_PRESENT"></a>

A rule exists that this call did not register.


<pre><code><b>const</b> <a href="transfer_policy.md#0x2_transfer_policy_E_RULE_ALREADY_PRESENT">E_RULE_ALREADY_PRESENT</a>: u64 = 11;
</code></pre>



<a name="0x2_transfer_policy_new"></a>

## Function `new`

Create a policy and its owner cap for <code>T</code>.

Sharing the policy is what makes <code>T</code> tradable in a kiosk. A type whose
policy is never shared simply cannot be traded — trading is opt-in, and
the creator revokes it by destroying the policy.

The publisher must be from <code>T</code>'s defining package
(<code><a href="package.md#0x2_package_assert_governs_type">package::assert_governs_type</a></code>): a policy minted from a foreign
publisher would let a package inject royalty rules into trades it does
not own.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_new">new</a>&lt;T: store, key, W: drop&gt;(pub: &<a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;W&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): (<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_new">new</a>&lt;T: key + store, W: drop&gt;(
    pub: &Publisher&lt;W&gt;,
    ctx: &<b>mut</b> TxContext,
): (<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;, <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">TransferPolicyCap</a>&lt;T&gt;) {
    <a href="package.md#0x2_package_assert_governs_type">package::assert_governs_type</a>&lt;T, W&gt;(pub);
    <b>let</b> policy = <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt; {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        rules: <a href="vec_map.md#0x2_vec_map_empty">vec_map::empty</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="transfer_policy.md#0x2_transfer_policy_AnyRule">AnyRule</a>&gt;(),
        amounts: <a href="vec_map.md#0x2_vec_map_empty">vec_map::empty</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;(),
        owed: 0,
    };
    <b>let</b> cap = <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">TransferPolicyCap</a>&lt;T&gt; { id: <a href="object.md#0x2_object_new">object::new</a>(ctx) };
    (policy, cap)
}
</code></pre>



</details>

<a name="0x2_transfer_policy_new_for_krc_721_collection"></a>

## Function `new_for_krc_721_collection`

Create a KRC-721 policy scoped to the collection authorized by <code>cap</code>.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_new_for_krc_721_collection">new_for_krc_721_collection</a>(cap: &<a href="collection.md#0x2_collection_NftCap">collection::NftCap</a>, collection_ref: &<a href="collection.md#0x2_collection_Collection">collection::Collection</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): (<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;<a href="krc_721.md#0x2_krc_721_Nft">krc_721::Nft</a>&gt;, <a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionScope">transfer_policy::Krc721CollectionScope</a>, <a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">transfer_policy::Krc721CollectionPolicyCap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_new_for_krc_721_collection">new_for_krc_721_collection</a>(
    cap: &NftCap,
    collection_ref: &Collection,
    ctx: &<b>mut</b> TxContext,
): (<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;Nft&gt;, <a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionScope">Krc721CollectionScope</a>, <a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">Krc721CollectionPolicyCap</a>) {
    <b>let</b> collection_id = <a href="collection.md#0x2_collection_collection_id">collection::collection_id</a>(collection_ref);
    <b>assert</b>!(<a href="collection.md#0x2_collection_cap_collection_id">collection::cap_collection_id</a>(cap) == collection_id, <a href="transfer_policy.md#0x2_transfer_policy_E_COLLECTION_MISMATCH">E_COLLECTION_MISMATCH</a>);
    <b>let</b> policy = <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;Nft&gt; {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        rules: <a href="vec_map.md#0x2_vec_map_empty">vec_map::empty</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="transfer_policy.md#0x2_transfer_policy_AnyRule">AnyRule</a>&gt;(),
        amounts: <a href="vec_map.md#0x2_vec_map_empty">vec_map::empty</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;(),
        owed: 0,
    };
    <b>let</b> policy_id = <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&policy.id);
    <b>let</b> scope = <a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionScope">Krc721CollectionScope</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        policy_id,
        collection_id,
    };
    <b>let</b> cap = <a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">Krc721CollectionPolicyCap</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        policy_id,
        collection_id,
        cap: <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">TransferPolicyCap</a>&lt;Nft&gt; { id: <a href="object.md#0x2_object_new">object::new</a>(ctx) },
    };
    (policy, scope, cap)
}
</code></pre>



</details>

<a name="0x2_transfer_policy_share"></a>

## Function `share`

Share the policy, making <code>T</code> tradable.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_share">share</a>&lt;T: store, key&gt;(policy: <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_share">share</a>&lt;T: key + store&gt;(policy: <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;) {
    <a href="transfer.md#0x2_transfer_share_object">transfer::share_object</a>(policy);
}
</code></pre>



</details>

<a name="0x2_transfer_policy_destroy"></a>

## Function `destroy`

Destroy the policy, making <code>T</code> permanently non-tradable. Any pending
request for <code>T</code> can no longer confirm.

No value is stranded: rules pay beneficiaries directly, so the only thing
lost is the <code>owed</code> accounting figure.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_destroy">destroy</a>&lt;T: store, key&gt;(policy: <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_destroy">destroy</a>&lt;T: key + store&gt;(policy: <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;) {
    <b>let</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a> { id, rules, amounts, owed } = policy;
    <b>let</b> _ = rules;
    <b>let</b> _ = amounts;
    <b>let</b> _ = owed;
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
}
</code></pre>



</details>

<a name="0x2_transfer_policy_transfer_cap"></a>

## Function `transfer_cap`

Transfer the owner cap, handing over full control of <code>T</code>'s rules.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_transfer_cap">transfer_cap</a>&lt;T: store, key&gt;(cap: <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;T&gt;, recipient: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_transfer_cap">transfer_cap</a>&lt;T: key + store&gt;(cap: <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">TransferPolicyCap</a>&lt;T&gt;, recipient: <b>address</b>) {
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(cap, recipient);
}
</code></pre>



</details>

<a name="0x2_transfer_policy_freeze_cap"></a>

## Function `freeze_cap`

Freeze the owner cap so rules can never be changed again.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_freeze_cap">freeze_cap</a>&lt;T: store, key&gt;(cap: <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_freeze_cap">freeze_cap</a>&lt;T: key + store&gt;(cap: <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">TransferPolicyCap</a>&lt;T&gt;) {
    <a href="transfer.md#0x2_transfer_public_freeze_object">transfer::public_freeze_object</a>(cap);
}
</code></pre>



</details>

<a name="0x2_transfer_policy_share_krc_721_scope"></a>

## Function `share_krc_721_scope`

Share collection scope information for KRC-721 trade entry points.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_share_krc_721_scope">share_krc_721_scope</a>(scope: <a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionScope">transfer_policy::Krc721CollectionScope</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_share_krc_721_scope">share_krc_721_scope</a>(scope: <a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionScope">Krc721CollectionScope</a>) {
    <a href="transfer.md#0x2_transfer_share_object">transfer::share_object</a>(scope);
}
</code></pre>



</details>

<a name="0x2_transfer_policy_transfer_krc_721_collection_cap"></a>

## Function `transfer_krc_721_collection_cap`

Transfer control of a collection-scoped KRC-721 policy.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_transfer_krc_721_collection_cap">transfer_krc_721_collection_cap</a>(cap: <a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">transfer_policy::Krc721CollectionPolicyCap</a>, recipient: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_transfer_krc_721_collection_cap">transfer_krc_721_collection_cap</a>(
    cap: <a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">Krc721CollectionPolicyCap</a>,
    recipient: <b>address</b>,
) {
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(cap, recipient);
}
</code></pre>



</details>

<a name="0x2_transfer_policy_krc_721_scope_collection_id"></a>

## Function `krc_721_scope_collection_id`

The collection governed by a KRC-721 scope.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_krc_721_scope_collection_id">krc_721_scope_collection_id</a>(scope: &<a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionScope">transfer_policy::Krc721CollectionScope</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_krc_721_scope_collection_id">krc_721_scope_collection_id</a>(scope: &<a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionScope">Krc721CollectionScope</a>): <b>address</b> {
    scope.collection_id
}
</code></pre>



</details>

<a name="0x2_transfer_policy_add_krc_721_collection_rule"></a>

## Function `add_krc_721_collection_rule`

Replace or register a rule on a collection-scoped KRC-721 policy.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_add_krc_721_collection_rule">add_krc_721_collection_rule</a>&lt;Rule: drop&gt;(rule: Rule, policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;<a href="krc_721.md#0x2_krc_721_Nft">krc_721::Nft</a>&gt;, cap: &<a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">transfer_policy::Krc721CollectionPolicyCap</a>, config: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_add_krc_721_collection_rule">add_krc_721_collection_rule</a>&lt;Rule: drop&gt;(
    rule: Rule,
    policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;Nft&gt;,
    cap: &<a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">Krc721CollectionPolicyCap</a>,
    config: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    <a href="transfer_policy.md#0x2_transfer_policy_assert_krc_721_policy_cap">assert_krc_721_policy_cap</a>(policy, cap);
    <a href="transfer_policy.md#0x2_transfer_policy_add_rule">add_rule</a>&lt;Rule, Nft&gt;(rule, policy, &cap.cap, config);
}
</code></pre>



</details>

<a name="0x2_transfer_policy_replace_krc_721_collection_rule"></a>

## Function `replace_krc_721_collection_rule`

Update a rule on a collection-scoped KRC-721 policy.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_replace_krc_721_collection_rule">replace_krc_721_collection_rule</a>&lt;Rule: drop&gt;(rule: Rule, policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;<a href="krc_721.md#0x2_krc_721_Nft">krc_721::Nft</a>&gt;, cap: &<a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">transfer_policy::Krc721CollectionPolicyCap</a>, config: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_replace_krc_721_collection_rule">replace_krc_721_collection_rule</a>&lt;Rule: drop&gt;(
    rule: Rule,
    policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;Nft&gt;,
    cap: &<a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">Krc721CollectionPolicyCap</a>,
    config: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    <a href="transfer_policy.md#0x2_transfer_policy_assert_krc_721_policy_cap">assert_krc_721_policy_cap</a>(policy, cap);
    <a href="transfer_policy.md#0x2_transfer_policy_replace_rule">replace_rule</a>&lt;Rule, Nft&gt;(rule, policy, &cap.cap, config);
}
</code></pre>



</details>

<a name="0x2_transfer_policy_remove_krc_721_collection_rule"></a>

## Function `remove_krc_721_collection_rule`

Remove a rule from a collection-scoped KRC-721 policy.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_remove_krc_721_collection_rule">remove_krc_721_collection_rule</a>&lt;Rule: drop&gt;(rule: Rule, policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;<a href="krc_721.md#0x2_krc_721_Nft">krc_721::Nft</a>&gt;, cap: &<a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">transfer_policy::Krc721CollectionPolicyCap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_remove_krc_721_collection_rule">remove_krc_721_collection_rule</a>&lt;Rule: drop&gt;(
    rule: Rule,
    policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;Nft&gt;,
    cap: &<a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">Krc721CollectionPolicyCap</a>,
) {
    <a href="transfer_policy.md#0x2_transfer_policy_assert_krc_721_policy_cap">assert_krc_721_policy_cap</a>(policy, cap);
    <a href="transfer_policy.md#0x2_transfer_policy_remove_rule">remove_rule</a>&lt;Rule, Nft&gt;(rule, policy, &cap.cap);
}
</code></pre>



</details>

<a name="0x2_transfer_policy_assert_krc_721_policy_cap"></a>

## Function `assert_krc_721_policy_cap`



<pre><code><b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_assert_krc_721_policy_cap">assert_krc_721_policy_cap</a>(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;<a href="krc_721.md#0x2_krc_721_Nft">krc_721::Nft</a>&gt;, cap: &<a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">transfer_policy::Krc721CollectionPolicyCap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_assert_krc_721_policy_cap">assert_krc_721_policy_cap</a>(
    policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;Nft&gt;,
    cap: &<a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionPolicyCap">Krc721CollectionPolicyCap</a>,
) {
    <b>assert</b>!(cap.policy_id == <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&policy.id), <a href="transfer_policy.md#0x2_transfer_policy_E_POLICY_MISMATCH">E_POLICY_MISMATCH</a>);
}
</code></pre>



</details>

<a name="0x2_transfer_policy_id"></a>

## Function `id`

The policy's id.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_id">id</a>&lt;T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;): <a href="object.md#0x2_object_ID">object::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_id">id</a>&lt;T: key + store&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;): ID {
    <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&policy.id)
}
</code></pre>



</details>

<a name="0x2_transfer_policy_cap_id"></a>

## Function `cap_id`

The owner cap's id.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_cap_id">cap_id</a>&lt;T: store, key&gt;(cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;T&gt;): <a href="object.md#0x2_object_ID">object::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_cap_id">cap_id</a>&lt;T: key + store&gt;(cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">TransferPolicyCap</a>&lt;T&gt;): ID {
    <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&cap.id)
}
</code></pre>



</details>

<a name="0x2_transfer_policy_new_request"></a>

## Function `new_request`

Open a request to transfer <code>item</code> for <code>paid</code>. The returned value has no
abilities and must be resolved by <code>confirm_request</code> before the
transaction can end.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_new_request">new_request</a>&lt;T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, item: <a href="object.md#0x2_object_ID">object::ID</a>, paid: u64, from: <a href="object.md#0x2_object_ID">object::ID</a>): <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_new_request">new_request</a>&lt;T: key + store&gt;(
    policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;,
    item: ID,
    paid: u64,
    from: ID,
): <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;T&gt; {
    <b>assert</b>!(!<a href="transfer_policy.md#0x2_transfer_policy_is_krc_721_type">is_krc_721_type</a>&lt;T&gt;(), <a href="transfer_policy.md#0x2_transfer_policy_E_COLLECTION_SCOPE_REQUIRED">E_COLLECTION_SCOPE_REQUIRED</a>);
    <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;T&gt; {
        item,
        paid,
        from,
        policy_id: <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&policy.id),
        receipts: <a href="vec_set.md#0x2_vec_set_empty">vec_set::empty</a>(),
    }
}
</code></pre>



</details>

<a name="0x2_transfer_policy_is_krc_721_type"></a>

## Function `is_krc_721_type`



<pre><code><b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_is_krc_721_type">is_krc_721_type</a>&lt;T: store, key&gt;(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_is_krc_721_type">is_krc_721_type</a>&lt;T: key + store&gt;(): bool {
    <b>let</b> actual_name = <a href="dependencies/move-stdlib/type_name.md#0x1_type_name_get">type_name::get</a>&lt;T&gt;();
    <b>let</b> nft_type_name = <a href="dependencies/move-stdlib/type_name.md#0x1_type_name_get">type_name::get</a>&lt;Nft&gt;();
    *<a href="dependencies/move-stdlib/type_name.md#0x1_type_name_borrow_string">type_name::borrow_string</a>(&actual_name)
        == *<a href="dependencies/move-stdlib/type_name.md#0x1_type_name_borrow_string">type_name::borrow_string</a>(&nft_type_name)
}
</code></pre>



</details>

<a name="0x2_transfer_policy_new_krc_721_request"></a>

## Function `new_krc_721_request`

Open a request for a KRC-721 NFT after checking its collection against
the policy scope.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_new_krc_721_request">new_krc_721_request</a>(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;<a href="krc_721.md#0x2_krc_721_Nft">krc_721::Nft</a>&gt;, scope: &<a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionScope">transfer_policy::Krc721CollectionScope</a>, nft: &<a href="krc_721.md#0x2_krc_721_Nft">krc_721::Nft</a>, paid: u64, from: <a href="object.md#0x2_object_ID">object::ID</a>): <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;<a href="krc_721.md#0x2_krc_721_Nft">krc_721::Nft</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_new_krc_721_request">new_krc_721_request</a>(
    policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;Nft&gt;,
    scope: &<a href="transfer_policy.md#0x2_transfer_policy_Krc721CollectionScope">Krc721CollectionScope</a>,
    nft: &Nft,
    paid: u64,
    from: ID,
): <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;Nft&gt; {
    <b>assert</b>!(scope.policy_id == <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&policy.id), <a href="transfer_policy.md#0x2_transfer_policy_E_POLICY_MISMATCH">E_POLICY_MISMATCH</a>);
    <b>assert</b>!(scope.collection_id == <a href="krc_721.md#0x2_krc_721_collection_of">krc_721::collection_of</a>(nft), <a href="transfer_policy.md#0x2_transfer_policy_E_COLLECTION_MISMATCH">E_COLLECTION_MISMATCH</a>);
    <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;Nft&gt; {
        item: <a href="krc_721.md#0x2_krc_721_id">krc_721::id</a>(nft),
        paid,
        from,
        policy_id: <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&policy.id),
        receipts: <a href="vec_set.md#0x2_vec_set_empty">vec_set::empty</a>(),
    }
}
</code></pre>



</details>

<a name="0x2_transfer_policy_confirm_request"></a>

## Function `confirm_request`

Resolve a request, consuming the hot potato.

Aborts unless every registered rule reported. This is the one point
where "the buyer paid" becomes "the rules are satisfied", and it is
reachable only while holding a request.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_confirm_request">confirm_request</a>&lt;T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, request: <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;): (<a href="object.md#0x2_object_ID">object::ID</a>, u64, <a href="object.md#0x2_object_ID">object::ID</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_confirm_request">confirm_request</a>&lt;T: key + store&gt;(
    policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;,
    request: <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;T&gt;,
): (ID, u64, ID) {
    <a href="transfer_policy.md#0x2_transfer_policy_assert_request_policy">assert_request_policy</a>(policy, &request);
    <b>assert</b>!(<a href="transfer_policy.md#0x2_transfer_policy_are_receipts_complete">are_receipts_complete</a>(policy, &request), <a href="transfer_policy.md#0x2_transfer_policy_E_INCOMPLETE_RECEIPTS">E_INCOMPLETE_RECEIPTS</a>);
    <b>let</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a> { item, paid, from, policy_id: _, receipts } = request;
    <b>let</b> _ = receipts;
    (item, paid, from)
}
</code></pre>



</details>

<a name="0x2_transfer_policy_are_receipts_complete"></a>

## Function `are_receipts_complete`

True iff every registered rule has reported on <code>request</code>.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_are_receipts_complete">are_receipts_complete</a>&lt;T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_are_receipts_complete">are_receipts_complete</a>&lt;T: key + store&gt;(
    policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;,
    request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;T&gt;,
): bool {
    <b>let</b> keys = <a href="vec_map.md#0x2_vec_map_keys">vec_map::keys</a>(&policy.rules);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&keys)) {
        <b>let</b> k = *<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&keys, i);
        <b>if</b> (!<a href="vec_set.md#0x2_vec_set_contains">vec_set::contains</a>(&request.receipts, &k)) {
            <b>return</b> <b>false</b>
        };
        i = i + 1;
    };
    <b>true</b>
}
</code></pre>



</details>

<a name="0x2_transfer_policy_assert_request_policy"></a>

## Function `assert_request_policy`

Abort unless <code>request</code> was created by <code>policy</code>.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_assert_request_policy">assert_request_policy</a>&lt;T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_assert_request_policy">assert_request_policy</a>&lt;T: key + store&gt;(
    policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;,
    request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;T&gt;,
) {
    <b>assert</b>!(
        request.policy_id == <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&policy.id),
        <a href="transfer_policy.md#0x2_transfer_policy_E_POLICY_MISMATCH">E_POLICY_MISMATCH</a>
    );
}
</code></pre>



</details>

<a name="0x2_transfer_policy_item"></a>

## Function `item`

The item being traded.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_item">item</a>&lt;T: store, key&gt;(request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;): <a href="object.md#0x2_object_ID">object::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_item">item</a>&lt;T: key + store&gt;(request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;T&gt;): ID {
    request.item
}
</code></pre>



</details>

<a name="0x2_transfer_policy_paid"></a>

## Function `paid`

The amount paid, for rules that charge a percentage.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_paid">paid</a>&lt;T: store, key&gt;(request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_paid">paid</a>&lt;T: key + store&gt;(request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;T&gt;): u64 {
    request.paid
}
</code></pre>



</details>

<a name="0x2_transfer_policy_from"></a>

## Function `from`

The source object (e.g. a kiosk).


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_from">from</a>&lt;T: store, key&gt;(request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;): <a href="object.md#0x2_object_ID">object::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_from">from</a>&lt;T: key + store&gt;(request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;T&gt;): ID {
    request.from
}
</code></pre>



</details>

<a name="0x2_transfer_policy_add_rule"></a>

## Function `add_rule`

Register <code>R</code> on <code>policy</code> with an encoded <code>config</code>.

Requires the type owner's <code><a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">TransferPolicyCap</a>&lt;T&gt;</code>, and <code>R</code>'s witness.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_add_rule">add_rule</a>&lt;Rule: drop, T: store, key&gt;(_rule: Rule, policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, _cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;T&gt;, config: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_add_rule">add_rule</a>&lt;Rule: drop, T: key + store&gt;(
    _rule: Rule,
    policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;,
    _cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">TransferPolicyCap</a>&lt;T&gt;,
    config: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    <b>let</b> key = <a href="transfer_policy.md#0x2_transfer_policy_rule_key">rule_key</a>&lt;Rule&gt;();
    <b>assert</b>!(!<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&policy.rules, &key), <a href="transfer_policy.md#0x2_transfer_policy_E_RULE_ALREADY_PRESENT">E_RULE_ALREADY_PRESENT</a>);
    <a href="vec_map.md#0x2_vec_map_insert">vec_map::insert</a>(&<b>mut</b> policy.rules, key, <a href="transfer_policy.md#0x2_transfer_policy_new_rule_config">new_rule_config</a>(config));
}
</code></pre>



</details>

<a name="0x2_transfer_policy_replace_rule"></a>

## Function `replace_rule`

Replace the config of an already-registered rule.

Removes first: <code><a href="vec_map.md#0x2_vec_map_insert">vec_map::insert</a></code> aborts on a duplicate key.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_replace_rule">replace_rule</a>&lt;Rule: drop, T: store, key&gt;(_rule: Rule, policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, _cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;T&gt;, config: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_replace_rule">replace_rule</a>&lt;Rule: drop, T: key + store&gt;(
    _rule: Rule,
    policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;,
    _cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">TransferPolicyCap</a>&lt;T&gt;,
    config: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    <b>let</b> key = <a href="transfer_policy.md#0x2_transfer_policy_rule_key">rule_key</a>&lt;Rule&gt;();
    <b>assert</b>!(<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&policy.rules, &key), <a href="transfer_policy.md#0x2_transfer_policy_E_NO_RULE">E_NO_RULE</a>);
    <b>let</b> (removed_key, removed) = <a href="vec_map.md#0x2_vec_map_remove">vec_map::remove</a>(&<b>mut</b> policy.rules, &key);
    <b>let</b> _ = removed_key;
    <b>let</b> _ = removed;
    <a href="vec_map.md#0x2_vec_map_insert">vec_map::insert</a>(&<b>mut</b> policy.rules, key, <a href="transfer_policy.md#0x2_transfer_policy_new_rule_config">new_rule_config</a>(config));
}
</code></pre>



</details>

<a name="0x2_transfer_policy_remove_rule"></a>

## Function `remove_rule`

Remove <code>R</code>. Receipts on past requests are unaffected because those were
already resolved.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_remove_rule">remove_rule</a>&lt;Rule: drop, T: store, key&gt;(_rule: Rule, policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, _cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_remove_rule">remove_rule</a>&lt;Rule: drop, T: key + store&gt;(
    _rule: Rule,
    policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;,
    _cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">TransferPolicyCap</a>&lt;T&gt;,
) {
    <b>let</b> key = <a href="transfer_policy.md#0x2_transfer_policy_rule_key">rule_key</a>&lt;Rule&gt;();
    <b>assert</b>!(<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&policy.rules, &key), <a href="transfer_policy.md#0x2_transfer_policy_E_NO_RULE">E_NO_RULE</a>);
    <b>let</b> (removed_key, removed) = <a href="vec_map.md#0x2_vec_map_remove">vec_map::remove</a>(&<b>mut</b> policy.rules, &key);
    <b>let</b> _ = removed_key;
    <b>let</b> _ = removed;
}
</code></pre>



</details>

<a name="0x2_transfer_policy_has_rule"></a>

## Function `has_rule`

True iff <code>R</code> is registered.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_has_rule">has_rule</a>&lt;Rule: drop, T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_has_rule">has_rule</a>&lt;Rule: drop, T: key + store&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;): bool {
    <a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&policy.rules, &<a href="transfer_policy.md#0x2_transfer_policy_rule_key">rule_key</a>&lt;Rule&gt;())
}
</code></pre>



</details>

<a name="0x2_transfer_policy_rule_count"></a>

## Function `rule_count`

Count of registered rules.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_rule_count">rule_count</a>&lt;T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_rule_count">rule_count</a>&lt;T: key + store&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;): u64 {
    <a href="vec_map.md#0x2_vec_map_size">vec_map::size</a>(&policy.rules)
}
</code></pre>



</details>

<a name="0x2_transfer_policy_rule_config"></a>

## Function `rule_config`

The encoded config bytes for <code>R</code>, for the rule module to decode.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_rule_config">rule_config</a>&lt;Rule: drop, T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_rule_config">rule_config</a>&lt;Rule: drop, T: key + store&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> key = <a href="transfer_policy.md#0x2_transfer_policy_rule_key">rule_key</a>&lt;Rule&gt;();
    <b>assert</b>!(<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&policy.rules, &key), <a href="transfer_policy.md#0x2_transfer_policy_E_NO_RULE">E_NO_RULE</a>);
    <b>let</b> config = <a href="vec_map.md#0x2_vec_map_get">vec_map::get</a>(&policy.rules, &key).config;
    config
}
</code></pre>



</details>

<a name="0x2_transfer_policy_new_rule_config"></a>

## Function `new_rule_config`

Build a fresh <code><a href="transfer_policy.md#0x2_transfer_policy_AnyRule">AnyRule</a></code> with no receipts.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_new_rule_config">new_rule_config</a>(config: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="transfer_policy.md#0x2_transfer_policy_AnyRule">transfer_policy::AnyRule</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_new_rule_config">new_rule_config</a>(config: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="transfer_policy.md#0x2_transfer_policy_AnyRule">AnyRule</a> {
    <a href="transfer_policy.md#0x2_transfer_policy_AnyRule">AnyRule</a> {
        receipts: <a href="vec_set.md#0x2_vec_set_empty">vec_set::empty</a>(),
        config,
    }
}
</code></pre>



</details>

<a name="0x2_transfer_policy_encode_config"></a>

## Function `encode_config`

Encode <code>config</code> as BCS bytes for <code>add_rule</code>.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_encode_config">encode_config</a>&lt;T: drop&gt;(config: &T): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_encode_config">encode_config</a>&lt;T: drop&gt;(config: &T): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="dependencies/move-stdlib/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(config)
}
</code></pre>



</details>

<a name="0x2_transfer_policy_add_receipt"></a>

## Function `add_receipt`

Submit the receipt for <code>R</code>, unblocking <code>confirm_request</code> for it.

The only path that can mark a rule satisfied, and it requires <code>R</code>'s
witness, so a rule's module is the only thing that can declare it paid.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_add_receipt">add_receipt</a>&lt;Rule: drop, T: store, key&gt;(_rule: Rule, policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, request: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_add_receipt">add_receipt</a>&lt;Rule: drop, T: key + store&gt;(
    _rule: Rule,
    policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;,
    request: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;T&gt;,
) {
    <a href="transfer_policy.md#0x2_transfer_policy_assert_request_policy">assert_request_policy</a>(policy, request);
    <b>let</b> key = <a href="transfer_policy.md#0x2_transfer_policy_rule_key">rule_key</a>&lt;Rule&gt;();
    <b>assert</b>!(<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&policy.rules, &key), <a href="transfer_policy.md#0x2_transfer_policy_E_NO_SUCH_RULE">E_NO_SUCH_RULE</a>);
    <a href="vec_set.md#0x2_vec_set_insert">vec_set::insert</a>(&<b>mut</b> <a href="vec_map.md#0x2_vec_map_get_mut">vec_map::get_mut</a>(&<b>mut</b> policy.rules, &key).receipts, key);
    <a href="vec_set.md#0x2_vec_set_insert">vec_set::insert</a>(&<b>mut</b> request.receipts, key);
}
</code></pre>



</details>

<a name="0x2_transfer_policy_receipts_count"></a>

## Function `receipts_count`

Number of rules that have reported on <code>request</code>.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_receipts_count">receipts_count</a>&lt;T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_receipts_count">receipts_count</a>&lt;T: key + store&gt;(
    policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;,
    request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;T&gt;,
): u64 {
    <b>let</b> keys = <a href="vec_map.md#0x2_vec_map_keys">vec_map::keys</a>(&policy.rules);
    <b>let</b> i = 0;
    <b>let</b> done = 0;
    <b>while</b> (i &lt; <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&keys)) {
        <b>let</b> k = *<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&keys, i);
        <b>if</b> (<a href="vec_set.md#0x2_vec_set_contains">vec_set::contains</a>(&request.receipts, &k)) {
            done = done + 1;
        };
        i = i + 1;
    };
    done
}
</code></pre>



</details>

<a name="0x2_transfer_policy_are_receipts_done"></a>

## Function `are_receipts_done`

True iff every registered rule has reported.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_are_receipts_done">are_receipts_done</a>&lt;T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">transfer_policy::TransferRequest</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_are_receipts_done">are_receipts_done</a>&lt;T: key + store&gt;(
    policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;,
    request: &<a href="transfer_policy.md#0x2_transfer_policy_TransferRequest">TransferRequest</a>&lt;T&gt;,
): bool {
    <a href="transfer_policy.md#0x2_transfer_policy_receipts_count">receipts_count</a>(policy, request) == <a href="vec_map.md#0x2_vec_map_size">vec_map::size</a>(&policy.rules)
}
</code></pre>



</details>

<a name="0x2_transfer_policy_collected"></a>

## Function `collected`

Amount <code>R</code> has collected so far.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_collected">collected</a>&lt;Rule: drop, T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_collected">collected</a>&lt;Rule: drop, T: key + store&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;): u64 {
    <b>let</b> key = <a href="transfer_policy.md#0x2_transfer_policy_rule_key">rule_key</a>&lt;Rule&gt;();
    <b>assert</b>!(<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&policy.rules, &key), <a href="transfer_policy.md#0x2_transfer_policy_E_NO_RULE">E_NO_RULE</a>);
    <a href="transfer_policy.md#0x2_transfer_policy_amount_of">amount_of</a>(policy, &key)
}
</code></pre>



</details>

<a name="0x2_transfer_policy_record_collection"></a>

## Function `record_collection`

Record that <code>R</code> collected <code>amount</code>. Rule modules call this after moving
the value to the beneficiary.

Accumulates rather than overwrites, and goes through remove-then-insert
because <code><a href="vec_map.md#0x2_vec_map_insert">vec_map::insert</a></code> aborts on an existing key.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_record_collection">record_collection</a>&lt;Rule: drop, T: store, key&gt;(_rule: Rule, policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_record_collection">record_collection</a>&lt;Rule: drop, T: key + store&gt;(
    _rule: Rule,
    policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;,
    amount: u64,
) {
    <b>let</b> key = <a href="transfer_policy.md#0x2_transfer_policy_rule_key">rule_key</a>&lt;Rule&gt;();
    <b>assert</b>!(<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&policy.rules, &key), <a href="transfer_policy.md#0x2_transfer_policy_E_NO_SUCH_RULE">E_NO_SUCH_RULE</a>);
    <b>let</b> already = <a href="transfer_policy.md#0x2_transfer_policy_amount_of">amount_of</a>(policy, &key);
    <a href="transfer_policy.md#0x2_transfer_policy_set_amount">set_amount</a>(policy, &key, already + amount);
}
</code></pre>



</details>

<a name="0x2_transfer_policy_take_collection"></a>

## Function `take_collection`

Take back what <code>R</code> collected. Owner-only via <code><a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">TransferPolicyCap</a>&lt;T&gt;</code>.
The caller pays the returned amount out.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_take_collection">take_collection</a>&lt;Rule: drop, T: store, key&gt;(_rule: Rule, policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, _cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">transfer_policy::TransferPolicyCap</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_take_collection">take_collection</a>&lt;Rule: drop, T: key + store&gt;(
    _rule: Rule,
    policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;,
    _cap: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicyCap">TransferPolicyCap</a>&lt;T&gt;,
): u64 {
    <b>let</b> key = <a href="transfer_policy.md#0x2_transfer_policy_rule_key">rule_key</a>&lt;Rule&gt;();
    <b>assert</b>!(<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&policy.rules, &key), <a href="transfer_policy.md#0x2_transfer_policy_E_NO_SUCH_RULE">E_NO_SUCH_RULE</a>);
    <b>let</b> amount = <a href="transfer_policy.md#0x2_transfer_policy_amount_of">amount_of</a>(policy, &key);
    <a href="transfer_policy.md#0x2_transfer_policy_set_amount">set_amount</a>(policy, &key, 0);
    amount
}
</code></pre>



</details>

<a name="0x2_transfer_policy_record_payout"></a>

## Function `record_payout`

Record that the owner has been paid <code>amount</code>, keeping the running total
a rule author can query for accounting.

Deliberately not custody: rule modules move the <code>Coin&lt;T&gt;</code> to the
beneficiary themselves and call this for the ledger. Holding real value
inside the policy would need a <code>Balance&lt;T&gt;</code>, which has neither <code><b>copy</b></code>
nor <code>drop</code> and so cannot live in a shareable struct.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_record_payout">record_payout</a>&lt;T: store, key&gt;(policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_record_payout">record_payout</a>&lt;T: key + store&gt;(
    policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;,
    amount: u64,
) {
    policy.owed = policy.owed + amount;
}
</code></pre>



</details>

<a name="0x2_transfer_policy_owed"></a>

## Function `owed`

Total the owner has been paid across the policy's lifetime.


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_owed">owed</a>&lt;T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_owed">owed</a>&lt;T: key + store&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;): u64 {
    policy.owed
}
</code></pre>



</details>

<a name="0x2_transfer_policy_rule_key"></a>

## Function `rule_key`

Canonical registry key for rule type <code>R</code>: its type-name bytes.


<pre><code><b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_rule_key">rule_key</a>&lt;Rule: drop&gt;(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_rule_key">rule_key</a>&lt;Rule: drop&gt;(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> name = std::type_name::get&lt;Rule&gt;();
    *std::ascii::as_bytes(std::type_name::borrow_string(&name))
}
</code></pre>



</details>

<a name="0x2_transfer_policy_amount_of"></a>

## Function `amount_of`

Read a ledger entry without requiring the rule to be registered.


<pre><code><b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_amount_of">amount_of</a>&lt;T: store, key&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, key: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_amount_of">amount_of</a>&lt;T: key + store&gt;(policy: &<a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;, key: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64 {
    <b>if</b> (<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&policy.amounts, key)) {
        *<a href="vec_map.md#0x2_vec_map_get">vec_map::get</a>(&policy.amounts, key)
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a name="0x2_transfer_policy_set_amount"></a>

## Function `set_amount`

Write a ledger entry. <code><a href="vec_map.md#0x2_vec_map_insert">vec_map::insert</a></code> aborts on a duplicate key, so an
existing entry is removed first.


<pre><code><b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_set_amount">set_amount</a>&lt;T: store, key&gt;(policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">transfer_policy::TransferPolicy</a>&lt;T&gt;, key: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transfer_policy.md#0x2_transfer_policy_set_amount">set_amount</a>&lt;T: key + store&gt;(policy: &<b>mut</b> <a href="transfer_policy.md#0x2_transfer_policy_TransferPolicy">TransferPolicy</a>&lt;T&gt;, key: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: u64) {
    <b>if</b> (<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&policy.amounts, key)) {
        <b>let</b> (removed_key, removed) = <a href="vec_map.md#0x2_vec_map_remove">vec_map::remove</a>(&<b>mut</b> policy.amounts, key);
        <b>let</b> _ = removed_key;
        <b>let</b> _ = removed;
    };
    <a href="vec_map.md#0x2_vec_map_insert">vec_map::insert</a>(&<b>mut</b> policy.amounts, *key, value);
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
