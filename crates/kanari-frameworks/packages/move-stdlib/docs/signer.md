
<a name="0x1_signer"></a>

# Module `0x1::signer`



-  [Function `borrow_address`](#0x1_signer_borrow_address)
-  [Function `address_of`](#0x1_signer_address_of)
-  [Function `address_to_u64`](#0x1_signer_address_to_u64)
-  [Function `address_to_bytes`](#0x1_signer_address_to_bytes)


<pre><code><b>use</b> <a href="address.md#0x1_address">0x1::address</a>;
</code></pre>



<a name="0x1_signer_borrow_address"></a>

## Function `borrow_address`



<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_borrow_address">borrow_address</a>(s: &<a href="signer.md#0x1_signer">signer</a>): &<b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_borrow_address">borrow_address</a>(s: &<a href="signer.md#0x1_signer">signer</a>): &<b>address</b>;
</code></pre>



</details>

<a name="0x1_signer_address_of"></a>

## Function `address_of`



<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_address_of">address_of</a>(s: &<a href="signer.md#0x1_signer">signer</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_address_of">address_of</a>(s: &<a href="signer.md#0x1_signer">signer</a>): <b>address</b> {
    *<a href="signer.md#0x1_signer_borrow_address">borrow_address</a>(s)
}
</code></pre>



</details>

<a name="0x1_signer_address_to_u64"></a>

## Function `address_to_u64`

Converts an <code><b>address</b></code> to a <code>u64</code>.

Note: A native implementation may provide a platform-specific
conversion (for example extracting the lower 8 bytes). The Move
stdlib in this repository uses a simple, deterministic Move-side
implementation so tests can run without depending on VM natives.
The implementation returns <code>0</code> for all inputs but is deterministic
(so repeated calls on the same input compare equal).


<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_address_to_u64">address_to_u64</a>(_a: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_address_to_u64">address_to_u64</a>(_a: <b>address</b>): u64 {
    0
}
</code></pre>



</details>

<a name="0x1_signer_address_to_bytes"></a>

## Function `address_to_bytes`

Converts an <code><b>address</b></code> to its raw byte representation.

This Move-side implementation returns a <code><a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;</code> of length
<code>std::address::length()</code> filled with zero bytes. It's not a true
serialization of the address but is sufficient for unit tests that
only compare lengths or perform equality of repeated conversions.


<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_address_to_bytes">address_to_bytes</a>(_a: <b>address</b>): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_address_to_bytes">address_to_bytes</a>(_a: <b>address</b>): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> v = std::vector::empty&lt;u8&gt;();
    <b>let</b> mut_v = v;
    <b>let</b> i = 0;
    <b>let</b> len = std::address::length();
    <b>let</b> mut_index = i;
    <b>while</b> (mut_index &lt; len) {
        std::vector::push_back(&<b>mut</b> mut_v, 0);
        mut_index = mut_index + 1;
    };
    mut_v
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")
