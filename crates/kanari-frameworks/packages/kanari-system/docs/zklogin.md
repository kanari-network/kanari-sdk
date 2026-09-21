
<a name="0x2_zklogin"></a>

# Module `0x2::zklogin`

zkLogin (Sui-style) — Phase 3b (linkable JWT path) + Phase 3c (private
Groth16 path).

- <code>verify</code> / <code>verify_session</code>: ephemeral Ed25519 signature + JWT RS256
against a JWK + claim binding (iss/aud/exp/nonce) + <code>max_epoch</code> vs the
chain epoch. <code>sub</code> and <code>salt</code> are visible inputs — simple Google-login
UX, but linkable.
- <code>verify_pinned_proof</code> / <code>verify_private_session</code>: BN254 Groth16 proof
against a PINNED ceremony VK hash + public inputs. Unpinned proof
acceptance does not exist at this layer: any other VK aborts with
<code><a href="zklogin.md#0x2_zklogin_E_VK_MISMATCH">E_VK_MISMATCH</a></code>. When the circuit keeps <code>sub</code>/<code>salt</code> private, the
chain learns nothing linkable beyond the public inputs (e.g. the
derived address).


-  [Constants](#@Constants_0)
-  [Function `ephemeral_pubkey_length`](#0x2_zklogin_ephemeral_pubkey_length)
-  [Function `ephemeral_sig_length`](#0x2_zklogin_ephemeral_sig_length)
-  [Function `salt_length`](#0x2_zklogin_salt_length)
-  [Function `verify`](#0x2_zklogin_verify)
-  [Function `native_verify`](#0x2_zklogin_native_verify)
-  [Function `verify_ephemeral`](#0x2_zklogin_verify_ephemeral)
-  [Function `native_verify_ephemeral`](#0x2_zklogin_native_verify_ephemeral)
-  [Function `derive_address`](#0x2_zklogin_derive_address)
-  [Function `native_derive_address`](#0x2_zklogin_native_derive_address)
-  [Function `check_nonce`](#0x2_zklogin_check_nonce)
-  [Function `native_check_nonce`](#0x2_zklogin_native_check_nonce)
-  [Function `verify_proof`](#0x2_zklogin_verify_proof)
-  [Function `native_verify_proof`](#0x2_zklogin_native_verify_proof)
-  [Function `verify_pinned_proof`](#0x2_zklogin_verify_pinned_proof)
-  [Function `verify_private_session`](#0x2_zklogin_verify_private_session)
-  [Function `verify_session`](#0x2_zklogin_verify_session)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x2_zklogin_EPHEMERAL_PUBKEY_LENGTH"></a>



<pre><code><b>const</b> <a href="zklogin.md#0x2_zklogin_EPHEMERAL_PUBKEY_LENGTH">EPHEMERAL_PUBKEY_LENGTH</a>: u64 = 32;
</code></pre>



<a name="0x2_zklogin_EPHEMERAL_SIG_LENGTH"></a>



<pre><code><b>const</b> <a href="zklogin.md#0x2_zklogin_EPHEMERAL_SIG_LENGTH">EPHEMERAL_SIG_LENGTH</a>: u64 = 64;
</code></pre>



<a name="0x2_zklogin_E_CLAIM_MISMATCH"></a>

iss/aud/sub mismatch or address derivation failed.


<pre><code><b>const</b> <a href="zklogin.md#0x2_zklogin_E_CLAIM_MISMATCH">E_CLAIM_MISMATCH</a>: u64 = 4;
</code></pre>



<a name="0x2_zklogin_E_EXPIRED"></a>

JWT expired.


<pre><code><b>const</b> <a href="zklogin.md#0x2_zklogin_E_EXPIRED">E_EXPIRED</a>: u64 = 5;
</code></pre>



<a name="0x2_zklogin_E_INVALID_EPHEMERAL_SIG"></a>

Ephemeral signature invalid (length checks in Move, crypto in native).


<pre><code><b>const</b> <a href="zklogin.md#0x2_zklogin_E_INVALID_EPHEMERAL_SIG">E_INVALID_EPHEMERAL_SIG</a>: u64 = 3;
</code></pre>



<a name="0x2_zklogin_E_INVALID_JWK"></a>

JWK document malformed or <code>kid</code> unknown.


<pre><code><b>const</b> <a href="zklogin.md#0x2_zklogin_E_INVALID_JWK">E_INVALID_JWK</a>: u64 = 2;
</code></pre>



<a name="0x2_zklogin_E_INVALID_JWT"></a>

JWT malformed or failed to parse.


<pre><code><b>const</b> <a href="zklogin.md#0x2_zklogin_E_INVALID_JWT">E_INVALID_JWT</a>: u64 = 1;
</code></pre>



<a name="0x2_zklogin_E_INVALID_PROOF"></a>

Groth16 verifying key / proof malformed.


<pre><code><b>const</b> <a href="zklogin.md#0x2_zklogin_E_INVALID_PROOF">E_INVALID_PROOF</a>: u64 = 6;
</code></pre>



<a name="0x2_zklogin_E_VK_MISMATCH"></a>

Pinned verifying-key hash mismatch.


<pre><code><b>const</b> <a href="zklogin.md#0x2_zklogin_E_VK_MISMATCH">E_VK_MISMATCH</a>: u64 = 7;
</code></pre>



<a name="0x2_zklogin_SALT_LENGTH"></a>



<pre><code><b>const</b> <a href="zklogin.md#0x2_zklogin_SALT_LENGTH">SALT_LENGTH</a>: u64 = 32;
</code></pre>



<a name="0x2_zklogin_ephemeral_pubkey_length"></a>

## Function `ephemeral_pubkey_length`



<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_ephemeral_pubkey_length">ephemeral_pubkey_length</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_ephemeral_pubkey_length">ephemeral_pubkey_length</a>(): u64 {
    <a href="zklogin.md#0x2_zklogin_EPHEMERAL_PUBKEY_LENGTH">EPHEMERAL_PUBKEY_LENGTH</a>
}
</code></pre>



</details>

<a name="0x2_zklogin_ephemeral_sig_length"></a>

## Function `ephemeral_sig_length`



<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_ephemeral_sig_length">ephemeral_sig_length</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_ephemeral_sig_length">ephemeral_sig_length</a>(): u64 {
    <a href="zklogin.md#0x2_zklogin_EPHEMERAL_SIG_LENGTH">EPHEMERAL_SIG_LENGTH</a>
}
</code></pre>



</details>

<a name="0x2_zklogin_salt_length"></a>

## Function `salt_length`



<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_salt_length">salt_length</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_salt_length">salt_length</a>(): u64 {
    <a href="zklogin.md#0x2_zklogin_SALT_LENGTH">SALT_LENGTH</a>
}
</code></pre>



</details>

<a name="0x2_zklogin_verify"></a>

## Function `verify`

Verify JWT RS256 against <code>jwks_json</code> (<code>{"keys":[...]}</code>) and check
<code>iss</code>/<code>aud</code>/expiry at <code>now_secs</code>. Returns true on success.
Aborts with the codes above on malformed input.


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_verify">verify</a>(jwt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwks_json: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, iss: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, aud: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, now_secs: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_verify">verify</a>(
    jwt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    jwks_json: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    iss: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    aud: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    now_secs: u64
): bool {
    <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(jwt) &gt; 0, <a href="zklogin.md#0x2_zklogin_E_INVALID_JWT">E_INVALID_JWT</a>);
    <a href="zklogin.md#0x2_zklogin_native_verify">native_verify</a>(jwt, jwks_json, iss, aud, now_secs)
}
</code></pre>



</details>

<a name="0x2_zklogin_native_verify"></a>

## Function `native_verify`



<pre><code><b>fun</b> <a href="zklogin.md#0x2_zklogin_native_verify">native_verify</a>(jwt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwks_json: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, iss: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, aud: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, now_secs: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_native_verify">native_verify</a>(
    jwt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    jwks_json: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    iss: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    aud: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    now_secs: u64
): bool;
</code></pre>



</details>

<a name="0x2_zklogin_verify_ephemeral"></a>

## Function `verify_ephemeral`

Ed25519 check for the ephemeral session key (non-aborting).
Argument order is (sig, pk, msg): the native pops <code>msg</code>, then <code>pk</code>,
then <code>sig</code> off the call stack, so the first declared parameter binds
to the signature. Verified by <code>test_verify_ephemeral_accepts_fixture</code>
with a real vector (swapped order returns false).


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_verify_ephemeral">verify_ephemeral</a>(sig: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, ephemeral_pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_verify_ephemeral">verify_ephemeral</a>(
    sig: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, ephemeral_pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): bool {
    <b>assert</b>!(
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(ephemeral_pubkey) == <a href="zklogin.md#0x2_zklogin_EPHEMERAL_PUBKEY_LENGTH">EPHEMERAL_PUBKEY_LENGTH</a>,
        <a href="zklogin.md#0x2_zklogin_E_INVALID_EPHEMERAL_SIG">E_INVALID_EPHEMERAL_SIG</a>
    );
    <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(sig) == <a href="zklogin.md#0x2_zklogin_EPHEMERAL_SIG_LENGTH">EPHEMERAL_SIG_LENGTH</a>, <a href="zklogin.md#0x2_zklogin_E_INVALID_EPHEMERAL_SIG">E_INVALID_EPHEMERAL_SIG</a>);
    <a href="zklogin.md#0x2_zklogin_native_verify_ephemeral">native_verify_ephemeral</a>(sig, ephemeral_pubkey, msg)
}
</code></pre>



</details>

<a name="0x2_zklogin_native_verify_ephemeral"></a>

## Function `native_verify_ephemeral`



<pre><code><b>fun</b> <a href="zklogin.md#0x2_zklogin_native_verify_ephemeral">native_verify_ephemeral</a>(sig: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, ephemeral_pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_native_verify_ephemeral">native_verify_ephemeral</a>(
    sig: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, ephemeral_pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): bool;
</code></pre>



</details>

<a name="0x2_zklogin_derive_address"></a>

## Function `derive_address`

Derive the 32-byte zkLogin address from claims + salt.


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_derive_address">derive_address</a>(iss: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, aud: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sub: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, salt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_derive_address">derive_address</a>(
    iss: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, aud: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sub: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, salt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(salt) == <a href="zklogin.md#0x2_zklogin_SALT_LENGTH">SALT_LENGTH</a>, <a href="zklogin.md#0x2_zklogin_E_CLAIM_MISMATCH">E_CLAIM_MISMATCH</a>);
    <a href="zklogin.md#0x2_zklogin_native_derive_address">native_derive_address</a>(iss, aud, sub, salt)
}
</code></pre>



</details>

<a name="0x2_zklogin_native_derive_address"></a>

## Function `native_derive_address`



<pre><code><b>fun</b> <a href="zklogin.md#0x2_zklogin_native_derive_address">native_derive_address</a>(iss: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, aud: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sub: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, salt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_native_derive_address">native_derive_address</a>(
    iss: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, aud: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sub: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, salt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x2_zklogin_check_nonce"></a>

## Function `check_nonce`

True iff the JWT <code>nonce</code> claim equals the ephemeral binding.


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_check_nonce">check_nonce</a>(jwt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, ephemeral_pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, max_epoch: u64, randomness: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_check_nonce">check_nonce</a>(
    jwt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    ephemeral_pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    max_epoch: u64,
    randomness: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): bool {
    <b>assert</b>!(
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(ephemeral_pubkey) == <a href="zklogin.md#0x2_zklogin_EPHEMERAL_PUBKEY_LENGTH">EPHEMERAL_PUBKEY_LENGTH</a>,
        <a href="zklogin.md#0x2_zklogin_E_INVALID_EPHEMERAL_SIG">E_INVALID_EPHEMERAL_SIG</a>
    );
    <a href="zklogin.md#0x2_zklogin_native_check_nonce">native_check_nonce</a>(jwt, ephemeral_pubkey, max_epoch, randomness)
}
</code></pre>



</details>

<a name="0x2_zklogin_native_check_nonce"></a>

## Function `native_check_nonce`



<pre><code><b>fun</b> <a href="zklogin.md#0x2_zklogin_native_check_nonce">native_check_nonce</a>(jwt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, ephemeral_pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, max_epoch: u64, randomness: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_native_check_nonce">native_check_nonce</a>(
    jwt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    ephemeral_pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    max_epoch: u64,
    randomness: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): bool;
</code></pre>



</details>

<a name="0x2_zklogin_verify_proof"></a>

## Function `verify_proof`

BN254 Groth16 proof check (Phase 3c, non-aborting).

<code>vk_bytes</code>: compressed <code>VerifyingKey&lt;Bn254&gt;</code>; <code>public_inputs_bytes</code>:
concatenated 32-byte big-endian field elements; <code>proof_bytes</code>:
compressed <code>Proof&lt;Bn254&gt;</code>. No <code>sub</code>/<code>salt</code>/JWT crosses this boundary,
so a circuit that keeps them private gives real unlinkability.
Aborts <code><a href="zklogin.md#0x2_zklogin_E_INVALID_PROOF">E_INVALID_PROOF</a></code> on empty/malformed key or proof.


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_verify_proof">verify_proof</a>(vk_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, public_inputs_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_verify_proof">verify_proof</a>(
    vk_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, public_inputs_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): bool {
    <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(vk_bytes) &gt; 0, <a href="zklogin.md#0x2_zklogin_E_INVALID_PROOF">E_INVALID_PROOF</a>);
    <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(proof_bytes) &gt; 0, <a href="zklogin.md#0x2_zklogin_E_INVALID_PROOF">E_INVALID_PROOF</a>);
    <a href="zklogin.md#0x2_zklogin_native_verify_proof">native_verify_proof</a>(vk_bytes, public_inputs_bytes, proof_bytes)
}
</code></pre>



</details>

<a name="0x2_zklogin_native_verify_proof"></a>

## Function `native_verify_proof`



<pre><code><b>fun</b> <a href="zklogin.md#0x2_zklogin_native_verify_proof">native_verify_proof</a>(vk_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, public_inputs_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_native_verify_proof">native_verify_proof</a>(
    vk_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, public_inputs_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): bool;
</code></pre>



</details>

<a name="0x2_zklogin_verify_pinned_proof"></a>

## Function `verify_pinned_proof`

Pinned-key proof check: aborts unless <code>sha2_256(vk_bytes)</code> equals the
ceremony VK hash the contract trusts, then verifies the proof.
Deployments hardcode their ceremony hash here (or pass it from a
versioned config object); proofs against any other VK abort, so a
locally-generated (toxic-waste) setup can never pass as canonical.


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_verify_pinned_proof">verify_pinned_proof</a>(expected_vk_hash: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, vk_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, public_inputs_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_verify_pinned_proof">verify_pinned_proof</a>(
    expected_vk_hash: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    vk_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    public_inputs_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    proof_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): bool {
    <b>assert</b>!(std::vector::length(&expected_vk_hash) == 32, <a href="zklogin.md#0x2_zklogin_E_VK_MISMATCH">E_VK_MISMATCH</a>);
    <b>let</b> ok = <a href="zklogin.md#0x2_zklogin_verify_proof">verify_proof</a>(&vk_bytes, public_inputs_bytes, proof_bytes);
    <b>assert</b>!(std::hash::sha2_256(vk_bytes) == expected_vk_hash, <a href="zklogin.md#0x2_zklogin_E_VK_MISMATCH">E_VK_MISMATCH</a>);
    ok
}
</code></pre>



</details>

<a name="0x2_zklogin_verify_private_session"></a>

## Function `verify_private_session`

Private session check: Groth16 proof valid AND ephemeral sig valid.
The JWT-private counterpart of <code>verify_session</code>.

The VK hash MUST be pinned: proofs against any other VK (including
toxic-waste demo setups) abort with <code><a href="zklogin.md#0x2_zklogin_E_VK_MISMATCH">E_VK_MISMATCH</a></code> before any
pairing work is trusted. There is no unpinned entry point on
purpose — use <code>verify_pinned_proof</code> + <code>verify_ephemeral</code> directly
only if you re-check the pin yourself.


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_verify_private_session">verify_private_session</a>(expected_vk_hash: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, vk_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, public_inputs_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, ephemeral_pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, ephemeral_sig: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_verify_private_session">verify_private_session</a>(
    expected_vk_hash: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    vk_bytes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    public_inputs_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    proof_bytes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    ephemeral_pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    ephemeral_sig: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): bool {
    <a href="zklogin.md#0x2_zklogin_verify_pinned_proof">verify_pinned_proof</a>(expected_vk_hash, vk_bytes, public_inputs_bytes, proof_bytes)
        && <a href="zklogin.md#0x2_zklogin_verify_ephemeral">verify_ephemeral</a>(ephemeral_sig, ephemeral_pubkey, msg)
}
</code></pre>



</details>

<a name="0x2_zklogin_verify_session"></a>

## Function `verify_session`

Full session check: JWT valid AND session live AND nonce bound AND
ephemeral sig valid. One call for login gates.

<code>max_epoch</code> is enforced against the chain epoch: sessions from an
older epoch abort with <code><a href="zklogin.md#0x2_zklogin_E_EXPIRED">E_EXPIRED</a></code>. This is the on-chain counterpart
of the Rust <code>check_max_epoch</code> used at admission — and the ONLY
timeliness bound proof-mode sessions have (they carry no JWT <code>exp</code>).


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_verify_session">verify_session</a>(jwt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, jwks_json: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, iss: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, aud: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, now_secs: u64, ephemeral_pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, max_epoch: u64, randomness: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, ephemeral_sig: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, ctx: &<a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zklogin.md#0x2_zklogin_verify_session">verify_session</a>(
    jwt: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    jwks_json: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    iss: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    aud: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    now_secs: u64,
    ephemeral_pubkey: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    max_epoch: u64,
    randomness: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    ephemeral_sig: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    ctx: &TxContext
): bool {
    <b>assert</b>!(max_epoch &gt;= <a href="tx_context.md#0x2_tx_context_epoch">tx_context::epoch</a>(ctx), <a href="zklogin.md#0x2_zklogin_E_EXPIRED">E_EXPIRED</a>);
    <a href="zklogin.md#0x2_zklogin_verify">verify</a>(jwt, jwks_json, iss, aud, now_secs)
        && <a href="zklogin.md#0x2_zklogin_check_nonce">check_nonce</a>(jwt, ephemeral_pubkey, max_epoch, randomness)
        && <a href="zklogin.md#0x2_zklogin_verify_ephemeral">verify_ephemeral</a>(ephemeral_sig, ephemeral_pubkey, msg)
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
