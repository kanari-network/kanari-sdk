
<a name="0x2_clock"></a>

# Module `0x2::clock`

APIs for accessing time from move calls, via the <code><a href="clock.md#0x2_clock_Clock">Clock</a></code>: a unique
shared object that is created during genesis.


-  [Resource `Clock`](#0x2_clock_Clock)
-  [Constants](#@Constants_0)
-  [Function `timestamp_ms`](#0x2_clock_timestamp_ms)
-  [Function `create`](#0x2_clock_create)
-  [Function `consensus_commit_prologue`](#0x2_clock_consensus_commit_prologue)


<pre><code><b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
</code></pre>



<a name="0x2_clock_Clock"></a>

## Resource `Clock`

Singleton shared object that exposes time to Move calls.


<pre><code><b>struct</b> <a href="clock.md#0x2_clock_Clock">Clock</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="object.md#0x2_object_UID">object::UID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>timestamp_ms: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_clock_E_NOT_SYSTEM_ADDRESS"></a>

Sender is not @0x0 the system address.


<pre><code><b>const</b> <a href="clock.md#0x2_clock_E_NOT_SYSTEM_ADDRESS">E_NOT_SYSTEM_ADDRESS</a>: u64 = 0;
</code></pre>



<a name="0x2_clock_E_TIMESTAMP_NOT_MONOTONIC"></a>

Timestamp is not monotonic (not greater than or equal to current time)


<pre><code><b>const</b> <a href="clock.md#0x2_clock_E_TIMESTAMP_NOT_MONOTONIC">E_TIMESTAMP_NOT_MONOTONIC</a>: u64 = 1;
</code></pre>



<a name="0x2_clock_timestamp_ms"></a>

## Function `timestamp_ms`

The <code><a href="clock.md#0x2_clock">clock</a></code>'s current timestamp as a running total of
milliseconds since an arbitrary point in the past.


<pre><code><b>public</b> <b>fun</b> <a href="clock.md#0x2_clock_timestamp_ms">timestamp_ms</a>(<a href="clock.md#0x2_clock">clock</a>: &<a href="clock.md#0x2_clock_Clock">clock::Clock</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="clock.md#0x2_clock_timestamp_ms">timestamp_ms</a>(<a href="clock.md#0x2_clock">clock</a>: &<a href="clock.md#0x2_clock_Clock">Clock</a>): u64 {
    <a href="clock.md#0x2_clock">clock</a>.timestamp_ms
}
</code></pre>



</details>

<a name="0x2_clock_create"></a>

## Function `create`

Create and share the singleton Clock -- this function is
called exactly once, during genesis.


<pre><code><b>public</b> <b>fun</b> <a href="clock.md#0x2_clock_create">create</a>(ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="clock.md#0x2_clock_create">create</a>(ctx: &<b>mut</b> TxContext) {
    <b>assert</b>!(<a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx) == @0x0, <a href="clock.md#0x2_clock_E_NOT_SYSTEM_ADDRESS">E_NOT_SYSTEM_ADDRESS</a>);

    <b>let</b> <a href="clock.md#0x2_clock">clock</a> = <a href="clock.md#0x2_clock_Clock">Clock</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        timestamp_ms: 0,
    };

    <a href="object.md#0x2_object_save_object">object::save_object</a>(&<a href="clock.md#0x2_clock">clock</a>);

   // 🚨 Transfer ownership <b>to</b> System Address (@0x0)
   // To properly clear the <a href="clock.md#0x2_clock">clock</a> value from the function according <b>to</b> Move rules.
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(<a href="clock.md#0x2_clock">clock</a>, @0x0);
}
</code></pre>



</details>

<a name="0x2_clock_consensus_commit_prologue"></a>

## Function `consensus_commit_prologue`

System call: Validator (the Rust node) will call this function every time the block is closed.


<pre><code><b>public</b> <b>fun</b> <a href="clock.md#0x2_clock_consensus_commit_prologue">consensus_commit_prologue</a>(<a href="clock.md#0x2_clock">clock</a>: &<b>mut</b> <a href="clock.md#0x2_clock_Clock">clock::Clock</a>, timestamp_ms: u64, ctx: &<a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="clock.md#0x2_clock_consensus_commit_prologue">consensus_commit_prologue</a>(<a href="clock.md#0x2_clock">clock</a>: &<b>mut</b> <a href="clock.md#0x2_clock_Clock">Clock</a>, timestamp_ms: u64, ctx: &TxContext) {
    // Requires that the call be made only through the System Validator.
    <b>assert</b>!(<a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx) == @0x0, <a href="clock.md#0x2_clock_E_NOT_SYSTEM_ADDRESS">E_NOT_SYSTEM_ADDRESS</a>);
    // Ensure that the new timestamp is greater than or equal <b>to</b> the current one
    // <b>to</b> maintain monotonicity of time on the blockchain
    <b>assert</b>!(timestamp_ms &gt;= <a href="clock.md#0x2_clock">clock</a>.timestamp_ms, <a href="clock.md#0x2_clock_E_TIMESTAMP_NOT_MONOTONIC">E_TIMESTAMP_NOT_MONOTONIC</a>);
    <a href="clock.md#0x2_clock">clock</a>.timestamp_ms = timestamp_ms;
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
