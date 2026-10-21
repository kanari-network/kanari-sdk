
<a name="0x2_types"></a>

# Module `0x2::types`

Type-convention checks mirroring <code>sui::types</code>.


-  [Function `is_one_time_witness`](#0x2_types_is_one_time_witness)
-  [Function `claim_witness`](#0x2_types_claim_witness)


<pre><code></code></pre>



<a name="0x2_types_is_one_time_witness"></a>

## Function `is_one_time_witness`

True iff <code>T</code> follows the one-time-witness convention: a <code>drop</code> struct
named after its defining module in all caps (e.g. <code>my_coin::MY_COIN</code>).

Convention check only — it does not consult on-chain state. Combine
with module-local construction (only the defining module can pack the
witness value) to bound authority, as <code><a href="coin.md#0x2_coin">coin</a></code> and <code><a href="balance.md#0x2_balance">balance</a></code> do.


<pre><code><b>public</b> <b>fun</b> <a href="types.md#0x2_types_is_one_time_witness">is_one_time_witness</a>&lt;T: drop&gt;(witness: &T): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="types.md#0x2_types_is_one_time_witness">is_one_time_witness</a>&lt;T: drop&gt;(witness: &T): bool;
</code></pre>



</details>

<a name="0x2_types_claim_witness"></a>

## Function `claim_witness`

Claim <code>T</code> for currency creation. Aborts when <code>T</code> was already claimed,
giving each coin type a single canonical currency. The host persists
claims; a second <code>create_currency</code>/<code>create_regulated_currency</code> for
the same <code>T</code> aborts even across transactions.


<pre><code><b>public</b> <b>fun</b> <a href="types.md#0x2_types_claim_witness">claim_witness</a>&lt;T: drop&gt;(witness: &T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="types.md#0x2_types_claim_witness">claim_witness</a>&lt;T: drop&gt;(witness: &T);
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
