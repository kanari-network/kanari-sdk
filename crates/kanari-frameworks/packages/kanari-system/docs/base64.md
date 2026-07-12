
<a name="0x2_base64"></a>

# Module `0x2::base64`

Base64 and Base64URL encoding/decoding utilities


-  [Function `decode`](#0x2_base64_decode)
-  [Function `native_decode`](#0x2_base64_native_decode)
-  [Function `encode`](#0x2_base64_encode)
-  [Function `native_encode`](#0x2_base64_native_encode)


<pre><code></code></pre>



<a name="0x2_base64_decode"></a>

## Function `decode`

Decodes a base64 or base64url encoded string into bytes
Supports both standard base64 and base64url (URL-safe) encoding


<pre><code><b>public</b> <b>fun</b> <a href="base64.md#0x2_base64_decode">decode</a>(input: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="base64.md#0x2_base64_decode">decode</a>(input: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="base64.md#0x2_base64_native_decode">native_decode</a>(input)
}
</code></pre>



</details>

<a name="0x2_base64_native_decode"></a>

## Function `native_decode`



<pre><code><b>fun</b> <a href="base64.md#0x2_base64_native_decode">native_decode</a>(input: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="base64.md#0x2_base64_native_decode">native_decode</a>(input: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x2_base64_encode"></a>

## Function `encode`

Encodes bytes into base64 string


<pre><code><b>public</b> <b>fun</b> <a href="base64.md#0x2_base64_encode">encode</a>(input: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="base64.md#0x2_base64_encode">encode</a>(input: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="base64.md#0x2_base64_native_encode">native_encode</a>(input)
}
</code></pre>



</details>

<a name="0x2_base64_native_encode"></a>

## Function `native_encode`



<pre><code><b>fun</b> <a href="base64.md#0x2_base64_native_encode">native_encode</a>(input: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="base64.md#0x2_base64_native_encode">native_encode</a>(input: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
