
<a name="0x2_balance"></a>

# Module `0x2::balance`



-  [Struct `Balance`](#0x2_balance_Balance)
-  [Struct `Supply`](#0x2_balance_Supply)
-  [Constants](#@Constants_0)
-  [Function `zero`](#0x2_balance_zero)
-  [Function `create`](#0x2_balance_create)
-  [Function `value`](#0x2_balance_value)
-  [Function `increase`](#0x2_balance_increase)
-  [Function `decrease`](#0x2_balance_decrease)
-  [Function `transfer`](#0x2_balance_transfer)
-  [Function `has_sufficient`](#0x2_balance_has_sufficient)
-  [Function `destroy`](#0x2_balance_destroy)
-  [Function `new_supply`](#0x2_balance_new_supply)
-  [Function `increase_supply`](#0x2_balance_increase_supply)
-  [Function `destroy_supply`](#0x2_balance_destroy_supply)
-  [Function `merge`](#0x2_balance_merge)
-  [Function `split`](#0x2_balance_split)


<pre><code></code></pre>



<a name="0x2_balance_Balance"></a>

## Struct `Balance`

Balance resource - Stores the balance value (generic per token type)


<pre><code><b>struct</b> <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt; <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_balance_Supply"></a>

## Struct `Supply`

Supply: mutable minting handle consumed to create balances


<pre><code><b>struct</b> <a href="balance.md#0x2_balance_Supply">Supply</a>&lt;T&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>total: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_balance_ERR_INSUFFICIENT_BALANCE"></a>

Error codes


<pre><code><b>const</b> <a href="balance.md#0x2_balance_ERR_INSUFFICIENT_BALANCE">ERR_INSUFFICIENT_BALANCE</a>: u64 = 1;
</code></pre>



<a name="0x2_balance_ERR_OVERFLOW"></a>



<pre><code><b>const</b> <a href="balance.md#0x2_balance_ERR_OVERFLOW">ERR_OVERFLOW</a>: u64 = 2;
</code></pre>



<a name="0x2_balance_ERR_ZERO_AMOUNT"></a>



<pre><code><b>const</b> <a href="balance.md#0x2_balance_ERR_ZERO_AMOUNT">ERR_ZERO_AMOUNT</a>: u64 = 3;
</code></pre>



<a name="0x2_balance_zero"></a>

## Function `zero`

Create a new zero-value Balance


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_zero">zero</a>&lt;T&gt;(): <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_zero">zero</a>&lt;T&gt;(): <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt; {
    <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt; { value: 0 }
}
</code></pre>



</details>

<a name="0x2_balance_create"></a>

## Function `create`

Create a new Balance with an initial value


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_create">create</a>&lt;T&gt;(value: u64): <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_create">create</a>&lt;T&gt;(value: u64): <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt; {
    <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt; { value }
}
</code></pre>



</details>

<a name="0x2_balance_value"></a>

## Function `value`

Get the current balance value


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_value">value</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: &<a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_value">value</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: &<a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt;): u64 {
    <a href="balance.md#0x2_balance">balance</a>.value
}
</code></pre>



</details>

<a name="0x2_balance_increase"></a>

## Function `increase`

Increase the balance value


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_increase">increase</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: &<b>mut</b> <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_increase">increase</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: &<b>mut</b> <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt;, amount: u64) {
    <b>let</b> new_value = <a href="balance.md#0x2_balance">balance</a>.value + amount;
    // Check for overflow
    <b>assert</b>!(new_value &gt;= <a href="balance.md#0x2_balance">balance</a>.value, <a href="balance.md#0x2_balance_ERR_OVERFLOW">ERR_OVERFLOW</a>);
    <a href="balance.md#0x2_balance">balance</a>.value = new_value;
}
</code></pre>



</details>

<a name="0x2_balance_decrease"></a>

## Function `decrease`

Decrease the balance value


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_decrease">decrease</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: &<b>mut</b> <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_decrease">decrease</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: &<b>mut</b> <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt;, amount: u64) {
    // Ensure amount is non-zero
    <b>assert</b>!(amount &gt; 0, <a href="balance.md#0x2_balance_ERR_ZERO_AMOUNT">ERR_ZERO_AMOUNT</a>);
    // Check for sufficient <a href="balance.md#0x2_balance">balance</a>
    <b>assert</b>!(<a href="balance.md#0x2_balance">balance</a>.value &gt;= amount, <a href="balance.md#0x2_balance_ERR_INSUFFICIENT_BALANCE">ERR_INSUFFICIENT_BALANCE</a>);
    <a href="balance.md#0x2_balance">balance</a>.value = <a href="balance.md#0x2_balance">balance</a>.value - amount;
}
</code></pre>



</details>

<a name="0x2_balance_transfer"></a>

## Function `transfer`

Transfer value from one Balance to another


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>&lt;T&gt;(from: &<b>mut</b> <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;, <b>to</b>: &<b>mut</b> <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>&lt;T&gt;(from: &<b>mut</b> <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt;, <b>to</b>: &<b>mut</b> <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt;, amount: u64) {
    // Ensure amount is non-zero
    <b>assert</b>!(amount &gt; 0, <a href="balance.md#0x2_balance_ERR_ZERO_AMOUNT">ERR_ZERO_AMOUNT</a>);
    <a href="balance.md#0x2_balance_decrease">decrease</a>&lt;T&gt;(from, amount);
    <a href="balance.md#0x2_balance_increase">increase</a>&lt;T&gt;(<b>to</b>, amount);
}
</code></pre>



</details>

<a name="0x2_balance_has_sufficient"></a>

## Function `has_sufficient`

Check if the balance is sufficient for a given amount


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_has_sufficient">has_sufficient</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: &<a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;, amount: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_has_sufficient">has_sufficient</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: &<a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt;, amount: u64): bool {
    <a href="balance.md#0x2_balance">balance</a>.value &gt;= amount
}
</code></pre>



</details>

<a name="0x2_balance_destroy"></a>

## Function `destroy`

Destroy the Balance and return its value


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_destroy">destroy</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_destroy">destroy</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt;): u64 {
    <b>let</b> <a href="balance.md#0x2_balance_Balance">Balance</a> { value } = <a href="balance.md#0x2_balance">balance</a>;
    value
}
</code></pre>



</details>

<a name="0x2_balance_new_supply"></a>

## Function `new_supply`

Create a new (empty) supply handle


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_new_supply">new_supply</a>&lt;T&gt;(): <a href="balance.md#0x2_balance_Supply">balance::Supply</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_new_supply">new_supply</a>&lt;T&gt;(): <a href="balance.md#0x2_balance_Supply">Supply</a>&lt;T&gt; {
    <a href="balance.md#0x2_balance_Supply">Supply</a>&lt;T&gt; { total: 0 }
}
</code></pre>



</details>

<a name="0x2_balance_increase_supply"></a>

## Function `increase_supply`

Increase supply: add <code>amount</code> to <code>s</code> and return a <code><a href="balance.md#0x2_balance_Balance">Balance</a></code> for the newly minted amount.


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_increase_supply">increase_supply</a>&lt;T&gt;(s: &<b>mut</b> <a href="balance.md#0x2_balance_Supply">balance::Supply</a>&lt;T&gt;, amount: u64): <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_increase_supply">increase_supply</a>&lt;T&gt;(s: &<b>mut</b> <a href="balance.md#0x2_balance_Supply">Supply</a>&lt;T&gt;, amount: u64): <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt; {
    // Ensure amount is non-zero for minting
    <b>assert</b>!(amount &gt; 0, <a href="balance.md#0x2_balance_ERR_ZERO_AMOUNT">ERR_ZERO_AMOUNT</a>);

    <b>let</b> new_total = s.total + amount;
    // Check for overflow
    <b>assert</b>!(new_total &gt;= s.total, <a href="balance.md#0x2_balance_ERR_OVERFLOW">ERR_OVERFLOW</a>);
    s.total = new_total;
    <a href="balance.md#0x2_balance_create">create</a>&lt;T&gt;(amount)
}
</code></pre>



</details>

<a name="0x2_balance_destroy_supply"></a>

## Function `destroy_supply`

Decrease/destroy a supply handle (legacy)


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_destroy_supply">destroy_supply</a>&lt;T&gt;(s: <a href="balance.md#0x2_balance_Supply">balance::Supply</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_destroy_supply">destroy_supply</a>&lt;T&gt;(s: <a href="balance.md#0x2_balance_Supply">Supply</a>&lt;T&gt;) {
    <b>let</b> <a href="balance.md#0x2_balance_Supply">Supply</a> { total: _ } = s;
}
</code></pre>



</details>

<a name="0x2_balance_merge"></a>

## Function `merge`

Merge two Balances together


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_merge">merge</a>&lt;T&gt;(dst: &<b>mut</b> <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;, src: <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_merge">merge</a>&lt;T&gt;(dst: &<b>mut</b> <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt;, src: <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt;) {
    <b>let</b> value = <a href="balance.md#0x2_balance_destroy">destroy</a>&lt;T&gt;(src);
    <a href="balance.md#0x2_balance_increase">increase</a>&lt;T&gt;(dst, value);
}
</code></pre>



</details>

<a name="0x2_balance_split"></a>

## Function `split`

Split the Balance into two


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_split">split</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: &<b>mut</b> <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;, amount: u64): <a href="balance.md#0x2_balance_Balance">balance::Balance</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance_split">split</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>: &<b>mut</b> <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt;, amount: u64): <a href="balance.md#0x2_balance_Balance">Balance</a>&lt;T&gt; {
    <a href="balance.md#0x2_balance_decrease">decrease</a>&lt;T&gt;(<a href="balance.md#0x2_balance">balance</a>, amount);
    <a href="balance.md#0x2_balance_create">create</a>&lt;T&gt;(amount)
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
