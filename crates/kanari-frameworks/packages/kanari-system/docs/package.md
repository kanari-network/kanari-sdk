
<a name="0x2_package"></a>

# Module `0x2::package`

Type ownership, mirroring <code>sui::package</code>.

<code><a href="package.md#0x2_package_Publisher">Publisher</a>&lt;W&gt;</code> is authority over the one-time witness type <code>W</code>, which the
defining module alone can construct. Sui keys the publisher on the witness
rather than on the object type, and so does this: it is the only arrangement
that keeps the token type free of <code>drop</code>.

The alternative — <code><a href="package.md#0x2_package_Publisher">Publisher</a>&lt;TokenType&gt;</code> bound from the token itself — looks
tighter but is unusable. The witness has to *be* the type it claims, so the
token would need <code>drop</code>, and a droppable token can be silently discarded by
whoever holds it. That bypasses supply accounting entirely: a burned token
never returns its unit to the cap, and the destroy leaves no trace. Keeping
<code>drop</code> off the token is worth more than the extra binding.

The consequence, shared with Sui, is that consumers take an *unbound*
<code>&<a href="package.md#0x2_package_Publisher">Publisher</a></code>: holding a publisher proves you own *some* type in your own
package, not that you own the specific <code>T</code> being configured. Sui closes the
remaining gap with a host-side one-Display-per-type registry; this runtime
has no such registry, so <code><a href="display.md#0x2_display">display</a>.<b>move</b></code> documents the caveat.


-  [Resource `Publisher`](#0x2_package_Publisher)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x2_package_new)
-  [Function `id`](#0x2_package_id)
-  [Function `publisher_address`](#0x2_package_publisher_address)
-  [Function `type_name`](#0x2_package_type_name)
-  [Function `transfer`](#0x2_package_transfer)
-  [Function `revoke_forever`](#0x2_package_revoke_forever)
-  [Function `type_name_bytes`](#0x2_package_type_name_bytes)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/ascii.md#0x1_ascii">0x1::ascii</a>;
<b>use</b> <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">0x1::type_name</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
<b>use</b> <a href="types.md#0x2_types">0x2::types</a>;
</code></pre>



<a name="0x2_package_Publisher"></a>

## Resource `Publisher`

Owns type <code>T</code>. The <code><b>phantom</b></code> marker means the value carries no data of
<code>T</code>; the type parameter exists purely to scope the authority.


<pre><code><b>struct</b> <a href="package.md#0x2_package_Publisher">Publisher</a>&lt;T&gt; <b>has</b> store, key
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
<code><a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 Witness type name, retained for off-chain discovery of which types
 have a publisher published.
</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_package_E_NOT_ONE_TIME_WITNESS"></a>

Type T is aborted when the witness does not follow the one-time-witness
convention.


<pre><code><b>const</b> <a href="package.md#0x2_package_E_NOT_ONE_TIME_WITNESS">E_NOT_ONE_TIME_WITNESS</a>: u64 = 0;
</code></pre>



<a name="0x2_package_new"></a>

## Function `new`

Claim authority over <code>T</code>. <code>witness</code> must be the one-time witness <code>T</code>
itself; only the defining module can construct it.


<pre><code><b>public</b> <b>fun</b> <a href="package.md#0x2_package_new">new</a>&lt;T: drop&gt;(witness: T, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="package.md#0x2_package_new">new</a>&lt;T: drop&gt;(witness: T, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="package.md#0x2_package_Publisher">Publisher</a>&lt;T&gt; {
    <b>assert</b>!(<a href="types.md#0x2_types_is_one_time_witness">types::is_one_time_witness</a>&lt;T&gt;(&witness), <a href="package.md#0x2_package_E_NOT_ONE_TIME_WITNESS">E_NOT_ONE_TIME_WITNESS</a>);
    <a href="package.md#0x2_package_Publisher">Publisher</a>&lt;T&gt; {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>: <a href="package.md#0x2_package_type_name_bytes">type_name_bytes</a>&lt;T&gt;(),
    }
}
</code></pre>



</details>

<a name="0x2_package_id"></a>

## Function `id`

The publisher's object id.


<pre><code><b>public</b> <b>fun</b> <a href="package.md#0x2_package_id">id</a>&lt;T&gt;(pub: &<a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;T&gt;): <a href="object.md#0x2_object_ID">object::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="package.md#0x2_package_id">id</a>&lt;T&gt;(pub: &<a href="package.md#0x2_package_Publisher">Publisher</a>&lt;T&gt;): <a href="object.md#0x2_object_ID">object::ID</a> {
    <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&pub.id)
}
</code></pre>



</details>

<a name="0x2_package_publisher_address"></a>

## Function `publisher_address`

The publisher's object address.


<pre><code><b>public</b> <b>fun</b> <a href="package.md#0x2_package_publisher_address">publisher_address</a>&lt;T&gt;(pub: &<a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="package.md#0x2_package_publisher_address">publisher_address</a>&lt;T&gt;(pub: &<a href="package.md#0x2_package_Publisher">Publisher</a>&lt;T&gt;): <b>address</b> {
    <a href="object.md#0x2_object_uid_address">object::uid_address</a>(&pub.id)
}
</code></pre>



</details>

<a name="0x2_package_type_name"></a>

## Function `type_name`

The witnessed type's name bytes, for indexers.


<pre><code><b>public</b> <b>fun</b> <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>&lt;T&gt;(pub: &<a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;T&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>&lt;T&gt;(pub: &<a href="package.md#0x2_package_Publisher">Publisher</a>&lt;T&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    pub.<a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>
}
</code></pre>



</details>

<a name="0x2_package_transfer"></a>

## Function `transfer`

Transfer authority over <code>T</code> to another address.


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>&lt;T&gt;(pub: <a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;T&gt;, recipient: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>&lt;T&gt;(pub: <a href="package.md#0x2_package_Publisher">Publisher</a>&lt;T&gt;, recipient: <b>address</b>) {
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(pub, recipient);
}
</code></pre>



</details>

<a name="0x2_package_revoke_forever"></a>

## Function `revoke_forever`

Freeze the publisher permanently, so <code>T</code> can never gain new display or
policy authority.


<pre><code><b>public</b> <b>fun</b> <a href="package.md#0x2_package_revoke_forever">revoke_forever</a>&lt;T&gt;(pub: <a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="package.md#0x2_package_revoke_forever">revoke_forever</a>&lt;T&gt;(pub: <a href="package.md#0x2_package_Publisher">Publisher</a>&lt;T&gt;) {
    <a href="transfer.md#0x2_transfer_public_freeze_object">transfer::public_freeze_object</a>(pub);
}
</code></pre>



</details>

<a name="0x2_package_type_name_bytes"></a>

## Function `type_name_bytes`

Canonical type-name bytes for <code>T</code>, matching <code><a href="dependencies/move-stdlib/type_name.md#0x1_type_name_get">type_name::get</a>&lt;T&gt;</code>.


<pre><code><b>fun</b> <a href="package.md#0x2_package_type_name_bytes">type_name_bytes</a>&lt;T&gt;(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="package.md#0x2_package_type_name_bytes">type_name_bytes</a>&lt;T&gt;(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> name = std::type_name::get&lt;T&gt;();
    *std::ascii::as_bytes(std::type_name::borrow_string(&name))
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
