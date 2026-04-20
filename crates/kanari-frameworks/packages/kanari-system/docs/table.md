
<a name="0x2_table"></a>

# Module `0x2::table`



-  [Resource `Table`](#0x2_table_Table)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x2_table_new)
-  [Function `add`](#0x2_table_add)
-  [Function `borrow_mut`](#0x2_table_borrow_mut)
-  [Function `borrow`](#0x2_table_borrow)
-  [Function `remove`](#0x2_table_remove)
-  [Function `contains`](#0x2_table_contains)
-  [Function `length`](#0x2_table_length)
-  [Function `destroy_empty`](#0x2_table_destroy_empty)


<pre><code><b>use</b> <a href="dynamic_field.md#0x2_dynamic_field">0x2::dynamic_field</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
</code></pre>



<a name="0x2_table_Table"></a>

## Resource `Table`

A Table map that stores key-value pairs dynamically


<pre><code><b>struct</b> <a href="table.md#0x2_table_Table">Table</a>&lt;K: <b>copy</b>, drop, store, V: store&gt; <b>has</b> store, key
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


<a name="0x2_table_ETableNotEmpty"></a>

Error codes


<pre><code><b>const</b> <a href="table.md#0x2_table_ETableNotEmpty">ETableNotEmpty</a>: u64 = 1;
</code></pre>



<a name="0x2_table_new"></a>

## Function `new`

Creates a new, empty table


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_new">new</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="table.md#0x2_table_Table">table::Table</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_new">new</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(ctx: &<b>mut</b> TxContext): <a href="table.md#0x2_table_Table">Table</a>&lt;K, V&gt; {
    <a href="table.md#0x2_table_Table">Table</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        size: 0,
    }
}
</code></pre>



</details>

<a name="0x2_table_add"></a>

## Function `add`

Adds a key-value pair to the table


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_add">add</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="table.md#0x2_table_Table">table::Table</a>&lt;K, V&gt;, k: K, v: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_add">add</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="table.md#0x2_table_Table">Table</a>&lt;K, V&gt;, k: K, v: V) {
    <a href="dynamic_field.md#0x2_dynamic_field_add">dynamic_field::add</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.id, k, v);
    <a href="table.md#0x2_table">table</a>.size = <a href="table.md#0x2_table">table</a>.size + 1;
}
</code></pre>



</details>

<a name="0x2_table_borrow_mut"></a>

## Function `borrow_mut`

Mutably borrows the value associated with the key


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="table.md#0x2_table_Table">table::Table</a>&lt;K, V&gt;, k: K): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="table.md#0x2_table_Table">Table</a>&lt;K, V&gt;, k: K): &<b>mut</b> V {
    <a href="dynamic_field.md#0x2_dynamic_field_borrow_mut">dynamic_field::borrow_mut</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.id, k)
}
</code></pre>



</details>

<a name="0x2_table_borrow"></a>

## Function `borrow`

Immutably borrows the value associated with the key


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_borrow">borrow</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="table.md#0x2_table_Table">table::Table</a>&lt;K, V&gt;, k: K): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_borrow">borrow</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="table.md#0x2_table_Table">Table</a>&lt;K, V&gt;, k: K): &V {
    <a href="dynamic_field.md#0x2_dynamic_field_borrow">dynamic_field::borrow</a>(&<a href="table.md#0x2_table">table</a>.id, k)
}
</code></pre>



</details>

<a name="0x2_table_remove"></a>

## Function `remove`

Removes the key-value pair and returns the value


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_remove">remove</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="table.md#0x2_table_Table">table::Table</a>&lt;K, V&gt;, k: K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_remove">remove</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="table.md#0x2_table_Table">Table</a>&lt;K, V&gt;, k: K): V {
    <b>let</b> v = <a href="dynamic_field.md#0x2_dynamic_field_remove">dynamic_field::remove</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.id, k);
    <a href="table.md#0x2_table">table</a>.size = <a href="table.md#0x2_table">table</a>.size - 1;
    v
}
</code></pre>



</details>

<a name="0x2_table_contains"></a>

## Function `contains`

Returns true if the table contains the key


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_contains">contains</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="table.md#0x2_table_Table">table::Table</a>&lt;K, V&gt;, k: K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_contains">contains</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="table.md#0x2_table_Table">Table</a>&lt;K, V&gt;, k: K): bool {
    <a href="dynamic_field.md#0x2_dynamic_field_exists_">dynamic_field::exists_</a>(&<a href="table.md#0x2_table">table</a>.id, k)
}
</code></pre>



</details>

<a name="0x2_table_length"></a>

## Function `length`

Returns the number of entries in the table


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_length">length</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="table.md#0x2_table_Table">table::Table</a>&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_length">length</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="table.md#0x2_table_Table">Table</a>&lt;K, V&gt;): u64 {
    <a href="table.md#0x2_table">table</a>.size
}
</code></pre>



</details>

<a name="0x2_table_destroy_empty"></a>

## Function `destroy_empty`

Destroys an empty table


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_destroy_empty">destroy_empty</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: <a href="table.md#0x2_table_Table">table::Table</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x2_table_destroy_empty">destroy_empty</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="table.md#0x2_table">table</a>: <a href="table.md#0x2_table_Table">Table</a>&lt;K, V&gt;) {
    <b>let</b> <a href="table.md#0x2_table_Table">Table</a> { id, size } = <a href="table.md#0x2_table">table</a>;
    <b>assert</b>!(size == 0, <a href="table.md#0x2_table_ETableNotEmpty">ETableNotEmpty</a>);
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
