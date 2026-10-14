
<a name="0x2_vec_map"></a>

# Module `0x2::vec_map`

A map backed by a vector, mirroring <code>sui::vec_map</code>.


-  [Struct `VecMap`](#0x2_vec_map_VecMap)
-  [Struct `Entry`](#0x2_vec_map_Entry)
-  [Constants](#@Constants_0)
-  [Function `empty`](#0x2_vec_map_empty)
-  [Function `insert`](#0x2_vec_map_insert)
-  [Function `remove`](#0x2_vec_map_remove)
-  [Function `pop`](#0x2_vec_map_pop)
-  [Function `get_mut`](#0x2_vec_map_get_mut)
-  [Function `get`](#0x2_vec_map_get)
-  [Function `try_get`](#0x2_vec_map_try_get)
-  [Function `contains`](#0x2_vec_map_contains)
-  [Function `size`](#0x2_vec_map_size)
-  [Function `is_empty`](#0x2_vec_map_is_empty)
-  [Function `destroy_empty`](#0x2_vec_map_destroy_empty)
-  [Function `into_keys_values`](#0x2_vec_map_into_keys_values)
-  [Function `from_keys_values`](#0x2_vec_map_from_keys_values)
-  [Function `keys`](#0x2_vec_map_keys)
-  [Function `get_idx_opt`](#0x2_vec_map_get_idx_opt)
-  [Function `get_idx`](#0x2_vec_map_get_idx)
-  [Function `get_entry_by_idx`](#0x2_vec_map_get_entry_by_idx)
-  [Function `get_entry_by_idx_mut`](#0x2_vec_map_get_entry_by_idx_mut)
-  [Function `remove_entry_by_idx`](#0x2_vec_map_remove_entry_by_idx)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="dependencies/move-stdlib/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x2_vec_map_VecMap"></a>

## Struct `VecMap`

A map data structure backed by a vector. Entries are kept in
insertion order (not sorted); all operations are O(N). For large
maps prefer <code><a href="table.md#0x2_table">table</a></code>/<code><a href="linked_table.md#0x2_linked_table">linked_table</a></code> instead.


<pre><code><b>struct</b> <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K: <b>copy</b>, V&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>contents: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="vec_map.md#0x2_vec_map_Entry">vec_map::Entry</a>&lt;K, V&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_vec_map_Entry"></a>

## Struct `Entry`

An entry in the map


<pre><code><b>struct</b> <a href="vec_map.md#0x2_vec_map_Entry">Entry</a>&lt;K: <b>copy</b>, V&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>key: K</code>
</dt>
<dd>

</dd>
<dt>
<code>value: V</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_vec_map_EKeyAlreadyExists"></a>

This key already exists in the map


<pre><code><b>const</b> <a href="vec_map.md#0x2_vec_map_EKeyAlreadyExists">EKeyAlreadyExists</a>: u64 = 0;
</code></pre>



<a name="0x2_vec_map_EKeyDoesNotExist"></a>

This key does not exist in the map


<pre><code><b>const</b> <a href="vec_map.md#0x2_vec_map_EKeyDoesNotExist">EKeyDoesNotExist</a>: u64 = 1;
</code></pre>



<a name="0x2_vec_map_EIndexOutOfBounds"></a>

Trying to access an element of the map at an invalid index


<pre><code><b>const</b> <a href="vec_map.md#0x2_vec_map_EIndexOutOfBounds">EIndexOutOfBounds</a>: u64 = 3;
</code></pre>



<a name="0x2_vec_map_EMapEmpty"></a>

Trying to pop from a map that is empty


<pre><code><b>const</b> <a href="vec_map.md#0x2_vec_map_EMapEmpty">EMapEmpty</a>: u64 = 4;
</code></pre>



<a name="0x2_vec_map_EMapNotEmpty"></a>

Trying to destroy a map that is not empty


<pre><code><b>const</b> <a href="vec_map.md#0x2_vec_map_EMapNotEmpty">EMapNotEmpty</a>: u64 = 2;
</code></pre>



<a name="0x2_vec_map_EUnequalLengths"></a>

Trying to construct a map from keys and values of different lengths


<pre><code><b>const</b> <a href="vec_map.md#0x2_vec_map_EUnequalLengths">EUnequalLengths</a>: u64 = 5;
</code></pre>



<a name="0x2_vec_map_empty"></a>

## Function `empty`

Create an empty <code><a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a></code>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_empty">empty</a>&lt;K: <b>copy</b>, V&gt;(): <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_empty">empty</a>&lt;K: <b>copy</b>, V&gt;(): <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt; {
    <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a> { contents: <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>() }
}
</code></pre>



</details>

<a name="0x2_vec_map_insert"></a>

## Function `insert`

Insert <code>key</code> |-> <code>value</code>. Aborts <code><a href="vec_map.md#0x2_vec_map_EKeyAlreadyExists">EKeyAlreadyExists</a></code> on duplicates.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_insert">insert</a>&lt;K: <b>copy</b>, V&gt;(self: &<b>mut</b> <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;, key: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_insert">insert</a>&lt;K: <b>copy</b>, V&gt;(self: &<b>mut</b> <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;, key: K, value: V) {
    <b>assert</b>!(!<a href="vec_map.md#0x2_vec_map_contains">contains</a>(self, &key), <a href="vec_map.md#0x2_vec_map_EKeyAlreadyExists">EKeyAlreadyExists</a>);
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> self.contents, <a href="vec_map.md#0x2_vec_map_Entry">Entry</a> { key, value });
}
</code></pre>



</details>

<a name="0x2_vec_map_remove"></a>

## Function `remove`

Remove <code>key</code> and return <code>(key, value)</code>. Aborts <code><a href="vec_map.md#0x2_vec_map_EKeyDoesNotExist">EKeyDoesNotExist</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_remove">remove</a>&lt;K: <b>copy</b>, V&gt;(self: &<b>mut</b> <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;, key: &K): (K, V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_remove">remove</a>&lt;K: <b>copy</b>, V&gt;(self: &<b>mut</b> <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;, key: &K): (K, V) {
    <b>let</b> idx = <a href="vec_map.md#0x2_vec_map_get_idx">get_idx</a>(self, key);
    <b>let</b> <a href="vec_map.md#0x2_vec_map_Entry">Entry</a> { key, value } = <a href="dependencies/move-stdlib/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> self.contents, idx);
    (key, value)
}
</code></pre>



</details>

<a name="0x2_vec_map_pop"></a>

## Function `pop`

Pop the most recently inserted entry. Aborts <code><a href="vec_map.md#0x2_vec_map_EMapEmpty">EMapEmpty</a></code> when empty.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_pop">pop</a>&lt;K: <b>copy</b>, V&gt;(self: &<b>mut</b> <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;): (K, V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_pop">pop</a>&lt;K: <b>copy</b>, V&gt;(self: &<b>mut</b> <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;): (K, V) {
    <b>assert</b>!(!<a href="dependencies/move-stdlib/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&self.contents), <a href="vec_map.md#0x2_vec_map_EMapEmpty">EMapEmpty</a>);
    <b>let</b> <a href="vec_map.md#0x2_vec_map_Entry">Entry</a> { key, value } = <a href="dependencies/move-stdlib/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> self.contents);
    (key, value)
}
</code></pre>



</details>

<a name="0x2_vec_map_get_mut"></a>

## Function `get_mut`

Mutably borrow the value for <code>key</code>. Aborts <code><a href="vec_map.md#0x2_vec_map_EKeyDoesNotExist">EKeyDoesNotExist</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_get_mut">get_mut</a>&lt;K: <b>copy</b>, V&gt;(self: &<b>mut</b> <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;, key: &K): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_get_mut">get_mut</a>&lt;K: <b>copy</b>, V&gt;(self: &<b>mut</b> <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;, key: &K): &<b>mut</b> V {
    <b>let</b> idx = <a href="vec_map.md#0x2_vec_map_get_idx">get_idx</a>(self, key);
    <b>let</b> entry = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> self.contents, idx);
    &<b>mut</b> entry.value
}
</code></pre>



</details>

<a name="0x2_vec_map_get"></a>

## Function `get`

Borrow the value for <code>key</code>. Aborts <code><a href="vec_map.md#0x2_vec_map_EKeyDoesNotExist">EKeyDoesNotExist</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_get">get</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;, key: &K): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_get">get</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;, key: &K): &V {
    <b>let</b> idx = <a href="vec_map.md#0x2_vec_map_get_idx">get_idx</a>(self, key);
    <b>let</b> entry = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&self.contents, idx);
    &entry.value
}
</code></pre>



</details>

<a name="0x2_vec_map_try_get"></a>

## Function `try_get`

Borrow a copyable value for <code>key</code>, or <code>None</code> when absent.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_try_get">try_get</a>&lt;K: <b>copy</b>, V: <b>copy</b>&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;, key: &K): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_try_get">try_get</a>&lt;K: <b>copy</b>, V: <b>copy</b>&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;, key: &K): Option&lt;V&gt; {
    <b>if</b> (<a href="vec_map.md#0x2_vec_map_contains">contains</a>(self, key)) {
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(*<a href="vec_map.md#0x2_vec_map_get">get</a>(self, key))
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a name="0x2_vec_map_contains"></a>

## Function `contains`

True iff an entry exists for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_contains">contains</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;, key: &K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_contains">contains</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;, key: &K): bool {
    <a href="dependencies/move-stdlib/option.md#0x1_option_is_some">option::is_some</a>(&<a href="vec_map.md#0x2_vec_map_get_idx_opt">get_idx_opt</a>(self, key))
}
</code></pre>



</details>

<a name="0x2_vec_map_size"></a>

## Function `size`

Number of entries.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_size">size</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_size">size</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;): u64 {
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&self.contents)
}
</code></pre>



</details>

<a name="0x2_vec_map_is_empty"></a>

## Function `is_empty`

True iff there are no entries.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_is_empty">is_empty</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_is_empty">is_empty</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;): bool {
    <a href="vec_map.md#0x2_vec_map_size">size</a>(self) == 0
}
</code></pre>



</details>

<a name="0x2_vec_map_destroy_empty"></a>

## Function `destroy_empty`

Destroy an empty map. Aborts <code><a href="vec_map.md#0x2_vec_map_EMapNotEmpty">EMapNotEmpty</a></code> otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_destroy_empty">destroy_empty</a>&lt;K: <b>copy</b>, V&gt;(self: <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_destroy_empty">destroy_empty</a>&lt;K: <b>copy</b>, V&gt;(self: <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;) {
    <b>let</b> <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a> { contents } = self;
    <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&contents), <a href="vec_map.md#0x2_vec_map_EMapNotEmpty">EMapNotEmpty</a>);
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(contents);
}
</code></pre>



</details>

<a name="0x2_vec_map_into_keys_values"></a>

## Function `into_keys_values`

Unpack into <code>(keys, values)</code> in insertion order.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_into_keys_values">into_keys_values</a>&lt;K: <b>copy</b>, V&gt;(self: <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;): (<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;K&gt;, <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_into_keys_values">into_keys_values</a>&lt;K: <b>copy</b>, V&gt;(self: <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;): (<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;K&gt;, <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;V&gt;) {
    <b>let</b> <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a> { contents } = self;
    <b>let</b> contents = contents;
    // Reverse so pops below yield insertion order.
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> contents);
    <b>let</b> i = 0;
    <b>let</b> n = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&contents);
    <b>let</b> keys = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> values = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>while</b> (i &lt; n) {
        <b>let</b> <a href="vec_map.md#0x2_vec_map_Entry">Entry</a> { key, value } = <a href="dependencies/move-stdlib/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> contents);
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> keys, key);
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> values, value);
        i = i + 1;
    };
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(contents);
    (keys, values)
}
</code></pre>



</details>

<a name="0x2_vec_map_from_keys_values"></a>

## Function `from_keys_values`

Build a map from parallel <code>keys</code>/<code>values</code> vectors.
Aborts <code><a href="vec_map.md#0x2_vec_map_EUnequalLengths">EUnequalLengths</a></code> when the lengths differ.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_from_keys_values">from_keys_values</a>&lt;K: <b>copy</b>, V&gt;(keys: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;V&gt;): <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_from_keys_values">from_keys_values</a>&lt;K: <b>copy</b>, V&gt;(
    keys: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;K&gt;,
    values: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;V&gt;,
): <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt; {
    <b>let</b> keys = keys;
    <b>let</b> values = values;
    <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&keys) == <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&values), <a href="vec_map.md#0x2_vec_map_EUnequalLengths">EUnequalLengths</a>);
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> keys);
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> values);
    <b>let</b> map = <a href="vec_map.md#0x2_vec_map_empty">empty</a>();
    <b>while</b> (!<a href="dependencies/move-stdlib/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&keys)) {
        <a href="vec_map.md#0x2_vec_map_insert">insert</a>(&<b>mut</b> map, <a href="dependencies/move-stdlib/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> keys), <a href="dependencies/move-stdlib/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> values));
    };
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(keys);
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(values);
    map
}
</code></pre>



</details>

<a name="0x2_vec_map_keys"></a>

## Function `keys`

All keys (insertion order; do not assume sorted).


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_keys">keys</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_keys">keys</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;K&gt; {
    <b>let</b> i = 0;
    <b>let</b> n = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&self.contents);
    <b>let</b> out = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>while</b> (i &lt; n) {
        <b>let</b> entry = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&self.contents, i);
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> out, entry.key);
        i = i + 1;
    };
    out
}
</code></pre>



</details>

<a name="0x2_vec_map_get_idx_opt"></a>

## Function `get_idx_opt`

Index of <code>key</code>, or <code>None</code> when absent (insertion order, not sorted).


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_get_idx_opt">get_idx_opt</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;, key: &K): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_get_idx_opt">get_idx_opt</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;, key: &K): Option&lt;u64&gt; {
    <b>let</b> i = 0;
    <b>let</b> n = <a href="vec_map.md#0x2_vec_map_size">size</a>(self);
    <b>while</b> (i &lt; n) {
        <b>if</b> (&<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&self.contents, i).key == key) {
            <b>return</b> <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(i)
        };
        i = i + 1;
    };
    <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
}
</code></pre>



</details>

<a name="0x2_vec_map_get_idx"></a>

## Function `get_idx`

Index of <code>key</code>. Aborts <code><a href="vec_map.md#0x2_vec_map_EKeyDoesNotExist">EKeyDoesNotExist</a></code> when absent.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_get_idx">get_idx</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;, key: &K): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_get_idx">get_idx</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;, key: &K): u64 {
    <b>let</b> idx_opt = <a href="vec_map.md#0x2_vec_map_get_idx_opt">get_idx_opt</a>(self, key);
    <b>assert</b>!(<a href="dependencies/move-stdlib/option.md#0x1_option_is_some">option::is_some</a>(&idx_opt), <a href="vec_map.md#0x2_vec_map_EKeyDoesNotExist">EKeyDoesNotExist</a>);
    <a href="dependencies/move-stdlib/option.md#0x1_option_destroy_some">option::destroy_some</a>(idx_opt)
}
</code></pre>



</details>

<a name="0x2_vec_map_get_entry_by_idx"></a>

## Function `get_entry_by_idx`

The <code>idx</code>th entry. Aborts <code><a href="vec_map.md#0x2_vec_map_EIndexOutOfBounds">EIndexOutOfBounds</a></code> when out of range.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_get_entry_by_idx">get_entry_by_idx</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;, idx: u64): (&K, &V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_get_entry_by_idx">get_entry_by_idx</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;, idx: u64): (&K, &V) {
    <b>assert</b>!(idx &lt; <a href="vec_map.md#0x2_vec_map_size">size</a>(self), <a href="vec_map.md#0x2_vec_map_EIndexOutOfBounds">EIndexOutOfBounds</a>);
    <b>let</b> entry = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&self.contents, idx);
    (&entry.key, &entry.value)
}
</code></pre>



</details>

<a name="0x2_vec_map_get_entry_by_idx_mut"></a>

## Function `get_entry_by_idx_mut`

Mutably borrow the <code>idx</code>th entry. Aborts <code><a href="vec_map.md#0x2_vec_map_EIndexOutOfBounds">EIndexOutOfBounds</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_get_entry_by_idx_mut">get_entry_by_idx_mut</a>&lt;K: <b>copy</b>, V&gt;(self: &<b>mut</b> <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;, idx: u64): (&K, &<b>mut</b> V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_get_entry_by_idx_mut">get_entry_by_idx_mut</a>&lt;K: <b>copy</b>, V&gt;(
    self: &<b>mut</b> <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;,
    idx: u64,
): (&K, &<b>mut</b> V) {
    <b>assert</b>!(idx &lt; <a href="vec_map.md#0x2_vec_map_size">size</a>(self), <a href="vec_map.md#0x2_vec_map_EIndexOutOfBounds">EIndexOutOfBounds</a>);
    <b>let</b> entry = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> self.contents, idx);
    (&entry.key, &<b>mut</b> entry.value)
}
</code></pre>



</details>

<a name="0x2_vec_map_remove_entry_by_idx"></a>

## Function `remove_entry_by_idx`

Remove the entry at <code>idx</code>. Aborts <code><a href="vec_map.md#0x2_vec_map_EIndexOutOfBounds">EIndexOutOfBounds</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_remove_entry_by_idx">remove_entry_by_idx</a>&lt;K: <b>copy</b>, V&gt;(self: &<b>mut</b> <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;K, V&gt;, idx: u64): (K, V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vec_map.md#0x2_vec_map_remove_entry_by_idx">remove_entry_by_idx</a>&lt;K: <b>copy</b>, V&gt;(self: &<b>mut</b> <a href="vec_map.md#0x2_vec_map_VecMap">VecMap</a>&lt;K, V&gt;, idx: u64): (K, V) {
    <b>assert</b>!(idx &lt; <a href="vec_map.md#0x2_vec_map_size">size</a>(self), <a href="vec_map.md#0x2_vec_map_EIndexOutOfBounds">EIndexOutOfBounds</a>);
    <b>let</b> <a href="vec_map.md#0x2_vec_map_Entry">Entry</a> { key, value } = <a href="dependencies/move-stdlib/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> self.contents, idx);
    (key, value)
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
