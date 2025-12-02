
<a name="0x2_ecdsa_r1"></a>

# Module `0x2::ecdsa_r1`



-  [Constants](#@Constants_0)
-  [Function `verify`](#0x2_ecdsa_r1_verify)
-  [Function `native_verify`](#0x2_ecdsa_r1_native_verify)
-  [Function `public_key_length`](#0x2_ecdsa_r1_public_key_length)
-  [Function `raw_signature_length`](#0x2_ecdsa_r1_raw_signature_length)


<pre><code></code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x2_ecdsa_r1_ErrorInvalidHashType"></a>



<pre><code><b>const</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_ErrorInvalidHashType">ErrorInvalidHashType</a>: u64 = 3;
</code></pre>



<a name="0x2_ecdsa_r1_ErrorInvalidPubKey"></a>



<pre><code><b>const</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_ErrorInvalidPubKey">ErrorInvalidPubKey</a>: u64 = 2;
</code></pre>



<a name="0x2_ecdsa_r1_ErrorInvalidSignature"></a>



<pre><code><b>const</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_ErrorInvalidSignature">ErrorInvalidSignature</a>: u64 = 1;
</code></pre>



<a name="0x2_ecdsa_r1_ECDSA_R1_COMPRESSED_PUBKEY_LENGTH"></a>

Compressed public key length for P-256


<pre><code><b>const</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_ECDSA_R1_COMPRESSED_PUBKEY_LENGTH">ECDSA_R1_COMPRESSED_PUBKEY_LENGTH</a>: u64 = 33;
</code></pre>



<a name="0x2_ecdsa_r1_ECDSA_R1_RAW_SIGNATURE_LENGTH"></a>

Signature length (r, s)


<pre><code><b>const</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_ECDSA_R1_RAW_SIGNATURE_LENGTH">ECDSA_R1_RAW_SIGNATURE_LENGTH</a>: u64 = 64;
</code></pre>



<a name="0x2_ecdsa_r1_HASH_TYPE_SHA256"></a>



<pre><code><b>const</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_HASH_TYPE_SHA256">HASH_TYPE_SHA256</a>: u8 = 1;
</code></pre>



<a name="0x2_ecdsa_r1_verify"></a>

## Function `verify`

Verifies an ECDSA signature over the secp256r1 (P-256) curve.
The message will be hashed with SHA256 before verification.


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_verify">verify</a>(signature: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, public_key: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_verify">verify</a>(
    signature: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    public_key: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): bool {
    <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(signature) == <a href="ecdsa_r1.md#0x2_ecdsa_r1_ECDSA_R1_RAW_SIGNATURE_LENGTH">ECDSA_R1_RAW_SIGNATURE_LENGTH</a>, <a href="ecdsa_r1.md#0x2_ecdsa_r1_ErrorInvalidSignature">ErrorInvalidSignature</a>);
    <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(public_key) == <a href="ecdsa_r1.md#0x2_ecdsa_r1_ECDSA_R1_COMPRESSED_PUBKEY_LENGTH">ECDSA_R1_COMPRESSED_PUBKEY_LENGTH</a>, <a href="ecdsa_r1.md#0x2_ecdsa_r1_ErrorInvalidPubKey">ErrorInvalidPubKey</a>);
    <a href="ecdsa_r1.md#0x2_ecdsa_r1_native_verify">native_verify</a>(signature, public_key, msg, <a href="ecdsa_r1.md#0x2_ecdsa_r1_HASH_TYPE_SHA256">HASH_TYPE_SHA256</a>)
}
</code></pre>



</details>

<a name="0x2_ecdsa_r1_native_verify"></a>

## Function `native_verify`



<pre><code><b>fun</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_native_verify">native_verify</a>(signature: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, public_key: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, hash_type: u8): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_native_verify">native_verify</a>(
    signature: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    public_key: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    hash_type: u8
): bool;
</code></pre>



</details>

<a name="0x2_ecdsa_r1_public_key_length"></a>

## Function `public_key_length`



<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_public_key_length">public_key_length</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_public_key_length">public_key_length</a>(): u64 {
    <a href="ecdsa_r1.md#0x2_ecdsa_r1_ECDSA_R1_COMPRESSED_PUBKEY_LENGTH">ECDSA_R1_COMPRESSED_PUBKEY_LENGTH</a>
}
</code></pre>



</details>

<a name="0x2_ecdsa_r1_raw_signature_length"></a>

## Function `raw_signature_length`



<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_raw_signature_length">raw_signature_length</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1_raw_signature_length">raw_signature_length</a>(): u64 {
    <a href="ecdsa_r1.md#0x2_ecdsa_r1_ECDSA_R1_RAW_SIGNATURE_LENGTH">ECDSA_R1_RAW_SIGNATURE_LENGTH</a>
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
