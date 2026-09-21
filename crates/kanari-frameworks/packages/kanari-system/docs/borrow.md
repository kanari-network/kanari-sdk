
<a name="0x2_borrow"></a>

# Module `0x2::borrow`

Safe borrowing helpers over <code><a href="object.md#0x2_object_borrow_global">object::borrow_global</a>[_mut]</code>.

Raw natives return a reference and leave persistence to the caller:
forgetting <code>save_object</code> after a <code>borrow_global_mut</code> silently drops the
mutation on commit (this exact bug once lost multisig approvals).


-  [Constants](#@Constants_0)
-  [Function `borrow`](#0x2_borrow_borrow)
-  [Function `borrow_mut`](#0x2_borrow_borrow_mut)
-  [Function `save`](#0x2_borrow_save)
-  [Function `assert_distinct`](#0x2_borrow_assert_distinct)
-  [Function `borrow_id`](#0x2_borrow_borrow_id)


<pre><code><b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x2_borrow_E_SAME_OBJECT"></a>

Alias check for the two-object helper.


<pre><code><b>const</b> <a href="borrow.md#0x2_borrow_E_SAME_OBJECT">E_SAME_OBJECT</a>: u64 = 1;
</code></pre>



<a name="0x2_borrow_borrow"></a>

## Function `borrow`

Read an object without mutation authority.


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow">borrow</a>&lt;T: key&gt;(addr: <b>address</b>): &T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow">borrow</a>&lt;T: key&gt;(addr: <b>address</b>): &T {
    <a href="object.md#0x2_object_borrow_global">object::borrow_global</a>&lt;T&gt;(addr)
}
</code></pre>



</details>

<a name="0x2_borrow_borrow_mut"></a>

## Function `borrow_mut`

Borrow mutably. The caller MUST call <code>save</code> (same module) after
mutating, otherwise the change is lost on commit.


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow_borrow_mut">borrow_mut</a>&lt;T: key&gt;(addr: <b>address</b>): &<b>mut</b> T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow_borrow_mut">borrow_mut</a>&lt;T: key&gt;(addr: <b>address</b>): &<b>mut</b> T {
    <a href="object.md#0x2_object_borrow_global_mut">object::borrow_global_mut</a>&lt;T&gt;(addr)
}
</code></pre>



</details>

<a name="0x2_borrow_save"></a>

## Function `save`

Persist a borrowed object. Call exactly once after mutating a
<code>borrow_mut</code> reference.


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow_save">save</a>&lt;T: key&gt;(obj: &T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow_save">save</a>&lt;T: key&gt;(obj: &T) {
    <a href="object.md#0x2_object_save_object">object::save_object</a>(obj);
}
</code></pre>



</details>

<a name="0x2_borrow_assert_distinct"></a>

## Function `assert_distinct`

Assert two object IDs are distinct before borrowing both mutably.
Move cannot hold two <code>&<b>mut</b></code> to the same object.


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow_assert_distinct">assert_distinct</a>(a: <b>address</b>, b: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow_assert_distinct">assert_distinct</a>(a: <b>address</b>, b: <b>address</b>) {
    <b>assert</b>!(a != b, <a href="borrow.md#0x2_borrow_E_SAME_OBJECT">E_SAME_OBJECT</a>);
}
</code></pre>



</details>

<a name="0x2_borrow_borrow_id"></a>

## Function `borrow_id`

Resolve an <code>ID</code> to its address, then borrow immutably.


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow_borrow_id">borrow_id</a>&lt;T: key&gt;(id: &<a href="object.md#0x2_object_ID">object::ID</a>): &T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="borrow.md#0x2_borrow_borrow_id">borrow_id</a>&lt;T: key&gt;(id: &ID): &T {
    <a href="borrow.md#0x2_borrow">borrow</a>&lt;T&gt;(<a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(id))
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
