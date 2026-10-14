
<a name="0x2_vec_set"></a>

# Module `0x2::vec_set`

A set backed by a vector: O(n) membership, insertion-ordered iteration.
Mirrors <code>sui::vec_set</code>; used for the per-type address sets in
<code>kanari_system::deny_list</code> so that module stays free of raw vectors.


-  [Struct `VecSet`](#0x2_vec_set_VecSet)
-  [Constants](#@Constants_0)
-  [Function `empty`](#0x2_vec_set_empty)
-  [Function `singleton`](#0x2_vec_set_singleton)
-  [Function `insert`](#0x2_vec_set_insert)
-  [Function `remove`](#0x2_vec_set_remove)
-  [Function `contains`](#0x2_vec_set_contains)
-  [Function `size`](#0x2_vec_set_size)
-  [Function `is_empty`](#0x2_vec_set_is_empty)
-  [Function `keys`](#0x2_vec_set_keys)
-  [Function `into_keys`](#0x2_vec_set_into_keys)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x2_vec_set_VecSet"></a>

## Struct `VecSet`



<pre><code><b>struct</b> <a href="vec_set.md#0x2_vec_set_VecSet">VecSet</a>&lt;K: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>contents: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;K&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_vec_set_EKeyAlreadyExists"></a>

Key is already present.


<pre><code><b>const</b> <a href="vec_set.md#0x2_vec_set_EKeyAlreadyExists">EKeyAlreadyExists</a>: u64 = 0;
</code></pre>



<a name="0x2_vec_set_EKeyDoesNotExist"></a>

Key is absent.


<pre><code><b>const</b> <a href="vec_set.md#0x2_vec_set_EKeyDoesNotExist">EKeyDoesNotExist</a>: u64 = 1;
</code></pre>



<a name="0x2_vec_set_empty"></a>

## Function `empty`

Create an empty set.


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_empty">empty</a>&lt;K: <b>copy</b>, drop, store&gt;(): <a href="vec_set.md#0x2_vec_set_VecSet">vec_set::VecSet</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_empty">empty</a>&lt;K: <b>copy</b> + drop + store&gt;(): <a href="vec_set.md#0x2_vec_set_VecSet">VecSet</a>&lt;K&gt; {
    <a href="vec_set.md#0x2_vec_set_VecSet">VecSet</a> { contents: <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>() }
}
</code></pre>



</details>

<a name="0x2_vec_set_singleton"></a>

## Function `singleton`

Create a single-element set.


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_singleton">singleton</a>&lt;K: <b>copy</b>, drop, store&gt;(key: K): <a href="vec_set.md#0x2_vec_set_VecSet">vec_set::VecSet</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_singleton">singleton</a>&lt;K: <b>copy</b> + drop + store&gt;(key: K): <a href="vec_set.md#0x2_vec_set_VecSet">VecSet</a>&lt;K&gt; {
    <a href="vec_set.md#0x2_vec_set_VecSet">VecSet</a> { contents: <a href="dependencies/move-stdlib/vector.md#0x1_vector_singleton">vector::singleton</a>(key) }
}
</code></pre>



</details>

<a name="0x2_vec_set_insert"></a>

## Function `insert`

Insert <code>key</code>. Aborts <code><a href="vec_set.md#0x2_vec_set_EKeyAlreadyExists">EKeyAlreadyExists</a></code> when already present.


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_insert">insert</a>&lt;K: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="vec_set.md#0x2_vec_set_VecSet">vec_set::VecSet</a>&lt;K&gt;, key: K)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_insert">insert</a>&lt;K: <b>copy</b> + drop + store&gt;(self: &<b>mut</b> <a href="vec_set.md#0x2_vec_set_VecSet">VecSet</a>&lt;K&gt;, key: K) {
    <b>assert</b>!(!<a href="dependencies/move-stdlib/vector.md#0x1_vector_contains">vector::contains</a>(&self.contents, &key), <a href="vec_set.md#0x2_vec_set_EKeyAlreadyExists">EKeyAlreadyExists</a>);
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> self.contents, key);
}
</code></pre>



</details>

<a name="0x2_vec_set_remove"></a>

## Function `remove`

Remove <code>key</code> and return nothing. Aborts <code><a href="vec_set.md#0x2_vec_set_EKeyDoesNotExist">EKeyDoesNotExist</a></code> when absent.


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_remove">remove</a>&lt;K: <b>copy</b>, drop, store&gt;(self: &<b>mut</b> <a href="vec_set.md#0x2_vec_set_VecSet">vec_set::VecSet</a>&lt;K&gt;, key: &K)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_remove">remove</a>&lt;K: <b>copy</b> + drop + store&gt;(self: &<b>mut</b> <a href="vec_set.md#0x2_vec_set_VecSet">VecSet</a>&lt;K&gt;, key: &K) {
    <b>let</b> (found, index) = <a href="dependencies/move-stdlib/vector.md#0x1_vector_index_of">vector::index_of</a>(&self.contents, key);
    <b>assert</b>!(found, <a href="vec_set.md#0x2_vec_set_EKeyDoesNotExist">EKeyDoesNotExist</a>);
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> self.contents, index);
}
</code></pre>



</details>

<a name="0x2_vec_set_contains"></a>

## Function `contains`

True iff <code>key</code> is in the set.


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_contains">contains</a>&lt;K: <b>copy</b>, drop, store&gt;(self: &<a href="vec_set.md#0x2_vec_set_VecSet">vec_set::VecSet</a>&lt;K&gt;, key: &K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_contains">contains</a>&lt;K: <b>copy</b> + drop + store&gt;(self: &<a href="vec_set.md#0x2_vec_set_VecSet">VecSet</a>&lt;K&gt;, key: &K): bool {
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_contains">vector::contains</a>(&self.contents, key)
}
</code></pre>



</details>

<a name="0x2_vec_set_size"></a>

## Function `size`

Number of elements.


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_size">size</a>&lt;K: <b>copy</b>, drop, store&gt;(self: &<a href="vec_set.md#0x2_vec_set_VecSet">vec_set::VecSet</a>&lt;K&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_size">size</a>&lt;K: <b>copy</b> + drop + store&gt;(self: &<a href="vec_set.md#0x2_vec_set_VecSet">VecSet</a>&lt;K&gt;): u64 {
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&self.contents)
}
</code></pre>



</details>

<a name="0x2_vec_set_is_empty"></a>

## Function `is_empty`

True iff there are no elements.


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_is_empty">is_empty</a>&lt;K: <b>copy</b>, drop, store&gt;(self: &<a href="vec_set.md#0x2_vec_set_VecSet">vec_set::VecSet</a>&lt;K&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_is_empty">is_empty</a>&lt;K: <b>copy</b> + drop + store&gt;(self: &<a href="vec_set.md#0x2_vec_set_VecSet">VecSet</a>&lt;K&gt;): bool {
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&self.contents)
}
</code></pre>



</details>

<a name="0x2_vec_set_keys"></a>

## Function `keys`

Copy of the elements in insertion order.


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_keys">keys</a>&lt;K: <b>copy</b>, drop, store&gt;(self: &<a href="vec_set.md#0x2_vec_set_VecSet">vec_set::VecSet</a>&lt;K&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_keys">keys</a>&lt;K: <b>copy</b> + drop + store&gt;(self: &<a href="vec_set.md#0x2_vec_set_VecSet">VecSet</a>&lt;K&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;K&gt; {
    self.contents
}
</code></pre>



</details>

<a name="0x2_vec_set_into_keys"></a>

## Function `into_keys`

Destructure into the elements in insertion order.


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_into_keys">into_keys</a>&lt;K: <b>copy</b>, drop, store&gt;(self: <a href="vec_set.md#0x2_vec_set_VecSet">vec_set::VecSet</a>&lt;K&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_set.md#0x2_vec_set_into_keys">into_keys</a>&lt;K: <b>copy</b> + drop + store&gt;(self: <a href="vec_set.md#0x2_vec_set_VecSet">VecSet</a>&lt;K&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;K&gt; {
    <b>let</b> <a href="vec_set.md#0x2_vec_set_VecSet">VecSet</a> { contents } = self;
    contents
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
