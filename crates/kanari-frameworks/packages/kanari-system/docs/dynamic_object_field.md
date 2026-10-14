
<a name="0x2_dynamic_object_field"></a>

# Module `0x2::dynamic_object_field`

Similar to <code>kanari_system::dynamic_field</code>, but the values bound to these
dynamic fields _must_ be objects themselves (<code>key + store</code>, enforced by
the type bounds below).

Implemented purely on top of <code><a href="dynamic_field.md#0x2_dynamic_field">dynamic_field</a></code> (no natives of its own):
the field name is wrapped to prevent key collisions with direct
<code><a href="dynamic_field.md#0x2_dynamic_field">dynamic_field</a></code> usage on the same object.


-  [Struct `Wrapper`](#0x2_dynamic_object_field_Wrapper)
-  [Constants](#@Constants_0)
-  [Function `record_child`](#0x2_dynamic_object_field_record_child)
-  [Function `unrecord_child`](#0x2_dynamic_object_field_unrecord_child)
-  [Function `list_children`](#0x2_dynamic_object_field_list_children)
-  [Function `field_id`](#0x2_dynamic_object_field_field_id)
-  [Function `add`](#0x2_dynamic_object_field_add)
-  [Function `borrow`](#0x2_dynamic_object_field_borrow)
-  [Function `borrow_mut`](#0x2_dynamic_object_field_borrow_mut)
-  [Function `remove`](#0x2_dynamic_object_field_remove)
-  [Function `exists_`](#0x2_dynamic_object_field_exists_)
-  [Function `exists_with_type`](#0x2_dynamic_object_field_exists_with_type)
-  [Function `id`](#0x2_dynamic_object_field_id)
-  [Function `children`](#0x2_dynamic_object_field_children)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="dynamic_field.md#0x2_dynamic_field">0x2::dynamic_field</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
</code></pre>



<a name="0x2_dynamic_object_field_Wrapper"></a>

## Struct `Wrapper`

Internal wrapper for the field name. The separate type is necessary
to prevent key collisions with direct usage of <code><a href="dynamic_field.md#0x2_dynamic_field">dynamic_field</a></code>.


<pre><code><b>struct</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_Wrapper">Wrapper</a>&lt;Name&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: Name</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_dynamic_object_field_EInvalidChildId"></a>

The bound value is shorter than a UID: corrupt state, or a non-object
field probed via hand-crafted bytecode (<code>field_id</code> is only declared
in this module, but shares the <code><a href="dynamic_field.md#0x2_dynamic_field">dynamic_field</a></code> native table).
Raised by the native; never referenced in Move, like the codes below.


<pre><code><b>const</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_EInvalidChildId">EInvalidChildId</a>: u64 = 3;
</code></pre>



<a name="0x2_dynamic_object_field_record_child"></a>

## Function `record_child`

Record a <code>(parent, name) -&gt; child</code> link in the host child index.
The link is metadata only; the value itself still flows through the
regular dynamic-field path.


<pre><code><b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_record_child">record_child</a>&lt;Name: <b>copy</b>, drop, store, Value: store, key&gt;(<a href="object.md#0x2_object">object</a>: &<b>mut</b> <a href="object.md#0x2_object_UID">object::UID</a>, name: Name, child: &Value)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_record_child">record_child</a>&lt;Name: <b>copy</b> + drop + store, Value: key + store&gt;(
    <a href="object.md#0x2_object">object</a>: &<b>mut</b> UID,
    name: Name,
    child: &Value,
);
</code></pre>



</details>

<a name="0x2_dynamic_object_field_unrecord_child"></a>

## Function `unrecord_child`

Drop the <code>(parent, name)</code> link from the host child index.


<pre><code><b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_unrecord_child">unrecord_child</a>&lt;Name: <b>copy</b>, drop, store&gt;(<a href="object.md#0x2_object">object</a>: &<b>mut</b> <a href="object.md#0x2_object_UID">object::UID</a>, name: Name)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_unrecord_child">unrecord_child</a>&lt;Name: <b>copy</b> + drop + store&gt;(
    <a href="object.md#0x2_object">object</a>: &<b>mut</b> UID,
    name: Name,
);
</code></pre>



</details>

<a name="0x2_dynamic_object_field_list_children"></a>

## Function `list_children`

Enumerate the live children of <code><a href="object.md#0x2_object">object</a></code> in insertion order, merging
this transaction's pending writes over durable state.


<pre><code><b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_list_children">list_children</a>(<a href="object.md#0x2_object">object</a>: &<a href="object.md#0x2_object_UID">object::UID</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_list_children">list_children</a>(<a href="object.md#0x2_object">object</a>: &UID): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;;
</code></pre>



</details>

<a name="0x2_dynamic_object_field_field_id"></a>

## Function `field_id`

Address of the child object bound under <code>name</code>. Aborts with the
dynamic-field "does not exist" code if the field is missing, and with
<code><a href="dynamic_object_field.md#0x2_dynamic_object_field_EInvalidChildId">EInvalidChildId</a></code> if the stored value cannot hold an object ID.
Safe by construction for values written via <code>add</code>: <code>key</code> types are
UID-first (publish-time object rule), so their BCS opens with the
32-byte UID.


<pre><code><b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_field_id">field_id</a>&lt;Name: <b>copy</b>, drop, store&gt;(<a href="object.md#0x2_object">object</a>: &<a href="object.md#0x2_object_UID">object::UID</a>, name: Name): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_field_id">field_id</a>&lt;Name: <b>copy</b> + drop + store&gt;(
    <a href="object.md#0x2_object">object</a>: &UID,
    name: Name,
): <b>address</b>;
</code></pre>



</details>

<a name="0x2_dynamic_object_field_add"></a>

## Function `add`

Adds a dynamic object field to <code><a href="object.md#0x2_object">object</a></code> under <code>name</code>, binding the
object <code>value</code>. Aborts if the field already exists.


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_add">add</a>&lt;Name: <b>copy</b>, drop, store, Value: store, key&gt;(<a href="object.md#0x2_object">object</a>: &<b>mut</b> <a href="object.md#0x2_object_UID">object::UID</a>, name: Name, value: Value)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_add">add</a>&lt;Name: <b>copy</b> + drop + store, Value: key + store&gt;(
    <a href="object.md#0x2_object">object</a>: &<b>mut</b> UID,
    name: Name,
    value: Value,
) {
    <a href="dynamic_object_field.md#0x2_dynamic_object_field_record_child">record_child</a>(<a href="object.md#0x2_object">object</a>, name, &value);
    <a href="dynamic_field.md#0x2_dynamic_field_add">dynamic_field::add</a>(<a href="object.md#0x2_object">object</a>, <a href="dynamic_object_field.md#0x2_dynamic_object_field_Wrapper">Wrapper</a> { name }, value);
}
</code></pre>



</details>

<a name="0x2_dynamic_object_field_borrow"></a>

## Function `borrow`

Immutably borrows the dynamic object field <code>name</code> on <code><a href="object.md#0x2_object">object</a></code>.
Aborts if the field does not exist or has a different value type.


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow">borrow</a>&lt;Name: <b>copy</b>, drop, store, Value: store, key&gt;(<a href="object.md#0x2_object">object</a>: &<a href="object.md#0x2_object_UID">object::UID</a>, name: Name): &Value
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow">borrow</a>&lt;Name: <b>copy</b> + drop + store, Value: key + store&gt;(
    <a href="object.md#0x2_object">object</a>: &UID,
    name: Name,
): &Value {
    <a href="dynamic_field.md#0x2_dynamic_field_borrow">dynamic_field::borrow</a>(<a href="object.md#0x2_object">object</a>, <a href="dynamic_object_field.md#0x2_dynamic_object_field_Wrapper">Wrapper</a> { name })
}
</code></pre>



</details>

<a name="0x2_dynamic_object_field_borrow_mut"></a>

## Function `borrow_mut`

Mutably borrows the dynamic object field <code>name</code> on <code><a href="object.md#0x2_object">object</a></code>.
Aborts if the field does not exist or has a different value type.


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_borrow_mut">borrow_mut</a>&lt;Name: <b>copy</b>, drop, store, Value: store, key&gt;(<a href="object.md#0x2_object">object</a>: &<b>mut</b> <a href="object.md#0x2_object_UID">object::UID</a>, name: Name): &<b>mut</b> Value
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_borrow_mut">borrow_mut</a>&lt;Name: <b>copy</b> + drop + store, Value: key + store&gt;(
    <a href="object.md#0x2_object">object</a>: &<b>mut</b> UID,
    name: Name,
): &<b>mut</b> Value {
    <a href="dynamic_field.md#0x2_dynamic_field_borrow_mut">dynamic_field::borrow_mut</a>(<a href="object.md#0x2_object">object</a>, <a href="dynamic_object_field.md#0x2_dynamic_object_field_Wrapper">Wrapper</a> { name })
}
</code></pre>



</details>

<a name="0x2_dynamic_object_field_remove"></a>

## Function `remove`

Removes the dynamic object field <code>name</code> on <code><a href="object.md#0x2_object">object</a></code> and returns the
bound object. Aborts if the field does not exist.


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_remove">remove</a>&lt;Name: <b>copy</b>, drop, store, Value: store, key&gt;(<a href="object.md#0x2_object">object</a>: &<b>mut</b> <a href="object.md#0x2_object_UID">object::UID</a>, name: Name): Value
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_remove">remove</a>&lt;Name: <b>copy</b> + drop + store, Value: key + store&gt;(
    <a href="object.md#0x2_object">object</a>: &<b>mut</b> UID,
    name: Name,
): Value {
    <a href="dynamic_object_field.md#0x2_dynamic_object_field_unrecord_child">unrecord_child</a>(<a href="object.md#0x2_object">object</a>, name);
    <a href="dynamic_field.md#0x2_dynamic_field_remove">dynamic_field::remove</a>(<a href="object.md#0x2_object">object</a>, <a href="dynamic_object_field.md#0x2_dynamic_object_field_Wrapper">Wrapper</a> { name })
}
</code></pre>



</details>

<a name="0x2_dynamic_object_field_exists_"></a>

## Function `exists_`

Returns true iff <code><a href="object.md#0x2_object">object</a></code> has a dynamic object field <code>name</code>.


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_exists_">exists_</a>&lt;Name: <b>copy</b>, drop, store&gt;(<a href="object.md#0x2_object">object</a>: &<a href="object.md#0x2_object_UID">object::UID</a>, name: Name): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_exists_">exists_</a>&lt;Name: <b>copy</b> + drop + store&gt;(
    <a href="object.md#0x2_object">object</a>: &UID,
    name: Name,
): bool {
    <a href="dynamic_field.md#0x2_dynamic_field_exists_">dynamic_field::exists_</a>(<a href="object.md#0x2_object">object</a>, <a href="dynamic_object_field.md#0x2_dynamic_object_field_Wrapper">Wrapper</a> { name })
}
</code></pre>



</details>

<a name="0x2_dynamic_object_field_exists_with_type"></a>

## Function `exists_with_type`

Returns true iff <code><a href="object.md#0x2_object">object</a></code> has a dynamic object field <code>name</code> whose
bound value type is exactly <code>Value</code>.


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_exists_with_type">exists_with_type</a>&lt;Name: <b>copy</b>, drop, store, Value: store, key&gt;(<a href="object.md#0x2_object">object</a>: &<a href="object.md#0x2_object_UID">object::UID</a>, name: Name): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_exists_with_type">exists_with_type</a>&lt;Name: <b>copy</b> + drop + store, Value: key + store&gt;(
    <a href="object.md#0x2_object">object</a>: &UID,
    name: Name,
): bool {
    <a href="dynamic_field.md#0x2_dynamic_field_exists_with_type">dynamic_field::exists_with_type</a>&lt;<a href="dynamic_object_field.md#0x2_dynamic_object_field_Wrapper">Wrapper</a>&lt;Name&gt;, Value&gt;(<a href="object.md#0x2_object">object</a>, <a href="dynamic_object_field.md#0x2_dynamic_object_field_Wrapper">Wrapper</a> { name })
}
</code></pre>



</details>

<a name="0x2_dynamic_object_field_id"></a>

## Function `id`

Returns the ID of the child object bound under <code>name</code>, or <code>none</code> if
the field does not exist.


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_id">id</a>&lt;Name: <b>copy</b>, drop, store&gt;(<a href="object.md#0x2_object">object</a>: &<a href="object.md#0x2_object_UID">object::UID</a>, name: Name): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="object.md#0x2_object_ID">object::ID</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_id">id</a>&lt;Name: <b>copy</b> + drop + store&gt;(
    <a href="object.md#0x2_object">object</a>: &UID,
    name: Name,
): Option&lt;<a href="object.md#0x2_object_ID">object::ID</a>&gt; {
    <b>if</b> (!<a href="dynamic_object_field.md#0x2_dynamic_object_field_exists_">exists_</a>(<a href="object.md#0x2_object">object</a>, name)) <b>return</b> <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>();
    <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(<a href="object.md#0x2_object_id_from_address">object::id_from_address</a>(<a href="dynamic_object_field.md#0x2_dynamic_object_field_field_id">field_id</a>(<a href="object.md#0x2_object">object</a>, name)))
}
</code></pre>



</details>

<a name="0x2_dynamic_object_field_children"></a>

## Function `children`

Live child object addresses of <code><a href="object.md#0x2_object">object</a></code> in insertion order.


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_children">children</a>(<a href="object.md#0x2_object">object</a>: &<a href="object.md#0x2_object_UID">object::UID</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dynamic_object_field.md#0x2_dynamic_object_field_children">children</a>(<a href="object.md#0x2_object">object</a>: &UID): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; {
    <a href="dynamic_object_field.md#0x2_dynamic_object_field_list_children">list_children</a>(<a href="object.md#0x2_object">object</a>)
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
