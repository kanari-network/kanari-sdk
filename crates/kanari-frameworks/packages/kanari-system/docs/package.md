
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
has no such registry. What this module CAN close it with is package-level
binding: type names carry the defining package's address, so
<code>assert_governs_type</code> enforces at runtime that a publisher's witness was
defined in the same package as the type being configured. A publisher from
a foreign package cannot mint <code>Display&lt;T&gt;</code> or <code>TransferPolicy&lt;T&gt;</code> for your
<code>T</code>.


-  [Resource `Publisher`](#0x2_package_Publisher)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x2_package_new)
-  [Function `id`](#0x2_package_id)
-  [Function `publisher_address`](#0x2_package_publisher_address)
-  [Function `type_name`](#0x2_package_type_name)
-  [Function `transfer`](#0x2_package_transfer)
-  [Function `revoke_forever`](#0x2_package_revoke_forever)
-  [Function `type_name_bytes`](#0x2_package_type_name_bytes)
-  [Function `assert_governs_type`](#0x2_package_assert_governs_type)
-  [Function `same_package_as`](#0x2_package_same_package_as)
-  [Function `same_package_type_name`](#0x2_package_same_package_type_name)
-  [Function `package_prefix_len`](#0x2_package_package_prefix_len)


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



<a name="0x2_package_E_WRONG_PACKAGE"></a>

A publisher was used to configure a type defined in a different
package than the publisher's witness.


<pre><code><b>const</b> <a href="package.md#0x2_package_E_WRONG_PACKAGE">E_WRONG_PACKAGE</a>: u64 = 1;
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

<a name="0x2_package_assert_governs_type"></a>

## Function `assert_governs_type`

Abort unless <code>pub</code> was claimed for a type defined in the same package
that defines <code>T</code>.

Consumers of a publisher — <code><a href="display.md#0x2_display">display</a></code>, <code><a href="transfer_policy.md#0x2_transfer_policy">transfer_policy</a></code> — call this:
without it, a package could claim a publisher for its own coin and then
mint a <code>Display&lt;Frog&gt;</code> or <code>TransferPolicy&lt;Frog&gt;</code> for a token type it
does not own. The check compares package addresses parsed from both
type names (<code>0x43</code> in <code>0x43::pet::KanariPet</code>); it cannot detect
same-package cross-module use, which is accepted, matching the
statement in the module header.


<pre><code><b>public</b> <b>fun</b> <a href="package.md#0x2_package_assert_governs_type">assert_governs_type</a>&lt;T, W&gt;(pub: &<a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;W&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="package.md#0x2_package_assert_governs_type">assert_governs_type</a>&lt;T, W&gt;(pub: &<a href="package.md#0x2_package_Publisher">Publisher</a>&lt;W&gt;) {
    <b>assert</b>!(<a href="package.md#0x2_package_same_package_as">same_package_as</a>&lt;T, W&gt;(pub), <a href="package.md#0x2_package_E_WRONG_PACKAGE">E_WRONG_PACKAGE</a>);
}
</code></pre>



</details>

<a name="0x2_package_same_package_as"></a>

## Function `same_package_as`

True when the witness that produced <code>pub</code> was defined in the same
package that defines <code>T</code>. See <code>assert_governs_type</code>.


<pre><code><b>public</b> <b>fun</b> <a href="package.md#0x2_package_same_package_as">same_package_as</a>&lt;T, W&gt;(pub: &<a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;W&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="package.md#0x2_package_same_package_as">same_package_as</a>&lt;T, W&gt;(pub: &<a href="package.md#0x2_package_Publisher">Publisher</a>&lt;W&gt;): bool {
    <a href="package.md#0x2_package_same_package_type_name">same_package_type_name</a>(&pub.<a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>, &<a href="package.md#0x2_package_type_name_bytes">type_name_bytes</a>&lt;T&gt;())
}
</code></pre>



</details>

<a name="0x2_package_same_package_type_name"></a>

## Function `same_package_type_name`

True when two type names name types from the same package: their
address sections, everything up to the first <code>::</code>, are byte-identical.


<pre><code><b>fun</b> <a href="package.md#0x2_package_same_package_type_name">same_package_type_name</a>(a: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="package.md#0x2_package_same_package_type_name">same_package_type_name</a>(a: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool {
    <b>let</b> al = <a href="package.md#0x2_package_package_prefix_len">package_prefix_len</a>(a);
    <b>let</b> bl = <a href="package.md#0x2_package_package_prefix_len">package_prefix_len</a>(b);
    <b>if</b> (al != bl) {
        <b>return</b> <b>false</b>
    };
    <b>let</b> i = 0;
    <b>let</b> same = <b>true</b>;
    <b>while</b> (same && i &lt; al) {
        same = same && (*<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(a, i) == *<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(b, i));
        i = i + 1;
    };
    same
}
</code></pre>



</details>

<a name="0x2_package_package_prefix_len"></a>

## Function `package_prefix_len`

Length of the address section of a type name: <code>3</code> for <code>0x43</code> in
<code>0x43::pet::KanariPet</code>. A name without a separator spans everything.


<pre><code><b>fun</b> <a href="package.md#0x2_package_package_prefix_len">package_prefix_len</a>(name: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="package.md#0x2_package_package_prefix_len">package_prefix_len</a>(name: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64 {
    <b>let</b> n = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(name);
    <b>let</b> i = 0;
    <b>while</b> (i + 1 &lt; n) {
        <b>if</b> (*<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(name, i) == 58 && *<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(name, i + 1) == 58) {
            <b>break</b>
        };
        i = i + 1;
    };
    <b>if</b> (i + 1 &lt; n) { i } <b>else</b> { n }
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
