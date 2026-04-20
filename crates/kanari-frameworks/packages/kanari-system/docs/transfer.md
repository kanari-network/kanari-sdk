
<a name="0x2_transfer"></a>

# Module `0x2::transfer`

Production-Ready Transfer Module
Uses proper address types with validation


-  [Struct `Transfer`](#0x2_transfer_Transfer)
-  [Resource `ObjectStore`](#0x2_transfer_ObjectStore)
-  [Constants](#@Constants_0)
-  [Function `create_transfer`](#0x2_transfer_create_transfer)
-  [Function `get_amount`](#0x2_transfer_get_amount)
-  [Function `get_from`](#0x2_transfer_get_from)
-  [Function `get_to`](#0x2_transfer_get_to)
-  [Function `total_amount`](#0x2_transfer_total_amount)
-  [Function `is_valid_amount`](#0x2_transfer_is_valid_amount)
-  [Function `freeze_object`](#0x2_transfer_freeze_object)
-  [Function `public_freeze_object`](#0x2_transfer_public_freeze_object)
-  [Function `public_transfer`](#0x2_transfer_public_transfer)
-  [Function `transfer_with_uid`](#0x2_transfer_transfer_with_uid)
-  [Function `share_object`](#0x2_transfer_share_object)


<pre><code><b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
</code></pre>



<a name="0x2_transfer_Transfer"></a>

## Struct `Transfer`

Transfer record


<pre><code><b>struct</b> <a href="transfer.md#0x2_transfer_Transfer">Transfer</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>from: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code><b>to</b>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_transfer_ObjectStore"></a>

## Resource `ObjectStore`



<pre><code><b>struct</b> <a href="transfer.md#0x2_transfer_ObjectStore">ObjectStore</a>&lt;T: store, key&gt; <b>has</b> key
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
<code>inner: T</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_transfer_ERR_INVALID_AMOUNT"></a>

Error codes


<pre><code><b>const</b> <a href="transfer.md#0x2_transfer_ERR_INVALID_AMOUNT">ERR_INVALID_AMOUNT</a>: u64 = 1;
</code></pre>



<a name="0x2_transfer_ERR_SAME_ADDRESS"></a>



<pre><code><b>const</b> <a href="transfer.md#0x2_transfer_ERR_SAME_ADDRESS">ERR_SAME_ADDRESS</a>: u64 = 2;
</code></pre>



<a name="0x2_transfer_create_transfer"></a>

## Function `create_transfer`

Create a transfer record with full validation
Validates: amount > 0 AND from != to


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_create_transfer">create_transfer</a>(from: <b>address</b>, <b>to</b>: <b>address</b>, amount: u64): <a href="transfer.md#0x2_transfer_Transfer">transfer::Transfer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_create_transfer">create_transfer</a>(from: <b>address</b>, <b>to</b>: <b>address</b>, amount: u64): <a href="transfer.md#0x2_transfer_Transfer">Transfer</a> {
    <b>assert</b>!(amount &gt; 0, <a href="transfer.md#0x2_transfer_ERR_INVALID_AMOUNT">ERR_INVALID_AMOUNT</a>);
    <b>assert</b>!(from != <b>to</b>, <a href="transfer.md#0x2_transfer_ERR_SAME_ADDRESS">ERR_SAME_ADDRESS</a>);
    <a href="transfer.md#0x2_transfer_Transfer">Transfer</a> { from, <b>to</b>, amount }
}
</code></pre>



</details>

<a name="0x2_transfer_get_amount"></a>

## Function `get_amount`

Get transfer details


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_get_amount">get_amount</a>(<a href="transfer.md#0x2_transfer">transfer</a>: &<a href="transfer.md#0x2_transfer_Transfer">transfer::Transfer</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_get_amount">get_amount</a>(<a href="transfer.md#0x2_transfer">transfer</a>: &<a href="transfer.md#0x2_transfer_Transfer">Transfer</a>): u64 {
    <a href="transfer.md#0x2_transfer">transfer</a>.amount
}
</code></pre>



</details>

<a name="0x2_transfer_get_from"></a>

## Function `get_from`



<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_get_from">get_from</a>(<a href="transfer.md#0x2_transfer">transfer</a>: &<a href="transfer.md#0x2_transfer_Transfer">transfer::Transfer</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_get_from">get_from</a>(<a href="transfer.md#0x2_transfer">transfer</a>: &<a href="transfer.md#0x2_transfer_Transfer">Transfer</a>): <b>address</b> {
    <a href="transfer.md#0x2_transfer">transfer</a>.from
}
</code></pre>



</details>

<a name="0x2_transfer_get_to"></a>

## Function `get_to`



<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_get_to">get_to</a>(<a href="transfer.md#0x2_transfer">transfer</a>: &<a href="transfer.md#0x2_transfer_Transfer">transfer::Transfer</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_get_to">get_to</a>(<a href="transfer.md#0x2_transfer">transfer</a>: &<a href="transfer.md#0x2_transfer_Transfer">Transfer</a>): <b>address</b> {
    <a href="transfer.md#0x2_transfer">transfer</a>.<b>to</b>
}
</code></pre>



</details>

<a name="0x2_transfer_total_amount"></a>

## Function `total_amount`

Calculate total from multiple transfers


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_total_amount">total_amount</a>(transfers: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="transfer.md#0x2_transfer_Transfer">transfer::Transfer</a>&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_total_amount">total_amount</a>(transfers: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="transfer.md#0x2_transfer_Transfer">Transfer</a>&gt;): u64 {
    <b>let</b> total = 0u64;
    <b>let</b> len = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(transfers);
    <b>let</b> i = 0u64;

    <b>while</b> (i &lt; len) {
        <b>let</b> <a href="transfer.md#0x2_transfer">transfer</a> = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(transfers, i);
        total = total + <a href="transfer.md#0x2_transfer">transfer</a>.amount;
        i = i + 1;
    };

    total
}
</code></pre>



</details>

<a name="0x2_transfer_is_valid_amount"></a>

## Function `is_valid_amount`

Check if amount is valid (non-zero)


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_is_valid_amount">is_valid_amount</a>(amount: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_is_valid_amount">is_valid_amount</a>(amount: u64): bool {
    amount &gt; 0
}
</code></pre>



</details>

<a name="0x2_transfer_freeze_object"></a>

## Function `freeze_object`

Freeze an object (make it immutable).
The object must have <code>key</code> and <code>store</code>.


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_freeze_object">freeze_object</a>&lt;T: store, key&gt;(obj: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="transfer.md#0x2_transfer_freeze_object">freeze_object</a>&lt;T: key + store&gt;(obj: T);
</code></pre>



</details>

<a name="0x2_transfer_public_freeze_object"></a>

## Function `public_freeze_object`

Helper to 'freeze' a metadata object returned by currency creation.
Redirects to <code>freeze_object</code> native.


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_public_freeze_object">public_freeze_object</a>&lt;T: store, key&gt;(obj: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_public_freeze_object">public_freeze_object</a>&lt;T: key + store&gt;(obj: T) {
    <a href="transfer.md#0x2_transfer_freeze_object">freeze_object</a>(obj)
}
</code></pre>



</details>

<a name="0x2_transfer_public_transfer"></a>

## Function `public_transfer`

DEPRECATED: Transfer function that doesn't properly track objects
Use share_object() or keep objects as function returns instead


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_public_transfer">public_transfer</a>&lt;T: store, key&gt;(obj: T, recipient: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_public_transfer">public_transfer</a>&lt;T: key + store&gt;(obj: T, recipient: <b>address</b>) {
    // WORKAROUND: Store <a href="object.md#0x2_object">object</a> data for tracking before consuming
    // Extract UID <b>if</b> <a href="object.md#0x2_object">object</a> <b>has</b> one
    <a href="transfer.md#0x2_transfer_transfer_with_uid">transfer_with_uid</a>(obj, recipient);
}
</code></pre>



</details>

<a name="0x2_transfer_transfer_with_uid"></a>

## Function `transfer_with_uid`

Internal transfer that extracts UID for tracking


<pre><code><b>fun</b> <a href="transfer.md#0x2_transfer_transfer_with_uid">transfer_with_uid</a>&lt;T: store, key&gt;(obj: T, recipient: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transfer.md#0x2_transfer_transfer_with_uid">transfer_with_uid</a>&lt;T: key + store&gt;(obj: T, recipient: <b>address</b>);
</code></pre>



</details>

<a name="0x2_transfer_share_object"></a>

## Function `share_object`

Share an object by returning it instead of transferring
The caller should handle storage. This is a workaround for object tracking.


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_share_object">share_object</a>&lt;T: store&gt;(obj: T): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer_share_object">share_object</a>&lt;T: store&gt;(obj: T): T {
    obj
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
