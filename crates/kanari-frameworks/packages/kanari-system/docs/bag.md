
<a name="0x2_bag"></a>

# Module `0x2::bag`



-  [Resource `Bag`](#0x2_bag_Bag)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x2_bag_new)
-  [Function `add`](#0x2_bag_add)
-  [Function `borrow_mut`](#0x2_bag_borrow_mut)
-  [Function `borrow`](#0x2_bag_borrow)
-  [Function `remove`](#0x2_bag_remove)
-  [Function `contains`](#0x2_bag_contains)
-  [Function `length`](#0x2_bag_length)
-  [Function `destroy_empty`](#0x2_bag_destroy_empty)


<pre><code><b>use</b> <a href="dynamic_field.md#0x2_dynamic_field">0x2::dynamic_field</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
</code></pre>



<a name="0x2_bag_Bag"></a>

## Resource `Bag`

A Bag that stores heterogeneous key-value pairs dynamically


<pre><code><b>struct</b> <a href="bag.md#0x2_bag_Bag">Bag</a> <b>has</b> store, key
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
<code>size: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_bag_EBagNotEmpty"></a>

Error codes


<pre><code><b>const</b> <a href="bag.md#0x2_bag_EBagNotEmpty">EBagNotEmpty</a>: u64 = 1;
</code></pre>



<a name="0x2_bag_new"></a>

## Function `new`

Creates a new, empty bag


<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_new">new</a>(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="bag.md#0x2_bag_Bag">bag::Bag</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_new">new</a>(ctx: &<b>mut</b> TxContext): <a href="bag.md#0x2_bag_Bag">Bag</a> {
    <a href="bag.md#0x2_bag_Bag">Bag</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        size: 0,
    }
}
</code></pre>



</details>

<a name="0x2_bag_add"></a>

## Function `add`

Adds a key-value pair to the bag.
Types K and V can be different for every entry.


<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_add">add</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="bag.md#0x2_bag">bag</a>: &<b>mut</b> <a href="bag.md#0x2_bag_Bag">bag::Bag</a>, k: K, v: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_add">add</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="bag.md#0x2_bag">bag</a>: &<b>mut</b> <a href="bag.md#0x2_bag_Bag">Bag</a>, k: K, v: V) {
    <a href="dynamic_field.md#0x2_dynamic_field_add">dynamic_field::add</a>(&<b>mut</b> <a href="bag.md#0x2_bag">bag</a>.id, k, v);
    <a href="bag.md#0x2_bag">bag</a>.size = <a href="bag.md#0x2_bag">bag</a>.size + 1;
}
</code></pre>



</details>

<a name="0x2_bag_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="bag.md#0x2_bag">bag</a>: &<b>mut</b> <a href="bag.md#0x2_bag_Bag">bag::Bag</a>, k: K): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="bag.md#0x2_bag">bag</a>: &<b>mut</b> <a href="bag.md#0x2_bag_Bag">Bag</a>, k: K): &<b>mut</b> V {
    <a href="dynamic_field.md#0x2_dynamic_field_borrow_mut">dynamic_field::borrow_mut</a>(&<b>mut</b> <a href="bag.md#0x2_bag">bag</a>.id, k)
}
</code></pre>



</details>

<a name="0x2_bag_borrow"></a>

## Function `borrow`



<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_borrow">borrow</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="bag.md#0x2_bag">bag</a>: &<a href="bag.md#0x2_bag_Bag">bag::Bag</a>, k: K): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_borrow">borrow</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="bag.md#0x2_bag">bag</a>: &<a href="bag.md#0x2_bag_Bag">Bag</a>, k: K): &V {
    <a href="dynamic_field.md#0x2_dynamic_field_borrow">dynamic_field::borrow</a>(&<a href="bag.md#0x2_bag">bag</a>.id, k)
}
</code></pre>



</details>

<a name="0x2_bag_remove"></a>

## Function `remove`



<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_remove">remove</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="bag.md#0x2_bag">bag</a>: &<b>mut</b> <a href="bag.md#0x2_bag_Bag">bag::Bag</a>, k: K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_remove">remove</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="bag.md#0x2_bag">bag</a>: &<b>mut</b> <a href="bag.md#0x2_bag_Bag">Bag</a>, k: K): V {
    <b>let</b> v = <a href="dynamic_field.md#0x2_dynamic_field_remove">dynamic_field::remove</a>(&<b>mut</b> <a href="bag.md#0x2_bag">bag</a>.id, k);
    <a href="bag.md#0x2_bag">bag</a>.size = <a href="bag.md#0x2_bag">bag</a>.size - 1;
    v
}
</code></pre>



</details>

<a name="0x2_bag_contains"></a>

## Function `contains`



<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_contains">contains</a>&lt;K: <b>copy</b>, drop, store&gt;(<a href="bag.md#0x2_bag">bag</a>: &<a href="bag.md#0x2_bag_Bag">bag::Bag</a>, k: K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_contains">contains</a>&lt;K: <b>copy</b> + drop + store&gt;(<a href="bag.md#0x2_bag">bag</a>: &<a href="bag.md#0x2_bag_Bag">Bag</a>, k: K): bool {
    <a href="dynamic_field.md#0x2_dynamic_field_exists_">dynamic_field::exists_</a>(&<a href="bag.md#0x2_bag">bag</a>.id, k)
}
</code></pre>



</details>

<a name="0x2_bag_length"></a>

## Function `length`



<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_length">length</a>(<a href="bag.md#0x2_bag">bag</a>: &<a href="bag.md#0x2_bag_Bag">bag::Bag</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_length">length</a>(<a href="bag.md#0x2_bag">bag</a>: &<a href="bag.md#0x2_bag_Bag">Bag</a>): u64 {
    <a href="bag.md#0x2_bag">bag</a>.size
}
</code></pre>



</details>

<a name="0x2_bag_destroy_empty"></a>

## Function `destroy_empty`



<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_destroy_empty">destroy_empty</a>(<a href="bag.md#0x2_bag">bag</a>: <a href="bag.md#0x2_bag_Bag">bag::Bag</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bag.md#0x2_bag_destroy_empty">destroy_empty</a>(<a href="bag.md#0x2_bag">bag</a>: <a href="bag.md#0x2_bag_Bag">Bag</a>) {
    <b>let</b> <a href="bag.md#0x2_bag_Bag">Bag</a> { id, size } = <a href="bag.md#0x2_bag">bag</a>;
    <b>assert</b>!(size == 0, <a href="bag.md#0x2_bag_EBagNotEmpty">EBagNotEmpty</a>);
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
