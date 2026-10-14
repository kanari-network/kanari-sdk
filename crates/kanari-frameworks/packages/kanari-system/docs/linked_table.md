
<a name="0x2_linked_table"></a>

# Module `0x2::linked_table`

Ordered table mirroring <code>sui::linked_table</code>: like <code><a href="table.md#0x2_table">table</a></code>, but entries
are linked for ordered insertion and removal.

NOTE: <code>contains</code> uses name-based existence (<code><a href="dynamic_field.md#0x2_dynamic_field_exists_">dynamic_field::exists_</a></code>)
because this runtime exposes no typed existence check; a field with the
same name but a different value type would report present and then abort
on <code><a href="borrow.md#0x2_borrow">borrow</a></code> with a type mismatch instead.


-  [Resource `LinkedTable`](#0x2_linked_table_LinkedTable)
-  [Struct `Node`](#0x2_linked_table_Node)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x2_linked_table_new)
-  [Function `front`](#0x2_linked_table_front)
-  [Function `back`](#0x2_linked_table_back)
-  [Function `push_front`](#0x2_linked_table_push_front)
-  [Function `push_back`](#0x2_linked_table_push_back)
-  [Function `borrow`](#0x2_linked_table_borrow)
-  [Function `borrow_mut`](#0x2_linked_table_borrow_mut)
-  [Function `prev`](#0x2_linked_table_prev)
-  [Function `next`](#0x2_linked_table_next)
-  [Function `remove`](#0x2_linked_table_remove)
-  [Function `pop_front`](#0x2_linked_table_pop_front)
-  [Function `pop_back`](#0x2_linked_table_pop_back)
-  [Function `contains`](#0x2_linked_table_contains)
-  [Function `length`](#0x2_linked_table_length)
-  [Function `is_empty`](#0x2_linked_table_is_empty)
-  [Function `destroy_empty`](#0x2_linked_table_destroy_empty)
-  [Function `drop`](#0x2_linked_table_drop)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="dynamic_field.md#0x2_dynamic_field">0x2::dynamic_field</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
</code></pre>



<a name="0x2_linked_table_LinkedTable"></a>

## Resource `LinkedTable`



<pre><code><b>struct</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K: <b>copy</b>, drop, store, V: store&gt; <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="object.md#0x2_object_UID">object::UID</a></code>
</dt>
<dd>
 the ID of this table
</dd>
<dt>
<code>size: u64</code>
</dt>
<dd>
 the number of key-value pairs in the table
</dd>
<dt>
<code>head: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;</code>
</dt>
<dd>
 the front of the table, i.e. the key of the first entry
</dd>
<dt>
<code>tail: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;</code>
</dt>
<dd>
 the back of the table, i.e. the key of the last entry
</dd>
</dl>


</details>

<a name="0x2_linked_table_Node"></a>

## Struct `Node`



<pre><code><b>struct</b> <a href="linked_table.md#0x2_linked_table_Node">Node</a>&lt;K: <b>copy</b>, drop, store, V: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>prev: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;</code>
</dt>
<dd>
 the previous key
</dd>
<dt>
<code>next: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;</code>
</dt>
<dd>
 the next key
</dd>
<dt>
<code>value: V</code>
</dt>
<dd>
 the value being stored
</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_linked_table_ETableNotEmpty"></a>



<pre><code><b>const</b> <a href="linked_table.md#0x2_linked_table_ETableNotEmpty">ETableNotEmpty</a>: u64 = 0;
</code></pre>



<a name="0x2_linked_table_ETableIsEmpty"></a>



<pre><code><b>const</b> <a href="linked_table.md#0x2_linked_table_ETableIsEmpty">ETableIsEmpty</a>: u64 = 1;
</code></pre>



<a name="0x2_linked_table_new"></a>

## Function `new`

Creates a new, empty table


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_new">new</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_new">new</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(ctx: &<b>mut</b> TxContext): <a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt; {
    <a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        size: 0,
        head: <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>(),
        tail: <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>(),
    }
}
</code></pre>



</details>

<a name="0x2_linked_table_front"></a>

## Function `front`

The key of the first element, or <code>None</code> when empty.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_front">front</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;): &<a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_front">front</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;): &Option&lt;K&gt; {
    &<a href="table.md#0x2_table">table</a>.head
}
</code></pre>



</details>

<a name="0x2_linked_table_back"></a>

## Function `back`

The key of the last element, or <code>None</code> when empty.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_back">back</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;): &<a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_back">back</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;): &Option&lt;K&gt; {
    &<a href="table.md#0x2_table">table</a>.tail
}
</code></pre>



</details>

<a name="0x2_linked_table_push_front"></a>

## Function `push_front`

Insert at the front. Aborts when <code>k</code> already exists.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_push_front">push_front</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;, k: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_push_front">push_front</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(
    <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;,
    k: K,
    value: V,
) {
    <b>let</b> old_head = <a href="dependencies/move-stdlib/option.md#0x1_option_swap_or_fill">option::swap_or_fill</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.head, k);
    <b>if</b> (<a href="dependencies/move-stdlib/option.md#0x1_option_is_none">option::is_none</a>(&<a href="table.md#0x2_table">table</a>.tail)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_fill">option::fill</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.tail, k);
    };
    <b>let</b> prev = <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>();
    <b>let</b> next = <b>if</b> (<a href="dependencies/move-stdlib/option.md#0x1_option_is_some">option::is_some</a>(&old_head)) {
        <b>let</b> old_head_k = <a href="dependencies/move-stdlib/option.md#0x1_option_destroy_some">option::destroy_some</a>(old_head);
        <a href="dynamic_field.md#0x2_dynamic_field_borrow_mut">dynamic_field::borrow_mut</a>&lt;K, <a href="linked_table.md#0x2_linked_table_Node">Node</a>&lt;K, V&gt;&gt;(&<b>mut</b> <a href="table.md#0x2_table">table</a>.id, old_head_k).prev =
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(k);
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(old_head_k)
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    };
    <a href="dynamic_field.md#0x2_dynamic_field_add">dynamic_field::add</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.id, k, <a href="linked_table.md#0x2_linked_table_Node">Node</a> { prev, next, value });
    <a href="table.md#0x2_table">table</a>.size = <a href="table.md#0x2_table">table</a>.size + 1;
}
</code></pre>



</details>

<a name="0x2_linked_table_push_back"></a>

## Function `push_back`

Insert at the back. Aborts when <code>k</code> already exists.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_push_back">push_back</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;, k: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_push_back">push_back</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(
    <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;,
    k: K,
    value: V,
) {
    <b>if</b> (<a href="dependencies/move-stdlib/option.md#0x1_option_is_none">option::is_none</a>(&<a href="table.md#0x2_table">table</a>.head)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_fill">option::fill</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.head, k);
    };
    <b>let</b> old_tail = <a href="dependencies/move-stdlib/option.md#0x1_option_swap_or_fill">option::swap_or_fill</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.tail, k);
    <b>let</b> prev = <b>if</b> (<a href="dependencies/move-stdlib/option.md#0x1_option_is_some">option::is_some</a>(&old_tail)) {
        <b>let</b> old_tail_k = <a href="dependencies/move-stdlib/option.md#0x1_option_destroy_some">option::destroy_some</a>(old_tail);
        <a href="dynamic_field.md#0x2_dynamic_field_borrow_mut">dynamic_field::borrow_mut</a>&lt;K, <a href="linked_table.md#0x2_linked_table_Node">Node</a>&lt;K, V&gt;&gt;(&<b>mut</b> <a href="table.md#0x2_table">table</a>.id, old_tail_k).next =
            <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(k);
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(old_tail_k)
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    };
    <b>let</b> next = <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>();
    <a href="dynamic_field.md#0x2_dynamic_field_add">dynamic_field::add</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.id, k, <a href="linked_table.md#0x2_linked_table_Node">Node</a> { prev, next, value });
    <a href="table.md#0x2_table">table</a>.size = <a href="table.md#0x2_table">table</a>.size + 1;
}
</code></pre>



</details>

<a name="0x2_linked_table_borrow"></a>

## Function `borrow`

Immutably borrow the value for <code>k</code>. Aborts when absent.


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow">borrow</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;, k: K): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow">borrow</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(
    <a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;,
    k: K,
): &V {
    &<a href="dynamic_field.md#0x2_dynamic_field_borrow">dynamic_field::borrow</a>&lt;K, <a href="linked_table.md#0x2_linked_table_Node">Node</a>&lt;K, V&gt;&gt;(&<a href="table.md#0x2_table">table</a>.id, k).value
}
</code></pre>



</details>

<a name="0x2_linked_table_borrow_mut"></a>

## Function `borrow_mut`

Mutably borrow the value for <code>k</code>. Aborts when absent.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;, k: K): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(
    <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;,
    k: K,
): &<b>mut</b> V {
    &<b>mut</b> <a href="dynamic_field.md#0x2_dynamic_field_borrow_mut">dynamic_field::borrow_mut</a>&lt;K, <a href="linked_table.md#0x2_linked_table_Node">Node</a>&lt;K, V&gt;&gt;(&<b>mut</b> <a href="table.md#0x2_table">table</a>.id, k).value
}
</code></pre>



</details>

<a name="0x2_linked_table_prev"></a>

## Function `prev`

The key before <code>k</code>, or <code>None</code> for the front element.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_prev">prev</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;, k: K): &<a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_prev">prev</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(
    <a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;,
    k: K,
): &Option&lt;K&gt; {
    &<a href="dynamic_field.md#0x2_dynamic_field_borrow">dynamic_field::borrow</a>&lt;K, <a href="linked_table.md#0x2_linked_table_Node">Node</a>&lt;K, V&gt;&gt;(&<a href="table.md#0x2_table">table</a>.id, k).prev
}
</code></pre>



</details>

<a name="0x2_linked_table_next"></a>

## Function `next`

The key after <code>k</code>, or <code>None</code> for the back element.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_next">next</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;, k: K): &<a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_next">next</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(
    <a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;,
    k: K,
): &Option&lt;K&gt; {
    &<a href="dynamic_field.md#0x2_dynamic_field_borrow">dynamic_field::borrow</a>&lt;K, <a href="linked_table.md#0x2_linked_table_Node">Node</a>&lt;K, V&gt;&gt;(&<a href="table.md#0x2_table">table</a>.id, k).next
}
</code></pre>



</details>

<a name="0x2_linked_table_remove"></a>

## Function `remove`

Remove <code>k</code> (splicing the ordering) and return its value.
Aborts when absent.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_remove">remove</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;, k: K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_remove">remove</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(
    <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;,
    k: K,
): V {
    <b>let</b> <a href="linked_table.md#0x2_linked_table_Node">Node</a> { prev, next, value } = <a href="dynamic_field.md#0x2_dynamic_field_remove">dynamic_field::remove</a>(&<b>mut</b> <a href="table.md#0x2_table">table</a>.id, k);
    <a href="table.md#0x2_table">table</a>.size = <a href="table.md#0x2_table">table</a>.size - 1;
    <b>if</b> (<a href="dependencies/move-stdlib/option.md#0x1_option_is_some">option::is_some</a>(&prev)) {
        <a href="dynamic_field.md#0x2_dynamic_field_borrow_mut">dynamic_field::borrow_mut</a>&lt;K, <a href="linked_table.md#0x2_linked_table_Node">Node</a>&lt;K, V&gt;&gt;(
            &<b>mut</b> <a href="table.md#0x2_table">table</a>.id,
            *<a href="dependencies/move-stdlib/option.md#0x1_option_borrow">option::borrow</a>(&prev),
        ).next = next;
    };
    <b>if</b> (<a href="dependencies/move-stdlib/option.md#0x1_option_is_some">option::is_some</a>(&next)) {
        <a href="dynamic_field.md#0x2_dynamic_field_borrow_mut">dynamic_field::borrow_mut</a>&lt;K, <a href="linked_table.md#0x2_linked_table_Node">Node</a>&lt;K, V&gt;&gt;(
            &<b>mut</b> <a href="table.md#0x2_table">table</a>.id,
            *<a href="dependencies/move-stdlib/option.md#0x1_option_borrow">option::borrow</a>(&next),
        ).prev = prev;
    };
    <b>if</b> (<a href="dependencies/move-stdlib/option.md#0x1_option_borrow">option::borrow</a>(&<a href="table.md#0x2_table">table</a>.head) == &k) {
        <a href="table.md#0x2_table">table</a>.head = next;
    };
    <b>if</b> (<a href="dependencies/move-stdlib/option.md#0x1_option_borrow">option::borrow</a>(&<a href="table.md#0x2_table">table</a>.tail) == &k) {
        <a href="table.md#0x2_table">table</a>.tail = prev;
    };
    value
}
</code></pre>



</details>

<a name="0x2_linked_table_pop_front"></a>

## Function `pop_front`

Remove the front entry. Aborts <code><a href="linked_table.md#0x2_linked_table_ETableIsEmpty">ETableIsEmpty</a></code> when empty.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_pop_front">pop_front</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;): (K, V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_pop_front">pop_front</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(
    <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;,
): (K, V) {
    <b>assert</b>!(<a href="dependencies/move-stdlib/option.md#0x1_option_is_some">option::is_some</a>(&<a href="table.md#0x2_table">table</a>.head), <a href="linked_table.md#0x2_linked_table_ETableIsEmpty">ETableIsEmpty</a>);
    <b>let</b> head = *<a href="dependencies/move-stdlib/option.md#0x1_option_borrow">option::borrow</a>(&<a href="table.md#0x2_table">table</a>.head);
    (head, <a href="linked_table.md#0x2_linked_table_remove">remove</a>(<a href="table.md#0x2_table">table</a>, head))
}
</code></pre>



</details>

<a name="0x2_linked_table_pop_back"></a>

## Function `pop_back`

Remove the back entry. Aborts <code><a href="linked_table.md#0x2_linked_table_ETableIsEmpty">ETableIsEmpty</a></code> when empty.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_pop_back">pop_back</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;): (K, V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_pop_back">pop_back</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(
    <a href="table.md#0x2_table">table</a>: &<b>mut</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;,
): (K, V) {
    <b>assert</b>!(<a href="dependencies/move-stdlib/option.md#0x1_option_is_some">option::is_some</a>(&<a href="table.md#0x2_table">table</a>.tail), <a href="linked_table.md#0x2_linked_table_ETableIsEmpty">ETableIsEmpty</a>);
    <b>let</b> tail = *<a href="dependencies/move-stdlib/option.md#0x1_option_borrow">option::borrow</a>(&<a href="table.md#0x2_table">table</a>.tail);
    (tail, <a href="linked_table.md#0x2_linked_table_remove">remove</a>(<a href="table.md#0x2_table">table</a>, tail))
}
</code></pre>



</details>

<a name="0x2_linked_table_contains"></a>

## Function `contains`

True iff a value is associated with <code>k</code>. See the module note about
name-based (not typed) existence.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_contains">contains</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;, k: K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_contains">contains</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(
    <a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;,
    k: K,
): bool {
    <a href="dynamic_field.md#0x2_dynamic_field_exists_">dynamic_field::exists_</a>(&<a href="table.md#0x2_table">table</a>.id, k)
}
</code></pre>



</details>

<a name="0x2_linked_table_length"></a>

## Function `length`

Number of entries.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_length">length</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_length">length</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;): u64 {
    <a href="table.md#0x2_table">table</a>.size
}
</code></pre>



</details>

<a name="0x2_linked_table_is_empty"></a>

## Function `is_empty`

True iff there are no entries.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_is_empty">is_empty</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_is_empty">is_empty</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="table.md#0x2_table">table</a>: &<a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;): bool {
    <a href="table.md#0x2_table">table</a>.size == 0
}
</code></pre>



</details>

<a name="0x2_linked_table_destroy_empty"></a>

## Function `destroy_empty`

Destroy an empty table. Aborts <code><a href="linked_table.md#0x2_linked_table_ETableNotEmpty">ETableNotEmpty</a></code> otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_destroy_empty">destroy_empty</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x2_table">table</a>: <a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_destroy_empty">destroy_empty</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(<a href="table.md#0x2_table">table</a>: <a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;) {
    <b>let</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a> { id, size, head: _, tail: _ } = <a href="table.md#0x2_table">table</a>;
    <b>assert</b>!(size == 0, <a href="linked_table.md#0x2_linked_table_ETableNotEmpty">ETableNotEmpty</a>);
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
}
</code></pre>



</details>

<a name="0x2_linked_table_drop"></a>

## Function `drop`

Drop a possibly non-empty table. Usable only when <code>V</code> has <code>drop</code>.


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_drop">drop</a>&lt;K: <b>copy</b>, drop, store, V: drop, store&gt;(<a href="table.md#0x2_table">table</a>: <a href="linked_table.md#0x2_linked_table_LinkedTable">linked_table::LinkedTable</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="linked_table.md#0x2_linked_table_drop">drop</a>&lt;K: <b>copy</b> + drop + store, V: drop + store&gt;(<a href="table.md#0x2_table">table</a>: <a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a>&lt;K, V&gt;) {
    <b>let</b> <a href="linked_table.md#0x2_linked_table_LinkedTable">LinkedTable</a> { id, size: _, head: _, tail: _ } = <a href="table.md#0x2_table">table</a>;
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
