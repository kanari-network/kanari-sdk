
<a name="0x2_display"></a>

# Module `0x2::display`

Type-keyed metadata, mirroring <code>sui::display</code>.

Sui does not put name/image on each token object. One shared <code><a href="display.md#0x2_display_Display">Display</a>&lt;T&gt;</code>
holds a <code>VecMap&lt;String, String&gt;</code> keyed by the type's name, so a wallet can
render *any* object type without per-object metadata and without the
creator writing a custom viewer. Per-token uniqueness comes from the object
id, not from the metadata.

Every mutation takes an *unbound* <code>&Publisher</code>, matching Sui. Binding it to
<code>&Publisher</code> would be tighter in principle, but the publisher is keyed on
the one-time witness type rather than on <code>T</code> (see <code>kanari_system::package</code>),
so there is no <code>Publisher</code> to bind to. Holding a publisher proves you own
a type in your own package.

Sui additionally enforces one Display per type through a host-side registry,
so a second Display for the same <code>T</code> is rejected at publish time. This
runtime now does the same: <code>new_display</code> claims <code>T</code> in the host's durable
type registry, so the first Display created for a type is the only one that
can ever exist. Combined with the unbound <code>&Publisher&lt;W&gt;</code> argument, that
closes the gap — a publisher for *some* type can no longer mint a
<code><a href="display.md#0x2_display_Display">Display</a>&lt;T&gt;</code> once the real owner has already created one.


-  [Resource `Display`](#0x2_display_Display)
-  [Constants](#@Constants_0)
-  [Function `create_with_name`](#0x2_display_create_with_name)
-  [Function `create_empty`](#0x2_display_create_empty)
-  [Function `create_with_fields`](#0x2_display_create_with_fields)
-  [Function `new_display`](#0x2_display_new_display)
-  [Function `seed_fields`](#0x2_display_seed_fields)
-  [Function `add`](#0x2_display_add)
-  [Function `remove`](#0x2_display_remove)
-  [Function `get`](#0x2_display_get)
-  [Function `get_with_default`](#0x2_display_get_with_default)
-  [Function `field_count`](#0x2_display_field_count)
-  [Function `id`](#0x2_display_id)
-  [Function `display_address`](#0x2_display_display_address)
-  [Function `type_name_of`](#0x2_display_type_name_of)
-  [Function `share`](#0x2_display_share)
-  [Function `transfer`](#0x2_display_transfer)
-  [Function `destroy`](#0x2_display_destroy)
-  [Function `type_name_bytes`](#0x2_display_type_name_bytes)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/ascii.md#0x1_ascii">0x1::ascii</a>;
<b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="dependencies/move-stdlib/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">0x1::type_name</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="package.md#0x2_package">0x2::package</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
<b>use</b> <a href="types.md#0x2_types">0x2::types</a>;
<b>use</b> <a href="vec_map.md#0x2_vec_map">0x2::vec_map</a>;
</code></pre>



<a name="0x2_display_Display"></a>

## Resource `Display`

Shared metadata for every object of type <code>T</code>. <code>T</code> is <code><b>phantom</b></code>: the
value stores no <code>T</code> data, and the parameter exists so that one
<code><a href="display.md#0x2_display_Display">Display</a>&lt;T&gt;</code> cannot be passed where <code><a href="display.md#0x2_display_Display">Display</a>&lt;U&gt;</code> is expected. That is
what makes the type the key.


<pre><code><b>struct</b> <a href="display.md#0x2_display_Display">Display</a>&lt;T: key&gt; <b>has</b> store, key
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
<code>fields: <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>
 <code>T</code>'s canonical type name is always the first entry, so indexers
 can identify the display from its object alone.
</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_display_E_FIELD_COUNT_MISMATCH"></a>

The key/value seed vectors had different lengths.


<pre><code><b>const</b> <a href="display.md#0x2_display_E_FIELD_COUNT_MISMATCH">E_FIELD_COUNT_MISMATCH</a>: u64 = 0;
</code></pre>



<a name="0x2_display_E_PROTECTED_FIELD"></a>

<code>T</code>'s own type-name entry cannot be removed.


<pre><code><b>const</b> <a href="display.md#0x2_display_E_PROTECTED_FIELD">E_PROTECTED_FIELD</a>: u64 = 1;
</code></pre>



<a name="0x2_display_create_with_name"></a>

## Function `create_with_name`

Create a display for <code>T</code> with a <code>name</code> field already set.


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_create_with_name">create_with_name</a>&lt;T: key, W: drop&gt;(pub: &<a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;W&gt;, name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_create_with_name">create_with_name</a>&lt;T: key, W: drop&gt;(
    pub: &Publisher&lt;W&gt;,
    name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    ctx: &<b>mut</b> TxContext,
): <a href="display.md#0x2_display_Display">Display</a>&lt;T&gt; {
    <b>let</b> keys = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;();
    <b>let</b> values = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;();
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> keys, b"name");
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> values, name);
    <a href="display.md#0x2_display_create_with_fields">create_with_fields</a>&lt;T, W&gt;(pub, keys, values, ctx)
}
</code></pre>



</details>

<a name="0x2_display_create_empty"></a>

## Function `create_empty`

Create an empty display for <code>T</code>.


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_create_empty">create_empty</a>&lt;T: key, W: drop&gt;(pub: &<a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;W&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_create_empty">create_empty</a>&lt;T: key, W: drop&gt;(
    pub: &Publisher&lt;W&gt;,
    ctx: &<b>mut</b> TxContext,
): <a href="display.md#0x2_display_Display">Display</a>&lt;T&gt; {
    <b>let</b> keys = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;();
    <b>let</b> values = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;();
    <a href="display.md#0x2_display_new_display">new_display</a>&lt;T, W&gt;(pub, keys, values, ctx)
}
</code></pre>



</details>

<a name="0x2_display_create_with_fields"></a>

## Function `create_with_fields`

Create a display seeded with <code>keys</code> / <code>values</code>. Both vectors must be the
same length; a mismatched pair would leave a half-written field.


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_create_with_fields">create_with_fields</a>&lt;T: key, W: drop&gt;(pub: &<a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;W&gt;, keys: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, values: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_create_with_fields">create_with_fields</a>&lt;T: key, W: drop&gt;(
    pub: &Publisher&lt;W&gt;,
    keys: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    values: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    ctx: &<b>mut</b> TxContext,
): <a href="display.md#0x2_display_Display">Display</a>&lt;T&gt; {
    <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&keys) == <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&values), <a href="display.md#0x2_display_E_FIELD_COUNT_MISMATCH">E_FIELD_COUNT_MISMATCH</a>);
    <a href="display.md#0x2_display_new_display">new_display</a>&lt;T, W&gt;(pub, keys, values, ctx)
}
</code></pre>



</details>

<a name="0x2_display_new_display"></a>

## Function `new_display`

Seed fields into a caller-built <code>VecMap</code>. Split out because this legacy
edition has no <code><b>let</b> <b>mut</b></code> bindings, so the loop body needs a named local.


<pre><code><b>fun</b> <a href="display.md#0x2_display_new_display">new_display</a>&lt;T: key, W: drop&gt;(_pub: &<a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;W&gt;, keys: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, values: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="display.md#0x2_display_new_display">new_display</a>&lt;T: key, W: drop&gt;(
    _pub: &Publisher&lt;W&gt;,
    keys: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    values: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    ctx: &<b>mut</b> TxContext,
): <a href="display.md#0x2_display_Display">Display</a>&lt;T&gt; {
    // One <a href="display.md#0x2_display_Display">Display</a> per type, enforced by the host and persisted across
    // transactions.
    <a href="types.md#0x2_types_claim_display">types::claim_display</a>&lt;T&gt;();
    <b>let</b> <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a> = utf8(<a href="display.md#0x2_display_type_name_bytes">type_name_bytes</a>&lt;T&gt;());
    <b>let</b> fields = <a href="display.md#0x2_display_seed_fields">seed_fields</a>(keys, values, <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>);
    <a href="display.md#0x2_display_Display">Display</a>&lt;T&gt; { id: <a href="object.md#0x2_object_new">object::new</a>(ctx), fields }
}
</code></pre>



</details>

<a name="0x2_display_seed_fields"></a>

## Function `seed_fields`

Build the initial field map: the type name first, then every seed pair.


<pre><code><b>fun</b> <a href="display.md#0x2_display_seed_fields">seed_fields</a>(keys: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, values: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>): <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="display.md#0x2_display_seed_fields">seed_fields</a>(
    keys: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    values: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>: String,
): VecMap&lt;String, String&gt; {
    <b>let</b> fields = <a href="vec_map.md#0x2_vec_map_empty">vec_map::empty</a>&lt;String, String&gt;();
    <a href="vec_map.md#0x2_vec_map_insert">vec_map::insert</a>(&<b>mut</b> fields, <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>, <a href="dependencies/move-stdlib/type_name.md#0x1_type_name">type_name</a>);
    <b>let</b> n = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&keys);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; n) {
        <b>let</b> k = *<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&keys, i);
        <b>let</b> v = *<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&values, i);
        <a href="vec_map.md#0x2_vec_map_insert">vec_map::insert</a>(&<b>mut</b> fields, utf8(k), utf8(v));
        i = i + 1;
    };
    fields
}
</code></pre>



</details>

<a name="0x2_display_add"></a>

## Function `add`

Add or replace one field.


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_add">add</a>&lt;T: key, W: drop&gt;(<a href="display.md#0x2_display">display</a>: &<b>mut</b> <a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;, _pub: &<a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;W&gt;, name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_add">add</a>&lt;T: key, W: drop&gt;(
    <a href="display.md#0x2_display">display</a>: &<b>mut</b> <a href="display.md#0x2_display_Display">Display</a>&lt;T&gt;,
    _pub: &Publisher&lt;W&gt;,
    name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    value: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    <a href="vec_map.md#0x2_vec_map_insert">vec_map::insert</a>(&<b>mut</b> <a href="display.md#0x2_display">display</a>.fields, utf8(name), utf8(value));
}
</code></pre>



</details>

<a name="0x2_display_remove"></a>

## Function `remove`

Remove one field. <code>T</code>'s own type-name entry is protected: indexers key
off it, and losing it would make the display unattributable.


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_remove">remove</a>&lt;T: key, W: drop&gt;(<a href="display.md#0x2_display">display</a>: &<b>mut</b> <a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;, _pub: &<a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;W&gt;, name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_remove">remove</a>&lt;T: key, W: drop&gt;(
    <a href="display.md#0x2_display">display</a>: &<b>mut</b> <a href="display.md#0x2_display_Display">Display</a>&lt;T&gt;,
    _pub: &Publisher&lt;W&gt;,
    name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    <b>let</b> key = utf8(name);
    <b>assert</b>!(key != utf8(<a href="display.md#0x2_display_type_name_bytes">type_name_bytes</a>&lt;T&gt;()), <a href="display.md#0x2_display_E_PROTECTED_FIELD">E_PROTECTED_FIELD</a>);
    <a href="vec_map.md#0x2_vec_map_remove">vec_map::remove</a>(&<b>mut</b> <a href="display.md#0x2_display">display</a>.fields, &key);
}
</code></pre>



</details>

<a name="0x2_display_get"></a>

## Function `get`

Read one field, or <code>none</code> when absent.


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_get">get</a>&lt;T: key&gt;(<a href="display.md#0x2_display">display</a>: &<a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;, name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_get">get</a>&lt;T: key&gt;(<a href="display.md#0x2_display">display</a>: &<a href="display.md#0x2_display_Display">Display</a>&lt;T&gt;, name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;String&gt; {
    <b>let</b> key = utf8(name);
    <b>if</b> (<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&<a href="display.md#0x2_display">display</a>.fields, &key)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(*<a href="vec_map.md#0x2_vec_map_get">vec_map::get</a>(&<a href="display.md#0x2_display">display</a>.fields, &key))
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>&lt;String&gt;()
    }
}
</code></pre>



</details>

<a name="0x2_display_get_with_default"></a>

## Function `get_with_default`

Read one field, falling back to <code>default</code> when absent. Convenient for
callers that always need a string (wallets rendering a list).


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_get_with_default">get_with_default</a>&lt;T: key&gt;(<a href="display.md#0x2_display">display</a>: &<a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;, name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, default: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_get_with_default">get_with_default</a>&lt;T: key&gt;(
    <a href="display.md#0x2_display">display</a>: &<a href="display.md#0x2_display_Display">Display</a>&lt;T&gt;,
    name: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    default: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): String {
    <b>let</b> key = utf8(name);
    <b>if</b> (<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&<a href="display.md#0x2_display">display</a>.fields, &key)) {
        *<a href="vec_map.md#0x2_vec_map_get">vec_map::get</a>(&<a href="display.md#0x2_display">display</a>.fields, &key)
    } <b>else</b> {
        utf8(default)
    }
}
</code></pre>



</details>

<a name="0x2_display_field_count"></a>

## Function `field_count`

Number of fields currently set.


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_field_count">field_count</a>&lt;T: key&gt;(<a href="display.md#0x2_display">display</a>: &<a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_field_count">field_count</a>&lt;T: key&gt;(<a href="display.md#0x2_display">display</a>: &<a href="display.md#0x2_display_Display">Display</a>&lt;T&gt;): u64 {
    <a href="vec_map.md#0x2_vec_map_size">vec_map::size</a>(&<a href="display.md#0x2_display">display</a>.fields)
}
</code></pre>



</details>

<a name="0x2_display_id"></a>

## Function `id`

The display's object id.


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_id">id</a>&lt;T: key&gt;(<a href="display.md#0x2_display">display</a>: &<a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;): <a href="object.md#0x2_object_ID">object::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_id">id</a>&lt;T: key&gt;(<a href="display.md#0x2_display">display</a>: &<a href="display.md#0x2_display_Display">Display</a>&lt;T&gt;): ID {
    <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&<a href="display.md#0x2_display">display</a>.id)
}
</code></pre>



</details>

<a name="0x2_display_display_address"></a>

## Function `display_address`

The display's object address.


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_display_address">display_address</a>&lt;T: key&gt;(<a href="display.md#0x2_display">display</a>: &<a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_display_address">display_address</a>&lt;T: key&gt;(<a href="display.md#0x2_display">display</a>: &<a href="display.md#0x2_display_Display">Display</a>&lt;T&gt;): <b>address</b> {
    <a href="object.md#0x2_object_uid_address">object::uid_address</a>(&<a href="display.md#0x2_display">display</a>.id)
}
</code></pre>



</details>

<a name="0x2_display_type_name_of"></a>

## Function `type_name_of`

<code>T</code>'s canonical type-name bytes.


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_type_name_of">type_name_of</a>&lt;T: key&gt;(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_type_name_of">type_name_of</a>&lt;T: key&gt;(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="display.md#0x2_display_type_name_bytes">type_name_bytes</a>&lt;T&gt;()
}
</code></pre>



</details>

<a name="0x2_display_share"></a>

## Function `share`

Share the display so anyone can read it. There is nothing sensitive in
the fields, and wallets need it shared to discover it.


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_share">share</a>&lt;T: key&gt;(<a href="display.md#0x2_display">display</a>: <a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_share">share</a>&lt;T: key&gt;(<a href="display.md#0x2_display">display</a>: <a href="display.md#0x2_display_Display">Display</a>&lt;T&gt;) {
    <a href="transfer.md#0x2_transfer_share_object">transfer::share_object</a>(<a href="display.md#0x2_display">display</a>);
}
</code></pre>



</details>

<a name="0x2_display_transfer"></a>

## Function `transfer`

Transfer the display to a specific address instead of sharing it.


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>&lt;T: key&gt;(<a href="display.md#0x2_display">display</a>: <a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;, recipient: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>&lt;T: key&gt;(<a href="display.md#0x2_display">display</a>: <a href="display.md#0x2_display_Display">Display</a>&lt;T&gt;, recipient: <b>address</b>) {
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(<a href="display.md#0x2_display">display</a>, recipient);
}
</code></pre>



</details>

<a name="0x2_display_destroy"></a>

## Function `destroy`

Destroy the display. <code>T</code>'s metadata becomes unresolvable off-chain; the
type itself and its objects are unaffected.

<code>VecMap</code> carries <code>drop</code>, so the field map goes away with the value; only
the <code>UID</code> needs explicit deletion.


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_destroy">destroy</a>&lt;T: key, W: drop&gt;(<a href="display.md#0x2_display">display</a>: <a href="display.md#0x2_display_Display">display::Display</a>&lt;T&gt;, _pub: &<a href="package.md#0x2_package_Publisher">package::Publisher</a>&lt;W&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="display.md#0x2_display_destroy">destroy</a>&lt;T: key, W: drop&gt;(<a href="display.md#0x2_display">display</a>: <a href="display.md#0x2_display_Display">Display</a>&lt;T&gt;, _pub: &Publisher&lt;W&gt;) {
    <b>let</b> <a href="display.md#0x2_display_Display">Display</a> { id, fields } = <a href="display.md#0x2_display">display</a>;
    <b>let</b> _ = fields;
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
}
</code></pre>



</details>

<a name="0x2_display_type_name_bytes"></a>

## Function `type_name_bytes`

Canonical type-name bytes for <code>T</code>.


<pre><code><b>fun</b> <a href="display.md#0x2_display_type_name_bytes">type_name_bytes</a>&lt;T: key&gt;(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="display.md#0x2_display_type_name_bytes">type_name_bytes</a>&lt;T: key&gt;(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
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
