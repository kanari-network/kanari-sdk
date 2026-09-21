
<a name="0x2_signature_multisig"></a>

# Module `0x2::signature_multisig`

Threshold multisig over raw signature verification.

Unlike <code><a href="multisig.md#0x2_multisig">multisig</a></code> (which custodies <code>Coin&lt;T&gt;</code> and executes proposals),
this module verifies that at least <code>threshold</code> of <code>n</code> DISTINCT public
keys signed the same message — for ed25519, secp256k1 ECDSA, and P-256
ECDSA.

Callers bind the result to their own authorization logic (e.g. minting,
admin actions, cross-chain messages). No state, no funds custody.


<a name="@Security_contract_(callers_MUST_obey)_0"></a>

## Security contract (callers MUST obey)

- Pass a FIXED committee: the <code>public_keys</code> vector must come from
on-chain state the caller controls, never from transaction input an
attacker can choose. This module rejects duplicate keys, but it cannot
tell whether the committee itself is the right one.
- Replay protection is the caller's job: build <code>msg</code> with
[<code>build_message</code>] (domain separator + unique nonce) so a signature
for one action can never be replayed as another action.


-  [Security contract (callers MUST obey)](#@Security_contract_(callers_MUST_obey)_0)
-  [Constants](#@Constants_1)
-  [Function `scheme_ed25519`](#0x2_signature_multisig_scheme_ed25519)
-  [Function `scheme_ecdsa_k1`](#0x2_signature_multisig_scheme_ecdsa_k1)
-  [Function `scheme_ecdsa_r1`](#0x2_signature_multisig_scheme_ecdsa_r1)
-  [Function `count_valid_ed25519`](#0x2_signature_multisig_count_valid_ed25519)
-  [Function `count_valid_ecdsa_k1`](#0x2_signature_multisig_count_valid_ecdsa_k1)
-  [Function `count_valid_ecdsa_r1`](#0x2_signature_multisig_count_valid_ecdsa_r1)
-  [Function `verify_threshold_ed25519`](#0x2_signature_multisig_verify_threshold_ed25519)
-  [Function `verify_threshold_ecdsa_k1`](#0x2_signature_multisig_verify_threshold_ecdsa_k1)
-  [Function `verify_threshold_ecdsa_r1`](#0x2_signature_multisig_verify_threshold_ecdsa_r1)
-  [Function `verify_threshold_mixed`](#0x2_signature_multisig_verify_threshold_mixed)
-  [Function `assert_valid_threshold`](#0x2_signature_multisig_assert_valid_threshold)
-  [Function `assert_no_duplicate_keys`](#0x2_signature_multisig_assert_no_duplicate_keys)
-  [Function `build_message`](#0x2_signature_multisig_build_message)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="dependencies/move-stdlib/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="ecdsa_k1.md#0x2_ecdsa_k1">0x2::ecdsa_k1</a>;
<b>use</b> <a href="ecdsa_r1.md#0x2_ecdsa_r1">0x2::ecdsa_r1</a>;
<b>use</b> <a href="ed25519.md#0x2_ed25519">0x2::ed25519</a>;
</code></pre>



<a name="@Constants_1"></a>

## Constants


<a name="0x2_signature_multisig_E_INVALID_THRESHOLD"></a>

Threshold is zero, or exceeds the number of keys.


<pre><code><b>const</b> <a href="signature_multisig.md#0x2_signature_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>: u64 = 2;
</code></pre>



<a name="0x2_signature_multisig_E_THRESHOLD_NOT_MET"></a>

Not enough valid signatures to meet the threshold.


<pre><code><b>const</b> <a href="signature_multisig.md#0x2_signature_multisig_E_THRESHOLD_NOT_MET">E_THRESHOLD_NOT_MET</a>: u64 = 1;
</code></pre>



<a name="0x2_signature_multisig_E_DUPLICATE_KEY"></a>

The same public key appears twice. Without this check one signer
could satisfy an n-of-n threshold alone by replaying their own
key/signature pair.


<pre><code><b>const</b> <a href="signature_multisig.md#0x2_signature_multisig_E_DUPLICATE_KEY">E_DUPLICATE_KEY</a>: u64 = 5;
</code></pre>



<a name="0x2_signature_multisig_E_INVALID_SCHEME"></a>

Unsupported scheme selector.


<pre><code><b>const</b> <a href="signature_multisig.md#0x2_signature_multisig_E_INVALID_SCHEME">E_INVALID_SCHEME</a>: u64 = 4;
</code></pre>



<a name="0x2_signature_multisig_E_LENGTH_MISMATCH"></a>

Keys and signatures vectors have different lengths.


<pre><code><b>const</b> <a href="signature_multisig.md#0x2_signature_multisig_E_LENGTH_MISMATCH">E_LENGTH_MISMATCH</a>: u64 = 3;
</code></pre>



<a name="0x2_signature_multisig_SCHEME_ECDSA_K1"></a>



<pre><code><b>const</b> <a href="signature_multisig.md#0x2_signature_multisig_SCHEME_ECDSA_K1">SCHEME_ECDSA_K1</a>: u8 = 1;
</code></pre>



<a name="0x2_signature_multisig_SCHEME_ECDSA_R1"></a>



<pre><code><b>const</b> <a href="signature_multisig.md#0x2_signature_multisig_SCHEME_ECDSA_R1">SCHEME_ECDSA_R1</a>: u8 = 2;
</code></pre>



<a name="0x2_signature_multisig_SCHEME_ED25519"></a>

Signature scheme selectors.


<pre><code><b>const</b> <a href="signature_multisig.md#0x2_signature_multisig_SCHEME_ED25519">SCHEME_ED25519</a>: u8 = 0;
</code></pre>



<a name="0x2_signature_multisig_scheme_ed25519"></a>

## Function `scheme_ed25519`



<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_scheme_ed25519">scheme_ed25519</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_scheme_ed25519">scheme_ed25519</a>(): u8 {
    <a href="signature_multisig.md#0x2_signature_multisig_SCHEME_ED25519">SCHEME_ED25519</a>
}
</code></pre>



</details>

<a name="0x2_signature_multisig_scheme_ecdsa_k1"></a>

## Function `scheme_ecdsa_k1`



<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_scheme_ecdsa_k1">scheme_ecdsa_k1</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_scheme_ecdsa_k1">scheme_ecdsa_k1</a>(): u8 {
    <a href="signature_multisig.md#0x2_signature_multisig_SCHEME_ECDSA_K1">SCHEME_ECDSA_K1</a>
}
</code></pre>



</details>

<a name="0x2_signature_multisig_scheme_ecdsa_r1"></a>

## Function `scheme_ecdsa_r1`



<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_scheme_ecdsa_r1">scheme_ecdsa_r1</a>(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_scheme_ecdsa_r1">scheme_ecdsa_r1</a>(): u8 {
    <a href="signature_multisig.md#0x2_signature_multisig_SCHEME_ECDSA_R1">SCHEME_ECDSA_R1</a>
}
</code></pre>



</details>

<a name="0x2_signature_multisig_count_valid_ed25519"></a>

## Function `count_valid_ed25519`

Count how many <code>(public_key, signature)</code> pairs verify for <code>msg</code>.
Entries that fail to verify count as zero — the call only aborts on
malformed input (length mismatch, bad threshold, bad scheme).


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_count_valid_ed25519">count_valid_ed25519</a>(public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_count_valid_ed25519">count_valid_ed25519</a>(
    public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): u64 {
    <b>let</b> n = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(public_keys);
    <b>assert</b>!(n == <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(signatures), <a href="signature_multisig.md#0x2_signature_multisig_E_LENGTH_MISMATCH">E_LENGTH_MISMATCH</a>);
    <a href="signature_multisig.md#0x2_signature_multisig_assert_no_duplicate_keys">assert_no_duplicate_keys</a>(public_keys);
    <b>let</b> (count, i) = (0u64, 0u64);
    <b>while</b> (i &lt; n) {
        <b>if</b> (<a href="ed25519.md#0x2_ed25519_verify">ed25519::verify</a>(
            <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(signatures, i),
            <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(public_keys, i),
            msg
        )) {
            count = count + 1;
        };
        i = i + 1;
    };
    count
}
</code></pre>



</details>

<a name="0x2_signature_multisig_count_valid_ecdsa_k1"></a>

## Function `count_valid_ecdsa_k1`

Count valid secp256k1 ECDSA signatures (<code><a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a></code>: 0 = Keccak256, 1 = Sha256).


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_count_valid_ecdsa_k1">count_valid_ecdsa_k1</a>(public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a>: u8): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_count_valid_ecdsa_k1">count_valid_ecdsa_k1</a>(
    public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a>: u8
): u64 {
    <b>let</b> n = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(public_keys);
    <b>assert</b>!(n == <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(signatures), <a href="signature_multisig.md#0x2_signature_multisig_E_LENGTH_MISMATCH">E_LENGTH_MISMATCH</a>);
    <a href="signature_multisig.md#0x2_signature_multisig_assert_no_duplicate_keys">assert_no_duplicate_keys</a>(public_keys);
    <b>let</b> (count, i) = (0u64, 0u64);
    <b>while</b> (i &lt; n) {
        <b>if</b> (<a href="ecdsa_k1.md#0x2_ecdsa_k1_verify">ecdsa_k1::verify</a>(
            <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(signatures, i),
            <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(public_keys, i),
            msg,
            <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a>
        )) {
            count = count + 1;
        };
        i = i + 1;
    };
    count
}
</code></pre>



</details>

<a name="0x2_signature_multisig_count_valid_ecdsa_r1"></a>

## Function `count_valid_ecdsa_r1`

Count valid P-256 ECDSA signatures (SHA-256, 64-byte raw <code>(r, s)</code>).


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_count_valid_ecdsa_r1">count_valid_ecdsa_r1</a>(public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_count_valid_ecdsa_r1">count_valid_ecdsa_r1</a>(
    public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): u64 {
    <b>let</b> n = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(public_keys);
    <b>assert</b>!(n == <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(signatures), <a href="signature_multisig.md#0x2_signature_multisig_E_LENGTH_MISMATCH">E_LENGTH_MISMATCH</a>);
    <a href="signature_multisig.md#0x2_signature_multisig_assert_no_duplicate_keys">assert_no_duplicate_keys</a>(public_keys);
    <b>let</b> (count, i) = (0u64, 0u64);
    <b>while</b> (i &lt; n) {
        <b>if</b> (<a href="ecdsa_r1.md#0x2_ecdsa_r1_verify">ecdsa_r1::verify</a>(
            <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(signatures, i),
            <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(public_keys, i),
            msg
        )) {
            count = count + 1;
        };
        i = i + 1;
    };
    count
}
</code></pre>



</details>

<a name="0x2_signature_multisig_verify_threshold_ed25519"></a>

## Function `verify_threshold_ed25519`

Verify <code>threshold</code>-of-<code>n</code> ed25519 signatures. Aborts
<code><a href="signature_multisig.md#0x2_signature_multisig_E_THRESHOLD_NOT_MET">E_THRESHOLD_NOT_MET</a></code> when fewer than <code>threshold</code> verify.


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_verify_threshold_ed25519">verify_threshold_ed25519</a>(public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, threshold: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_verify_threshold_ed25519">verify_threshold_ed25519</a>(
    public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    threshold: u64
) {
    <a href="signature_multisig.md#0x2_signature_multisig_assert_valid_threshold">assert_valid_threshold</a>(threshold, <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(public_keys));
    <b>assert</b>!(
        <a href="signature_multisig.md#0x2_signature_multisig_count_valid_ed25519">count_valid_ed25519</a>(public_keys, signatures, msg) &gt;= threshold,
        <a href="signature_multisig.md#0x2_signature_multisig_E_THRESHOLD_NOT_MET">E_THRESHOLD_NOT_MET</a>
    );
}
</code></pre>



</details>

<a name="0x2_signature_multisig_verify_threshold_ecdsa_k1"></a>

## Function `verify_threshold_ecdsa_k1`

Verify <code>threshold</code>-of-<code>n</code> secp256k1 ECDSA signatures.


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_verify_threshold_ecdsa_k1">verify_threshold_ecdsa_k1</a>(public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a>: u8, threshold: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_verify_threshold_ecdsa_k1">verify_threshold_ecdsa_k1</a>(
    public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a>: u8,
    threshold: u64
) {
    <a href="signature_multisig.md#0x2_signature_multisig_assert_valid_threshold">assert_valid_threshold</a>(threshold, <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(public_keys));
    <b>assert</b>!(
        <a href="signature_multisig.md#0x2_signature_multisig_count_valid_ecdsa_k1">count_valid_ecdsa_k1</a>(public_keys, signatures, msg, <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a>) &gt;= threshold,
        <a href="signature_multisig.md#0x2_signature_multisig_E_THRESHOLD_NOT_MET">E_THRESHOLD_NOT_MET</a>
    );
}
</code></pre>



</details>

<a name="0x2_signature_multisig_verify_threshold_ecdsa_r1"></a>

## Function `verify_threshold_ecdsa_r1`

Verify <code>threshold</code>-of-<code>n</code> P-256 ECDSA signatures.


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_verify_threshold_ecdsa_r1">verify_threshold_ecdsa_r1</a>(public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, threshold: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_verify_threshold_ecdsa_r1">verify_threshold_ecdsa_r1</a>(
    public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    threshold: u64
) {
    <a href="signature_multisig.md#0x2_signature_multisig_assert_valid_threshold">assert_valid_threshold</a>(threshold, <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(public_keys));
    <b>assert</b>!(
        <a href="signature_multisig.md#0x2_signature_multisig_count_valid_ecdsa_r1">count_valid_ecdsa_r1</a>(public_keys, signatures, msg) &gt;= threshold,
        <a href="signature_multisig.md#0x2_signature_multisig_E_THRESHOLD_NOT_MET">E_THRESHOLD_NOT_MET</a>
    );
}
</code></pre>



</details>

<a name="0x2_signature_multisig_verify_threshold_mixed"></a>

## Function `verify_threshold_mixed`

Mixed-scheme threshold: each entry picks its scheme via <code>schemes[i]</code>
(<code>0</code> = ed25519, <code>1</code> = ecdsa_k1 with <code><a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a></code>, <code>2</code> = ecdsa_r1).
Aborts <code><a href="signature_multisig.md#0x2_signature_multisig_E_INVALID_SCHEME">E_INVALID_SCHEME</a></code> on unknown selectors.


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_verify_threshold_mixed">verify_threshold_mixed</a>(schemes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a>: u8, threshold: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_verify_threshold_mixed">verify_threshold_mixed</a>(
    schemes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    signatures: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    msg: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a>: u8,
    threshold: u64
) {
    <b>let</b> n = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(public_keys);
    <b>assert</b>!(n == <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(signatures), <a href="signature_multisig.md#0x2_signature_multisig_E_LENGTH_MISMATCH">E_LENGTH_MISMATCH</a>);
    <b>assert</b>!(n == <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(schemes), <a href="signature_multisig.md#0x2_signature_multisig_E_LENGTH_MISMATCH">E_LENGTH_MISMATCH</a>);
    <a href="signature_multisig.md#0x2_signature_multisig_assert_no_duplicate_keys">assert_no_duplicate_keys</a>(public_keys);
    <a href="signature_multisig.md#0x2_signature_multisig_assert_valid_threshold">assert_valid_threshold</a>(threshold, n);
    <b>let</b> (count, i) = (0u64, 0u64);
    <b>while</b> (i &lt; n) {
        <b>let</b> scheme = *<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(schemes, i);
        <b>let</b> ok =
            <b>if</b> (scheme == <a href="signature_multisig.md#0x2_signature_multisig_SCHEME_ED25519">SCHEME_ED25519</a>) {
                <a href="ed25519.md#0x2_ed25519_verify">ed25519::verify</a>(
                    <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(signatures, i),
                    <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(public_keys, i),
                    msg
                )
            } <b>else</b> <b>if</b> (scheme == <a href="signature_multisig.md#0x2_signature_multisig_SCHEME_ECDSA_K1">SCHEME_ECDSA_K1</a>) {
                <a href="ecdsa_k1.md#0x2_ecdsa_k1_verify">ecdsa_k1::verify</a>(
                    <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(signatures, i),
                    <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(public_keys, i),
                    msg,
                    <a href="dependencies/move-stdlib/hash.md#0x1_hash">hash</a>
                )
            } <b>else</b> <b>if</b> (scheme == <a href="signature_multisig.md#0x2_signature_multisig_SCHEME_ECDSA_R1">SCHEME_ECDSA_R1</a>) {
                <a href="ecdsa_r1.md#0x2_ecdsa_r1_verify">ecdsa_r1::verify</a>(
                    <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(signatures, i),
                    <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(public_keys, i),
                    msg
                )
            } <b>else</b> {
                <b>abort</b> <a href="signature_multisig.md#0x2_signature_multisig_E_INVALID_SCHEME">E_INVALID_SCHEME</a>
            };
        <b>if</b> (ok) {
            count = count + 1;
        };
        i = i + 1;
    };
    <b>assert</b>!(count &gt;= threshold, <a href="signature_multisig.md#0x2_signature_multisig_E_THRESHOLD_NOT_MET">E_THRESHOLD_NOT_MET</a>);
}
</code></pre>



</details>

<a name="0x2_signature_multisig_assert_valid_threshold"></a>

## Function `assert_valid_threshold`



<pre><code><b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_assert_valid_threshold">assert_valid_threshold</a>(threshold: u64, n: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_assert_valid_threshold">assert_valid_threshold</a>(threshold: u64, n: u64) {
    <b>assert</b>!(
        threshold &gt; 0 && threshold &lt;= n, <a href="signature_multisig.md#0x2_signature_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>
    );
}
</code></pre>



</details>

<a name="0x2_signature_multisig_assert_no_duplicate_keys"></a>

## Function `assert_no_duplicate_keys`

Abort <code><a href="signature_multisig.md#0x2_signature_multisig_E_DUPLICATE_KEY">E_DUPLICATE_KEY</a></code> if any public key appears twice.
O(n^2) byte comparison — committees are small (typically <= 32),
so a hash set is not worth the extra dependency.


<pre><code><b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_assert_no_duplicate_keys">assert_no_duplicate_keys</a>(public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_assert_no_duplicate_keys">assert_no_duplicate_keys</a>(public_keys: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) {
    <b>let</b> n = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(public_keys);
    <b>let</b> i = 0u64;
    <b>while</b> (i &lt; n) {
        <b>let</b> j = i + 1;
        <b>while</b> (j &lt; n) {
            <b>assert</b>!(
                <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(public_keys, i) != <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(public_keys, j),
                <a href="signature_multisig.md#0x2_signature_multisig_E_DUPLICATE_KEY">E_DUPLICATE_KEY</a>
            );
            j = j + 1;
        };
        i = i + 1;
    };
}
</code></pre>



</details>

<a name="0x2_signature_multisig_build_message"></a>

## Function `build_message`

Build the message signers must sign: domain separator + length-prefixed
action id + unique nonce. Signers and verifiers MUST use this (or an
equivalent construction) so a signature cannot be replayed across
actions, modules, or chains.

Layout: <code>domain || u64 LE(action_id_len) || action_id || u64 LE(nonce)</code>.


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_build_message">build_message</a>(domain: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, action_id: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, nonce: u64): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signature_multisig.md#0x2_signature_multisig_build_message">build_message</a>(
    domain: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, action_id: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, nonce: u64
): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> msg = *domain;
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> msg, <a href="dependencies/move-stdlib/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(action_id)));
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> msg, *action_id);
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> msg, <a href="dependencies/move-stdlib/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&nonce));
    msg
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
