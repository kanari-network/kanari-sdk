
<a name="0x2_metadata"></a>

 Module `0x2::metadata`



-  [Resource `Metadata`](#0x2_metadata_Metadata)
-  [Function `new`](#0x2_metadata_new)
-  [Function `set_owner`](#0x2_metadata_set_owner)
-  [Function `get_owner`](#0x2_metadata_get_owner)
-  [Function `set_hash`](#0x2_metadata_set_hash)
-  [Function `get_hash`](#0x2_metadata_get_hash)
-  [Function `store`](#0x2_metadata_store)
-  [Function `verify_hash`](#0x2_metadata_verify_hash)


<pre><code></code></pre>



<a name="0x2_metadata_Metadata"></a>

# Resource `Metadata`

Metadata stores information about file ownership and content hash


<pre><code><b>struct</b> <a href="metadata.md#0x2_metadata_Metadata">Metadata</a> <b>has</b> drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>content_hash: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_metadata_new"></a>

# Function `new`

Create a new empty metadata instance


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_new">new</a>(): <a href="metadata.md#0x2_metadata_Metadata">metadata::Metadata</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_new">new</a>(): <a href="metadata.md#0x2_metadata_Metadata">Metadata</a> {
    <a href="metadata.md#0x2_metadata_Metadata">Metadata</a> {
        owner: @0x0,
        content_hash: <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;(),
    }
}
</code></pre>



</details>

<a name="0x2_metadata_set_owner"></a>

# Function `set_owner`

Set the owner of the metadata


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_set_owner">set_owner</a>(<a href="metadata.md#0x2_metadata">metadata</a>: &<b>mut</b> <a href="metadata.md#0x2_metadata_Metadata">metadata::Metadata</a>, owner: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_set_owner">set_owner</a>(<a href="metadata.md#0x2_metadata">metadata</a>: &<b>mut</b> <a href="metadata.md#0x2_metadata_Metadata">Metadata</a>, owner: <b>address</b>) {
    <a href="metadata.md#0x2_metadata">metadata</a>.owner = owner;
}
</code></pre>



</details>

<a name="0x2_metadata_get_owner"></a>

# Function `get_owner`

Get the owner of the metadata


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_get_owner">get_owner</a>(<a href="metadata.md#0x2_metadata">metadata</a>: &<a href="metadata.md#0x2_metadata_Metadata">metadata::Metadata</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_get_owner">get_owner</a>(<a href="metadata.md#0x2_metadata">metadata</a>: &<a href="metadata.md#0x2_metadata_Metadata">Metadata</a>): <b>address</b> {
    <a href="metadata.md#0x2_metadata">metadata</a>.owner
}
</code></pre>



</details>

<a name="0x2_metadata_set_hash"></a>

# Function `set_hash`

Set the content hash of the metadata


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_set_hash">set_hash</a>(<a href="metadata.md#0x2_metadata">metadata</a>: &<b>mut</b> <a href="metadata.md#0x2_metadata_Metadata">metadata::Metadata</a>, content_hash: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_set_hash">set_hash</a>(<a href="metadata.md#0x2_metadata">metadata</a>: &<b>mut</b> <a href="metadata.md#0x2_metadata_Metadata">Metadata</a>, content_hash: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <a href="metadata.md#0x2_metadata">metadata</a>.content_hash = content_hash;
}
</code></pre>



</details>

<a name="0x2_metadata_get_hash"></a>

# Function `get_hash`

Get the content hash from the metadata


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_get_hash">get_hash</a>(<a href="metadata.md#0x2_metadata">metadata</a>: &<a href="metadata.md#0x2_metadata_Metadata">metadata::Metadata</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_get_hash">get_hash</a>(<a href="metadata.md#0x2_metadata">metadata</a>: &<a href="metadata.md#0x2_metadata_Metadata">Metadata</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="metadata.md#0x2_metadata">metadata</a>.content_hash
}
</code></pre>



</details>

<a name="0x2_metadata_store"></a>

# Function `store`

Store the metadata and return it


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_store">store</a>(<a href="metadata.md#0x2_metadata">metadata</a>: <a href="metadata.md#0x2_metadata_Metadata">metadata::Metadata</a>): <a href="metadata.md#0x2_metadata_Metadata">metadata::Metadata</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_store">store</a>(<a href="metadata.md#0x2_metadata">metadata</a>: <a href="metadata.md#0x2_metadata_Metadata">Metadata</a>): <a href="metadata.md#0x2_metadata_Metadata">Metadata</a> {
    // In a real implementation, this might involve storing the <a href="metadata.md#0x2_metadata">metadata</a> on-chain
    // For now, we just <b>return</b> it
    <a href="metadata.md#0x2_metadata">metadata</a>
}
</code></pre>



</details>

<a name="0x2_metadata_verify_hash"></a>

# Function `verify_hash`

Verify if the provided hash matches the one stored in metadata


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_verify_hash">verify_hash</a>(<a href="metadata.md#0x2_metadata">metadata</a>: &<a href="metadata.md#0x2_metadata_Metadata">metadata::Metadata</a>, content_hash: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="metadata.md#0x2_metadata_verify_hash">verify_hash</a>(<a href="metadata.md#0x2_metadata">metadata</a>: &<a href="metadata.md#0x2_metadata_Metadata">Metadata</a>, content_hash: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool {
    <a href="metadata.md#0x2_metadata">metadata</a>.content_hash == content_hash
}
</code></pre>



</details>
