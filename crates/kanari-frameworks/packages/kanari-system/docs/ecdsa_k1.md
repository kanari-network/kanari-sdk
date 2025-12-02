
<a name="0x2_ecdsa_k1"></a>

# Module `0x2::ecdsa_k1`



-  [Constants](#@Constants_0)
-  [Function `public_key_length`](#0x2_ecdsa_k1_public_key_length)
-  [Function `uncompressed_public_key_length`](#0x2_ecdsa_k1_uncompressed_public_key_length)
-  [Function `keccak256`](#0x2_ecdsa_k1_keccak256)
-  [Function `sha256`](#0x2_ecdsa_k1_sha256)
-  [Function `ecdsa`](#0x2_ecdsa_k1_ecdsa)
-  [Function `schnorr`](#0x2_ecdsa_k1_schnorr)
-  [Function `ecrecover`](#0x2_ecdsa_k1_ecrecover)
-  [Function `decompress_pubkey`](#0x2_ecdsa_k1_decompress_pubkey)
-  [Function `verify`](#0x2_ecdsa_k1_verify)
-  [Function `ecrecover_eth_address`](#0x2_ecdsa_k1_ecrecover_eth_address)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/hash.md#0x1_hash">0x1::hash</a>;
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x2_ecdsa_k1_ECDSA"></a>

Signature types that are valid for verify.


<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ECDSA">ECDSA</a>: u8 = 0;
</code></pre>



<a name="0x2_ecdsa_k1_ECDSA_K1_COMPRESSED_PUBKEY_LENGTH"></a>

constant codes


<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ECDSA_K1_COMPRESSED_PUBKEY_LENGTH">ECDSA_K1_COMPRESSED_PUBKEY_LENGTH</a>: u64 = 33;
</code></pre>



<a name="0x2_ecdsa_k1_ECDSA_K1_SIG_LENGTH"></a>



<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ECDSA_K1_SIG_LENGTH">ECDSA_K1_SIG_LENGTH</a>: u64 = 64;
</code></pre>



<a name="0x2_ecdsa_k1_ECDSA_K1_UNCOMPRESSED_PUBKEY_LENGTH"></a>



<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ECDSA_K1_UNCOMPRESSED_PUBKEY_LENGTH">ECDSA_K1_UNCOMPRESSED_PUBKEY_LENGTH</a>: u64 = 65;
</code></pre>



<a name="0x2_ecdsa_k1_ErrorFailToRecoverPubKey"></a>

Error if the public key cannot be recovered from the signature.


<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ErrorFailToRecoverPubKey">ErrorFailToRecoverPubKey</a>: u64 = 1;
</code></pre>



<a name="0x2_ecdsa_k1_ErrorInvalidHashType"></a>

Invalid hash function.


<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ErrorInvalidHashType">ErrorInvalidHashType</a>: u64 = 4;
</code></pre>



<a name="0x2_ecdsa_k1_ErrorInvalidMessage"></a>

Error if the message is invalid.


<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ErrorInvalidMessage">ErrorInvalidMessage</a>: u64 = 6;
</code></pre>



<a name="0x2_ecdsa_k1_ErrorInvalidPubKey"></a>

Error if the public key is invalid.


<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ErrorInvalidPubKey">ErrorInvalidPubKey</a>: u64 = 3;
</code></pre>



<a name="0x2_ecdsa_k1_ErrorInvalidSchnorrSignature"></a>

Error if the schnorr signature is invalid.


<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ErrorInvalidSchnorrSignature">ErrorInvalidSchnorrSignature</a>: u64 = 7;
</code></pre>



<a name="0x2_ecdsa_k1_ErrorInvalidSignature"></a>

Error if the signature is invalid.


<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ErrorInvalidSignature">ErrorInvalidSignature</a>: u64 = 2;
</code></pre>



<a name="0x2_ecdsa_k1_ErrorInvalidXOnlyPubKey"></a>

Error if the x only public key is invalid.


<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ErrorInvalidXOnlyPubKey">ErrorInvalidXOnlyPubKey</a>: u64 = 5;
</code></pre>



<a name="0x2_ecdsa_k1_KECCAK256"></a>

Hash function name that are valid for ecrecover and verify.


<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_KECCAK256">KECCAK256</a>: u8 = 0;
</code></pre>



<a name="0x2_ecdsa_k1_SCHNORR"></a>



<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_SCHNORR">SCHNORR</a>: u8 = 1;
</code></pre>



<a name="0x2_ecdsa_k1_SHA256"></a>



<pre><code><b>const</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_SHA256">SHA256</a>: u8 = 1;
</code></pre>



<a name="0x2_ecdsa_k1_public_key_length"></a>

## Function `public_key_length`

built-in functions


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_public_key_length">public_key_length</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_public_key_length">public_key_length</a>(): u64 {
    <a href="ecdsa_k1.md#0x2_ecdsa_k1_ECDSA_K1_COMPRESSED_PUBKEY_LENGTH">ECDSA_K1_COMPRESSED_PUBKEY_LENGTH</a>
}
</code></pre>



</details>

<a name="0x2_ecdsa_k1_uncompressed_public_key_length"></a>

## Function `uncompressed_public_key_length`



<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_uncompressed_public_key_length">uncompressed_public_key_length</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_uncompressed_public_key_length">uncompressed_public_key_length</a>(): u64 {
    <a href="ecdsa_k1.md#0x2_ecdsa_k1_ECDSA_K1_UNCOMPRESSED_PUBKEY_LENGTH">ECDSA_K1_UNCOMPRESSED_PUBKEY_LENGTH</a>
}
</code></pre>



</details>

<a name="0x2_ecdsa_k1_keccak256"></a>

## Function `keccak256`



<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_keccak256">keccak256</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_keccak256">keccak256</a>(): u8 {
    <a href="ecdsa_k1.md#0x2_ecdsa_k1_KECCAK256">KECCAK256</a>
}
</code></pre>



</details>

<a name="0x2_ecdsa_k1_sha256"></a>

## Function `sha256`



<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_sha256">sha256</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_sha256">sha256</a>(): u8 {
    <a href="ecdsa_k1.md#0x2_ecdsa_k1_SHA256">SHA256</a>
}
</code></pre>



</details>

<a name="0x2_ecdsa_k1_ecdsa"></a>

## Function `ecdsa`



<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ecdsa">ecdsa</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ecdsa">ecdsa</a>(): u8 {
    <a href="ecdsa_k1.md#0x2_ecdsa_k1_ECDSA">ECDSA</a>
}
</code></pre>



</details>

<a name="0x2_ecdsa_k1_schnorr"></a>

## Function `schnorr`



<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_schnorr">schnorr</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_schnorr">schnorr</a>(): u8 {
    <a href="ecdsa_k1.md#0x2_ecdsa_k1_SCHNORR">SCHNORR</a>
}
</code></pre>



</details>

<a name="0x2_ecdsa_k1_ecrecover"></a>

## Function `ecrecover`

@param signature: A 65-bytes signature in form (r, s, v) that is signed using
The accepted v values are {0, 1, 2, 3}.
@param msg: The message that the signature is signed against, this is raw message without hashing.
@param hash: The hash function used to hash the message when signing.

If the signature is valid, return the corresponding recovered Secpk256k1 public
key, otherwise throw error. This is similar to ecrecover in Ethereum, can only be
applied to Ecdsa signatures.


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ecrecover">ecrecover</a>(signature: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a>: u8): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ecrecover">ecrecover</a>(signature: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a>: u8): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x2_ecdsa_k1_decompress_pubkey"></a>

## Function `decompress_pubkey`

@param pubkey: A 33-bytes compressed public key, a prefix either 0x02 or 0x03 and a 256-bit integer.

If the compressed public key is valid, return the 65-bytes uncompressed public key,
otherwise throw error.


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_decompress_pubkey">decompress_pubkey</a>(pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_decompress_pubkey">decompress_pubkey</a>(pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x2_ecdsa_k1_verify"></a>

## Function `verify`

@param signature: A 64-bytes signature in form (r, s) that is signed using
Ecdsa. This is an non-recoverable signature without recovery id.
@param public_key: A 33-bytes public key that is used to sign messages.
@param msg: The message that the signature is signed against.
@param hash: The hash function used to hash the message when signing.
TODO: @param sigtype: The signature type used to distinguish which signature to be used when verifying.

If the signature is valid to the pubkey and hashed message, return true. Else false.


<pre><code><b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_verify">verify</a>(signature: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, public_key: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a>: u8): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_verify">verify</a>(
    signature: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    public_key: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a>: u8,
): bool;
</code></pre>



</details>

<a name="0x2_ecdsa_k1_ecrecover_eth_address"></a>

## Function `ecrecover_eth_address`



<pre><code><b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ecrecover_eth_address">ecrecover_eth_address</a>(sig: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1_ecrecover_eth_address">ecrecover_eth_address</a>(sig: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>use</b> std::vector;
    <b>use</b> std::hash;

    // Normalize the last byte of the signature <b>to</b> be 0 or 1.
    <b>let</b> v = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> sig, 64);
    <b>if</b> (*v == 27) {
        *v = 0;
    } <b>else</b> <b>if</b> (*v == 28) {
        *v = 1;
    } <b>else</b> <b>if</b> (*v &gt; 35) {
        *v = (*v - 1) % 2;
    };

    <b>let</b> pubkey = <a href="ecdsa_k1.md#0x2_ecdsa_k1_ecrecover">ecrecover</a>(&sig, &msg, 0);

    <b>let</b> uncompressed = <a href="ecdsa_k1.md#0x2_ecdsa_k1_decompress_pubkey">decompress_pubkey</a>(&pubkey);

    // Take the last 64 bytes of the uncompressed pubkey.
    <b>let</b> uncompressed_64 = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <b>let</b> i = 1;
    <b>while</b> (i &lt; 65) {
        <b>let</b> value = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&uncompressed, i);
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> uncompressed_64, *value);
        i = i + 1;
    };

    // Take the last 20 bytes of the <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a> of the 64-bytes uncompressed pubkey.
    <b>let</b> hashed = <a href="dependencies/move-stdlib/hash.md#0x1_hash_keccak256">hash::keccak256</a>(&uncompressed_64);
    <b>let</b> addr = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <b>let</b> i = 12;
    <b>while</b> (i &lt; 32) {
        <b>let</b> value = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&hashed, i);
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> addr, *value);
        i = i + 1;
    };

    addr
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
