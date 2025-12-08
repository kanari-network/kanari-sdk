
<a name="0x2_deny_list"></a>

# Module `0x2::deny_list`



-  [Resource `DenyList`](#0x2_deny_list_DenyList)
-  [Resource `DenyCap`](#0x2_deny_list_DenyCap)
-  [Function `new_denylist`](#0x2_deny_list_new_denylist)
-  [Function `new_denycap`](#0x2_deny_list_new_denycap)
-  [Function `deny_list_add`](#0x2_deny_list_deny_list_add)
-  [Function `deny_list_remove`](#0x2_deny_list_deny_list_remove)


<pre><code><b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
</code></pre>



<a name="0x2_deny_list_DenyList"></a>

## Resource `DenyList`

Deny list resource storing addresses


<pre><code><b>struct</b> <a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a> <b>has</b> drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addresses: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_deny_list_DenyCap"></a>

## Resource `DenyCap`

Capability to mutate a DenyList for a specific coin type


<pre><code><b>struct</b> <a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a>&lt;T&gt; <b>has</b> drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="object.md#0x2_object_UID">object::UID</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_deny_list_new_denylist"></a>

## Function `new_denylist`

Create a new empty DenyList


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_new_denylist">new_denylist</a>(): <a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_new_denylist">new_denylist</a>(): <a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a> {
    <a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a> { addresses: <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;<b>address</b>&gt;() }
}
</code></pre>



</details>

<a name="0x2_deny_list_new_denycap"></a>

## Function `new_denycap`

Create a new DenyCap object


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_new_denycap">new_denycap</a>&lt;T&gt;(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="deny_list.md#0x2_deny_list_DenyCap">deny_list::DenyCap</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_new_denycap">new_denycap</a>&lt;T&gt;(ctx: &<b>mut</b> TxContext): <a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a>&lt;T&gt; {
    <a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a>&lt;T&gt; { id: <a href="object.md#0x2_object_new">object::new</a>(ctx) }
}
</code></pre>



</details>

<a name="0x2_deny_list_deny_list_add"></a>

## Function `deny_list_add`

Add an address to the deny list. (No-op implementation: placeholder)


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_deny_list_add">deny_list_add</a>&lt;T&gt;(_d: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, _cap: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyCap">deny_list::DenyCap</a>&lt;T&gt;, _addr: <b>address</b>, _ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_deny_list_add">deny_list_add</a>&lt;T&gt;(_d: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a>, _cap: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a>&lt;T&gt;, _addr: <b>address</b>, _ctx: &<b>mut</b> TxContext) {
    // Placeholder: Implement presence checks and <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a> insert <b>if</b> needed.
}
</code></pre>



</details>

<a name="0x2_deny_list_deny_list_remove"></a>

## Function `deny_list_remove`

Remove an address from the deny list. (No-op implementation: placeholder)


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_deny_list_remove">deny_list_remove</a>&lt;T&gt;(_d: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, _cap: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyCap">deny_list::DenyCap</a>&lt;T&gt;, _addr: <b>address</b>, _ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="deny_list.md#0x2_deny_list_deny_list_remove">deny_list_remove</a>&lt;T&gt;(_d: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyList">DenyList</a>, _cap: &<b>mut</b> <a href="deny_list.md#0x2_deny_list_DenyCap">DenyCap</a>&lt;T&gt;, _addr: <b>address</b>, _ctx: &<b>mut</b> TxContext) {
    // Placeholder: Implement removal logic <b>if</b> desired.
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
