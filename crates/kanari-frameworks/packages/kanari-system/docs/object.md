
<a name="0x2_object"></a>

# Module `0x2::object`



-  [Struct `UID`](#0x2_object_UID)
-  [Struct `ID`](#0x2_object_ID)
-  [Function `new`](#0x2_object_new)
-  [Function `uid_to_inner`](#0x2_object_uid_to_inner)
-  [Function `id_from_address`](#0x2_object_id_from_address)
-  [Function `id_to_address`](#0x2_object_id_to_address)
-  [Function `id_to_bytes`](#0x2_object_id_to_bytes)
-  [Function `uid_address`](#0x2_object_uid_address)
-  [Function `uid_to_u64`](#0x2_object_uid_to_u64)
-  [Function `uid_to_bytes`](#0x2_object_uid_to_bytes)
-  [Function `id_bytes`](#0x2_object_id_bytes)
-  [Function `save_object`](#0x2_object_save_object)
-  [Function `delete`](#0x2_object_delete)
-  [Function `delete_impl`](#0x2_object_delete_impl)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
</code></pre>



<a name="0x2_object_UID"></a>

## Struct `UID`

Simple UID wrapper used for resource IDs in this package.
The UID contains an object-style address generated from the
transaction context, ensuring it is unique per creation.


<pre><code><b>struct</b> <a href="object.md#0x2_object_UID">UID</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addr: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_object_ID"></a>

## Struct `ID`

ID is a copyable, storable identifier for an object.
It is used to reference objects without requiring ownership of the UID.


<pre><code><b>struct</b> <a href="object.md#0x2_object_ID">ID</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bytes: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_object_new"></a>

## Function `new`

Create a new UID by deriving a fresh object address from the
transaction context. This ensures the address is unique and based on the
current transaction input (e.g., transaction hash, counter).
Used by resources that need a guaranteed unique ID when they are created.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_new">new</a>(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="object.md#0x2_object_UID">object::UID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_new">new</a>(ctx: &<b>mut</b> TxContext): <a href="object.md#0x2_object_UID">UID</a> {
    <a href="object.md#0x2_object_UID">UID</a> { addr: <a href="tx_context.md#0x2_tx_context_fresh_object_address">tx_context::fresh_object_address</a>(ctx) }
}
</code></pre>



</details>

<a name="0x2_object_uid_to_inner"></a>

## Function `uid_to_inner`

Extract an <code><a href="object.md#0x2_object_ID">ID</a></code> from a <code><a href="object.md#0x2_object_UID">UID</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_uid_to_inner">uid_to_inner</a>(uid: &<a href="object.md#0x2_object_UID">object::UID</a>): <a href="object.md#0x2_object_ID">object::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_uid_to_inner">uid_to_inner</a>(uid: &<a href="object.md#0x2_object_UID">UID</a>): <a href="object.md#0x2_object_ID">ID</a> {
    <a href="object.md#0x2_object_ID">ID</a> { bytes: uid.addr }
}
</code></pre>



</details>

<a name="0x2_object_id_from_address"></a>

## Function `id_from_address`

Create an <code><a href="object.md#0x2_object_ID">ID</a></code> directly from an address.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_id_from_address">id_from_address</a>(bytes: <b>address</b>): <a href="object.md#0x2_object_ID">object::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_id_from_address">id_from_address</a>(bytes: <b>address</b>): <a href="object.md#0x2_object_ID">ID</a> {
    <a href="object.md#0x2_object_ID">ID</a> { bytes }
}
</code></pre>



</details>

<a name="0x2_object_id_to_address"></a>

## Function `id_to_address`

Get the underlying address of an <code><a href="object.md#0x2_object_ID">ID</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_id_to_address">id_to_address</a>(id: &<a href="object.md#0x2_object_ID">object::ID</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_id_to_address">id_to_address</a>(id: &<a href="object.md#0x2_object_ID">ID</a>): <b>address</b> {
    id.bytes
}
</code></pre>



</details>

<a name="0x2_object_id_to_bytes"></a>

## Function `id_to_bytes`

Get the address of an <code><a href="object.md#0x2_object_ID">ID</a></code> as a byte vector.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_id_to_bytes">id_to_bytes</a>(id: &<a href="object.md#0x2_object_ID">object::ID</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_id_to_bytes">id_to_bytes</a>(id: &<a href="object.md#0x2_object_ID">ID</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="dependencies/move-stdlib/signer.md#0x1_signer_address_to_bytes">signer::address_to_bytes</a>(id.bytes)
}
</code></pre>



</details>

<a name="0x2_object_uid_address"></a>

## Function `uid_address`

Return the underlying address for a UID.
This is the canonical representation of the object's ID.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_uid_address">uid_address</a>(u: &<a href="object.md#0x2_object_UID">object::UID</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_uid_address">uid_address</a>(u: &<a href="object.md#0x2_object_UID">UID</a>): <b>address</b> {
    u.addr
}
</code></pre>



</details>

<a name="0x2_object_uid_to_u64"></a>

## Function `uid_to_u64`

Return the object's address as a <code>u64</code> value.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_uid_to_u64">uid_to_u64</a>(u: &<a href="object.md#0x2_object_UID">object::UID</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_uid_to_u64">uid_to_u64</a>(u: &<a href="object.md#0x2_object_UID">UID</a>): u64 {
    <a href="dependencies/move-stdlib/signer.md#0x1_signer_address_to_u64">signer::address_to_u64</a>(u.addr)
}
</code></pre>



</details>

<a name="0x2_object_uid_to_bytes"></a>

## Function `uid_to_bytes`

Return the object's address as a <code><a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_uid_to_bytes">uid_to_bytes</a>(u: &<a href="object.md#0x2_object_UID">object::UID</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_uid_to_bytes">uid_to_bytes</a>(u: &<a href="object.md#0x2_object_UID">UID</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="dependencies/move-stdlib/signer.md#0x1_signer_address_to_bytes">signer::address_to_bytes</a>(u.addr)
}
</code></pre>



</details>

<a name="0x2_object_id_bytes"></a>

## Function `id_bytes`

Return the object's address as a <code><a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>.
This is useful for serialization, hashing, and interoperability across modules.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_id_bytes">id_bytes</a>(u: &<a href="object.md#0x2_object_UID">object::UID</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_id_bytes">id_bytes</a>(u: &<a href="object.md#0x2_object_UID">UID</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="dependencies/move-stdlib/signer.md#0x1_signer_address_to_bytes">signer::address_to_bytes</a>(u.addr)
}
</code></pre>



</details>

<a name="0x2_object_save_object"></a>

## Function `save_object`



<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_save_object">save_object</a>&lt;T: key&gt;(obj: &T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="object.md#0x2_object_save_object">save_object</a>&lt;T: key&gt;(obj: &T);
</code></pre>



</details>

<a name="0x2_object_delete"></a>

## Function `delete`

Delete an object by consuming its UID.
This removes the object from storage and potentially triggers a storage rebate.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_delete">delete</a>(id: <a href="object.md#0x2_object_UID">object::UID</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x2_object_delete">delete</a>(id: <a href="object.md#0x2_object_UID">UID</a>) {
    <a href="object.md#0x2_object_delete_impl">delete_impl</a>(id);
}
</code></pre>



</details>

<a name="0x2_object_delete_impl"></a>

## Function `delete_impl`



<pre><code><b>fun</b> <a href="object.md#0x2_object_delete_impl">delete_impl</a>(id: <a href="object.md#0x2_object_UID">object::UID</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="object.md#0x2_object_delete_impl">delete_impl</a>(id: <a href="object.md#0x2_object_UID">UID</a>);
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
