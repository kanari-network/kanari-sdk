
<a name="0x1_signer"></a>

# Module `0x1::signer`



-  [Function `borrow_address`](#0x1_signer_borrow_address)
-  [Function `address_of`](#0x1_signer_address_of)
-  [Function `address_to_u64`](#0x1_signer_address_to_u64)
-  [Function `address_to_bytes`](#0x1_signer_address_to_bytes)


<pre><code></code></pre>



<a name="0x1_signer_borrow_address"></a>

## Function `borrow_address`



<pre><code><b>public</b> <b>fun</b> <a href="../../dependencies/move-stdlib/signer.md#0x1_signer_borrow_address">borrow_address</a>(s: &<a href="../../dependencies/move-stdlib/signer.md#0x1_signer">signer</a>): &<b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="../../dependencies/move-stdlib/signer.md#0x1_signer_borrow_address">borrow_address</a>(s: &<a href="../../dependencies/move-stdlib/signer.md#0x1_signer">signer</a>): &<b>address</b>;
</code></pre>



</details>

<a name="0x1_signer_address_of"></a>

## Function `address_of`



<pre><code><b>public</b> <b>fun</b> <a href="../../dependencies/move-stdlib/signer.md#0x1_signer_address_of">address_of</a>(s: &<a href="../../dependencies/move-stdlib/signer.md#0x1_signer">signer</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="../../dependencies/move-stdlib/signer.md#0x1_signer_address_of">address_of</a>(s: &<a href="../../dependencies/move-stdlib/signer.md#0x1_signer">signer</a>): <b>address</b> {
    *<a href="../../dependencies/move-stdlib/signer.md#0x1_signer_borrow_address">borrow_address</a>(s)
}
</code></pre>



</details>

<a name="0x1_signer_address_to_u64"></a>

## Function `address_to_u64`

Converts an <code><b>address</b></code> to a <code>u64</code>: the value of its low 8 bytes
in big-endian order. High bytes are ignored, so this is a hash-like
projection, not an injection -- do not use it as a unique key.


<pre><code><b>public</b> <b>fun</b> <a href="../../dependencies/move-stdlib/signer.md#0x1_signer_address_to_u64">address_to_u64</a>(a: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="../../dependencies/move-stdlib/signer.md#0x1_signer_address_to_u64">address_to_u64</a>(a: <b>address</b>): u64;
</code></pre>



</details>

<a name="0x1_signer_address_to_bytes"></a>

## Function `address_to_bytes`

Converts an <code><b>address</b></code> to its 32-byte big-endian representation
(same encoding as <code>kanari_system::address::to_u256</code>).


<pre><code><b>public</b> <b>fun</b> <a href="../../dependencies/move-stdlib/signer.md#0x1_signer_address_to_bytes">address_to_bytes</a>(a: <b>address</b>): <a href="../../dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="../../dependencies/move-stdlib/signer.md#0x1_signer_address_to_bytes">address_to_bytes</a>(a: <b>address</b>): <a href="../../dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
