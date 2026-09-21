
<a name="0x2_multisig"></a>

# Module `0x2::multisig`

Secure multi-signature wallet.

Flow: create (+ fund) -> propose -> approve (until threshold) -> execute.
Every state-changing execution re-validates its preconditions at execution
time (balances, owner set, threshold), so proposals that go stale between
proposal and execution abort instead of doing the wrong thing.

Security properties:
- The wallet custodies <code>Coin&lt;T&gt;</code> itself; transfers split from that balance.
Executors cannot substitute their own funds.
- Every proposal is bound to exactly one wallet (<code>wallet_id</code>); a proposal
approved for wallet A can never execute against wallet B.
- Owner-set / threshold mutations happen for real inside <code>execute</code>.
- Proposals expire (<code>ttl_ms</code>, 0 = never) and can be cancelled by the
proposer or garbage-collected by anyone after expiry.


-  [Resource `MultisigWallet`](#0x2_multisig_MultisigWallet)
-  [Resource `TransactionProposal`](#0x2_multisig_TransactionProposal)
-  [Struct `WalletCreatedEvent`](#0x2_multisig_WalletCreatedEvent)
-  [Struct `FundsDepositedEvent`](#0x2_multisig_FundsDepositedEvent)
-  [Struct `TransactionProposedEvent`](#0x2_multisig_TransactionProposedEvent)
-  [Struct `TransactionApprovedEvent`](#0x2_multisig_TransactionApprovedEvent)
-  [Struct `TransactionExecutedEvent`](#0x2_multisig_TransactionExecutedEvent)
-  [Struct `ProposalCancelledEvent`](#0x2_multisig_ProposalCancelledEvent)
-  [Struct `OwnerChangedEvent`](#0x2_multisig_OwnerChangedEvent)
-  [Struct `ThresholdChangedEvent`](#0x2_multisig_ThresholdChangedEvent)
-  [Struct `TestCoin`](#0x2_multisig_TestCoin)
-  [Constants](#@Constants_0)
-  [Function `create_wallet`](#0x2_multisig_create_wallet)
-  [Function `deposit`](#0x2_multisig_deposit)
-  [Function `balance`](#0x2_multisig_balance)
-  [Function `new_proposal`](#0x2_multisig_new_proposal)
-  [Function `propose_transfer`](#0x2_multisig_propose_transfer)
-  [Function `propose_add_owner`](#0x2_multisig_propose_add_owner)
-  [Function `propose_remove_owner`](#0x2_multisig_propose_remove_owner)
-  [Function `propose_change_threshold`](#0x2_multisig_propose_change_threshold)
-  [Function `approve_transaction`](#0x2_multisig_approve_transaction)
-  [Function `cancel_proposal`](#0x2_multisig_cancel_proposal)
-  [Function `delete_expired_proposal`](#0x2_multisig_delete_expired_proposal)
-  [Function `execute_transaction`](#0x2_multisig_execute_transaction)
-  [Function `execute_transfer_checked`](#0x2_multisig_execute_transfer_checked)
-  [Function `create_wallet_entry`](#0x2_multisig_create_wallet_entry)
-  [Function `deposit_entry`](#0x2_multisig_deposit_entry)
-  [Function `propose_transfer_entry`](#0x2_multisig_propose_transfer_entry)
-  [Function `approve_entry`](#0x2_multisig_approve_entry)
-  [Function `execute_entry`](#0x2_multisig_execute_entry)
-  [Function `cancel_entry`](#0x2_multisig_cancel_entry)
-  [Function `execute_borrowed`](#0x2_multisig_execute_borrowed)
-  [Function `cancel_borrowed`](#0x2_multisig_cancel_borrowed)
-  [Function `is_owner`](#0x2_multisig_is_owner)
-  [Function `owner_count`](#0x2_multisig_owner_count)
-  [Function `get_threshold`](#0x2_multisig_get_threshold)
-  [Function `get_transaction_count`](#0x2_multisig_get_transaction_count)
-  [Function `has_enough_approvals`](#0x2_multisig_has_enough_approvals)
-  [Function `get_approval_count`](#0x2_multisig_get_approval_count)
-  [Function `is_executed`](#0x2_multisig_is_executed)
-  [Function `get_proposer`](#0x2_multisig_get_proposer)
-  [Function `get_tx_type`](#0x2_multisig_get_tx_type)
-  [Function `get_target_address`](#0x2_multisig_get_target_address)
-  [Function `get_amount`](#0x2_multisig_get_amount)
-  [Function `get_description`](#0x2_multisig_get_description)
-  [Function `get_expires_at_ms`](#0x2_multisig_get_expires_at_ms)
-  [Function `is_expired`](#0x2_multisig_is_expired)
-  [Function `wallet_address`](#0x2_multisig_wallet_address)
-  [Function `assert_bound`](#0x2_multisig_assert_bound)
-  [Function `check_duplicate_owners`](#0x2_multisig_check_duplicate_owners)
-  [Function `has_approved`](#0x2_multisig_has_approved)
-  [Function `remove_owner_addr`](#0x2_multisig_remove_owner_addr)
-  [Function `decode_threshold`](#0x2_multisig_decode_threshold)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="dependencies/move-stdlib/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="bcs.md#0x2_bcs">0x2::bcs</a>;
<b>use</b> <a href="borrow.md#0x2_borrow">0x2::borrow</a>;
<b>use</b> <a href="coin.md#0x2_coin">0x2::coin</a>;
<b>use</b> <a href="deny_list.md#0x2_deny_list">0x2::deny_list</a>;
<b>use</b> <a href="event.md#0x2_event">0x2::event</a>;
<b>use</b> <a href="math.md#0x2_math">0x2::math</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
</code></pre>



<a name="0x2_multisig_MultisigWallet"></a>

## Resource `MultisigWallet`

Multisig wallet. Custodies <code>Coin&lt;T&gt;</code> directly: the only way funds leave
is an executed transfer proposal.


<pre><code><b>struct</b> <a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt; <b>has</b> store, key
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
<code>owners: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>threshold: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_count: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>funds: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_multisig_TransactionProposal"></a>

## Resource `TransactionProposal`

Transaction proposal. Always bound to one wallet via <code>wallet_id</code>.


<pre><code><b>struct</b> <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> <b>has</b> store, key
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
<code>wallet_id: <a href="object.md#0x2_object_ID">object::ID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>tx_type: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>proposer: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>target_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>payload: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>approvers: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>executed: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>created_at_ms: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>expires_at_ms: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_multisig_WalletCreatedEvent"></a>

## Struct `WalletCreatedEvent`



<pre><code><b>struct</b> <a href="multisig.md#0x2_multisig_WalletCreatedEvent">WalletCreatedEvent</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>wallet_id: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>owners: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>threshold: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_multisig_FundsDepositedEvent"></a>

## Struct `FundsDepositedEvent`



<pre><code><b>struct</b> <a href="multisig.md#0x2_multisig_FundsDepositedEvent">FundsDepositedEvent</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>wallet_id: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>depositor: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_multisig_TransactionProposedEvent"></a>

## Struct `TransactionProposedEvent`



<pre><code><b>struct</b> <a href="multisig.md#0x2_multisig_TransactionProposedEvent">TransactionProposedEvent</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>wallet_id: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_id: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>tx_type: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>proposer: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>target_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>expires_at_ms: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_multisig_TransactionApprovedEvent"></a>

## Struct `TransactionApprovedEvent`



<pre><code><b>struct</b> <a href="multisig.md#0x2_multisig_TransactionApprovedEvent">TransactionApprovedEvent</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>wallet_id: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_id: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>approver: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>approval_count: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>threshold: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_multisig_TransactionExecutedEvent"></a>

## Struct `TransactionExecutedEvent`



<pre><code><b>struct</b> <a href="multisig.md#0x2_multisig_TransactionExecutedEvent">TransactionExecutedEvent</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>wallet_id: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_id: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>executor: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_multisig_ProposalCancelledEvent"></a>

## Struct `ProposalCancelledEvent`



<pre><code><b>struct</b> <a href="multisig.md#0x2_multisig_ProposalCancelledEvent">ProposalCancelledEvent</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>wallet_id: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_id: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>canceller: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_multisig_OwnerChangedEvent"></a>

## Struct `OwnerChangedEvent`



<pre><code><b>struct</b> <a href="multisig.md#0x2_multisig_OwnerChangedEvent">OwnerChangedEvent</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>wallet_id: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>action: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_multisig_ThresholdChangedEvent"></a>

## Struct `ThresholdChangedEvent`



<pre><code><b>struct</b> <a href="multisig.md#0x2_multisig_ThresholdChangedEvent">ThresholdChangedEvent</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>wallet_id: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_threshold: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_threshold: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_multisig_TestCoin"></a>

## Struct `TestCoin`

Witness for the throwaway test coin. Module-private: unusable outside.


<pre><code><b>struct</b> <a href="multisig.md#0x2_multisig_TestCoin">TestCoin</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_multisig_E_ALREADY_APPROVED"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_ALREADY_APPROVED">E_ALREADY_APPROVED</a>: u64 = 2;
</code></pre>



<a name="0x2_multisig_E_ALREADY_OWNER"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_ALREADY_OWNER">E_ALREADY_OWNER</a>: u64 = 11;
</code></pre>



<a name="0x2_multisig_E_BINDING_MISMATCH"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_BINDING_MISMATCH">E_BINDING_MISMATCH</a>: u64 = 13;
</code></pre>



<a name="0x2_multisig_E_CANNOT_REMOVE_LAST_OWNER"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_CANNOT_REMOVE_LAST_OWNER">E_CANNOT_REMOVE_LAST_OWNER</a>: u64 = 8;
</code></pre>



<a name="0x2_multisig_E_DENIED"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_DENIED">E_DENIED</a>: u64 = 18;
</code></pre>



<a name="0x2_multisig_E_EMPTY_OWNERS"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_EMPTY_OWNERS">E_EMPTY_OWNERS</a>: u64 = 6;
</code></pre>



<a name="0x2_multisig_E_INSUFFICIENT_BALANCE"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_INSUFFICIENT_BALANCE">E_INSUFFICIENT_BALANCE</a>: u64 = 10;
</code></pre>



<a name="0x2_multisig_E_INVALID_PAYLOAD"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_INVALID_PAYLOAD">E_INVALID_PAYLOAD</a>: u64 = 15;
</code></pre>



<a name="0x2_multisig_E_INVALID_THRESHOLD"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>: u64 = 5;
</code></pre>



<a name="0x2_multisig_E_INVALID_TRANSACTION_TYPE"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_INVALID_TRANSACTION_TYPE">E_INVALID_TRANSACTION_TYPE</a>: u64 = 9;
</code></pre>



<a name="0x2_multisig_E_NOT_EXPIRED"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_NOT_EXPIRED">E_NOT_EXPIRED</a>: u64 = 16;
</code></pre>



<a name="0x2_multisig_E_NOT_OWNER"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_NOT_OWNER">E_NOT_OWNER</a>: u64 = 1;
</code></pre>



<a name="0x2_multisig_E_NOT_PROPOSER"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_NOT_PROPOSER">E_NOT_PROPOSER</a>: u64 = 14;
</code></pre>



<a name="0x2_multisig_E_OWNER_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_OWNER_NOT_FOUND">E_OWNER_NOT_FOUND</a>: u64 = 7;
</code></pre>



<a name="0x2_multisig_E_PROPOSAL_EXPIRED"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_PROPOSAL_EXPIRED">E_PROPOSAL_EXPIRED</a>: u64 = 12;
</code></pre>



<a name="0x2_multisig_E_THRESHOLD_NOT_MET"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_THRESHOLD_NOT_MET">E_THRESHOLD_NOT_MET</a>: u64 = 3;
</code></pre>



<a name="0x2_multisig_E_TRANSACTION_ALREADY_EXECUTED"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_TRANSACTION_ALREADY_EXECUTED">E_TRANSACTION_ALREADY_EXECUTED</a>: u64 = 4;
</code></pre>



<a name="0x2_multisig_E_ZERO_ADDRESS"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_ZERO_ADDRESS">E_ZERO_ADDRESS</a>: u64 = 19;
</code></pre>



<a name="0x2_multisig_E_ZERO_AMOUNT"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_ZERO_AMOUNT">E_ZERO_AMOUNT</a>: u64 = 17;
</code></pre>



<a name="0x2_multisig_NO_EXPIRY"></a>

<code>ttl_ms == 0</code> means "never expires".


<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_NO_EXPIRY">NO_EXPIRY</a>: u64 = 0;
</code></pre>



<a name="0x2_multisig_TX_TYPE_ADD_OWNER"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_TX_TYPE_ADD_OWNER">TX_TYPE_ADD_OWNER</a>: u8 = 1;
</code></pre>



<a name="0x2_multisig_TX_TYPE_CHANGE_THRESHOLD"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_TX_TYPE_CHANGE_THRESHOLD">TX_TYPE_CHANGE_THRESHOLD</a>: u8 = 3;
</code></pre>



<a name="0x2_multisig_TX_TYPE_REMOVE_OWNER"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_TX_TYPE_REMOVE_OWNER">TX_TYPE_REMOVE_OWNER</a>: u8 = 2;
</code></pre>



<a name="0x2_multisig_TX_TYPE_TRANSFER"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_TX_TYPE_TRANSFER">TX_TYPE_TRANSFER</a>: u8 = 0;
</code></pre>



<a name="0x2_multisig_create_wallet"></a>

## Function `create_wallet`

Create a wallet funded with <code>initial_funds</code> (use <code><a href="coin.md#0x2_coin_zero">coin::zero</a></code> for empty).


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_create_wallet">create_wallet</a>&lt;T&gt;(owners: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, threshold: u64, initial_funds: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_create_wallet">create_wallet</a>&lt;T&gt;(
    owners: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    threshold: u64,
    initial_funds: Coin&lt;T&gt;,
    ctx: &<b>mut</b> TxContext
): <a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt; {
    <b>let</b> owners_len = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&owners);
    <b>assert</b>!(owners_len &gt; 0, <a href="multisig.md#0x2_multisig_E_EMPTY_OWNERS">E_EMPTY_OWNERS</a>);
    <b>assert</b>!(threshold &gt; 0, <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
    <b>assert</b>!(threshold &lt;= (owners_len <b>as</b> u64), <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
    <a href="multisig.md#0x2_multisig_check_duplicate_owners">check_duplicate_owners</a>(&owners);
    // Reject the zero <b>address</b>: it can never approve and poisons the owner set.
    <b>let</b> i = 0;
    <b>while</b> (i &lt; owners_len) {
        <b>assert</b>!(*<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&owners, i) != @0x0, <a href="multisig.md#0x2_multisig_E_ZERO_ADDRESS">E_ZERO_ADDRESS</a>);
        i = i + 1;
    };

    <b>let</b> wallet = <a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt; {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        owners,
        threshold,
        transaction_count: 0,
        funds: initial_funds
    };
    <a href="event.md#0x2_event_emit">event::emit</a>(
        <a href="multisig.md#0x2_multisig_WalletCreatedEvent">WalletCreatedEvent</a> {
            wallet_id: <a href="multisig.md#0x2_multisig_wallet_address">wallet_address</a>(&wallet),
            owners: wallet.owners,
            threshold: wallet.threshold
        }
    );
    wallet
}
</code></pre>



</details>

<a name="0x2_multisig_deposit"></a>

## Function `deposit`

Top up the wallet. Anyone can deposit.


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_deposit">deposit</a>&lt;T&gt;(wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, funds: <a href="coin.md#0x2_coin_Coin">coin::Coin</a>&lt;T&gt;, ctx: &<a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_deposit">deposit</a>&lt;T&gt;(
    wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;, funds: Coin&lt;T&gt;, ctx: &TxContext
) {
    <b>let</b> amount = <a href="coin.md#0x2_coin_value">coin::value</a>(&funds);
    <a href="coin.md#0x2_coin_join">coin::join</a>(&<b>mut</b> wallet.funds, funds);
    <a href="event.md#0x2_event_emit">event::emit</a>(
        <a href="multisig.md#0x2_multisig_FundsDepositedEvent">FundsDepositedEvent</a> {
            wallet_id: <a href="multisig.md#0x2_multisig_wallet_address">wallet_address</a>(wallet),
            depositor: <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx),
            amount
        }
    );
}
</code></pre>



</details>

<a name="0x2_multisig_balance"></a>

## Function `balance`

Current custodial balance.


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance">balance</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="balance.md#0x2_balance">balance</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;): u64 {
    <a href="coin.md#0x2_coin_value">coin::value</a>(&wallet.funds)
}
</code></pre>



</details>

<a name="0x2_multisig_new_proposal"></a>

## Function `new_proposal`

Shared constructor: owner-only, proposer auto-approves, TTL enforced.


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_new_proposal">new_proposal</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, tx_type: u8, target_address: <b>address</b>, amount: u64, payload: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, ttl_ms: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_new_proposal">new_proposal</a>&lt;T&gt;(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;,
    tx_type: u8,
    target_address: <b>address</b>,
    amount: u64,
    payload: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
    ttl_ms: u64,
    ctx: &<b>mut</b> TxContext
): <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
    <b>let</b> sender = <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);
    <b>assert</b>!(<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, sender), <a href="multisig.md#0x2_multisig_E_NOT_OWNER">E_NOT_OWNER</a>);

    <b>let</b> now_ms = <a href="tx_context.md#0x2_tx_context_epoch_timestamp_ms">tx_context::epoch_timestamp_ms</a>(ctx);
    <b>let</b> expires_at_ms =
        <b>if</b> (ttl_ms == <a href="multisig.md#0x2_multisig_NO_EXPIRY">NO_EXPIRY</a>) {
            <a href="multisig.md#0x2_multisig_NO_EXPIRY">NO_EXPIRY</a>
        } <b>else</b> {
            // Saturating add: absurd TTLs clamp instead of aborting <b>with</b> a
            // generic arithmetic error.
            <a href="math.md#0x2_math_saturating_add_u64">math::saturating_add_u64</a>(now_ms, ttl_ms)
        };
    <b>let</b> proposal = <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        wallet_id: <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&wallet.id),
        tx_type,
        proposer: sender,
        target_address,
        amount,
        payload,
        description,
        approvers: <a href="dependencies/move-stdlib/vector.md#0x1_vector_singleton">vector::singleton</a>(sender),
        executed: <b>false</b>,
        created_at_ms: now_ms,
        expires_at_ms
    };
    <a href="event.md#0x2_event_emit">event::emit</a>(
        <a href="multisig.md#0x2_multisig_TransactionProposedEvent">TransactionProposedEvent</a> {
            wallet_id: <a href="multisig.md#0x2_multisig_wallet_address">wallet_address</a>(wallet),
            transaction_id: <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&<a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&proposal.id)),
            tx_type,
            proposer: sender,
            target_address,
            amount,
            expires_at_ms
        }
    );
    proposal
}
</code></pre>



</details>

<a name="0x2_multisig_propose_transfer"></a>

## Function `propose_transfer`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_transfer">propose_transfer</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, target_address: <b>address</b>, amount: u64, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, ttl_ms: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_transfer">propose_transfer</a>&lt;T&gt;(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;,
    target_address: <b>address</b>,
    amount: u64,
    description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
    ttl_ms: u64,
    ctx: &<b>mut</b> TxContext
): <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
    <b>assert</b>!(amount &gt; 0, <a href="multisig.md#0x2_multisig_E_ZERO_AMOUNT">E_ZERO_AMOUNT</a>);
    <a href="multisig.md#0x2_multisig_new_proposal">new_proposal</a>(
        wallet,
        <a href="multisig.md#0x2_multisig_TX_TYPE_TRANSFER">TX_TYPE_TRANSFER</a>,
        target_address,
        amount,
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;(),
        description,
        ttl_ms,
        ctx
    )
}
</code></pre>



</details>

<a name="0x2_multisig_propose_add_owner"></a>

## Function `propose_add_owner`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_add_owner">propose_add_owner</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, new_owner: <b>address</b>, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, ttl_ms: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_add_owner">propose_add_owner</a>&lt;T&gt;(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;,
    new_owner: <b>address</b>,
    description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
    ttl_ms: u64,
    ctx: &<b>mut</b> TxContext
): <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
    <b>assert</b>!(new_owner != @0x0, <a href="multisig.md#0x2_multisig_E_ZERO_ADDRESS">E_ZERO_ADDRESS</a>);
    <b>assert</b>!(!<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, new_owner), <a href="multisig.md#0x2_multisig_E_ALREADY_OWNER">E_ALREADY_OWNER</a>);
    <a href="multisig.md#0x2_multisig_new_proposal">new_proposal</a>(
        wallet,
        <a href="multisig.md#0x2_multisig_TX_TYPE_ADD_OWNER">TX_TYPE_ADD_OWNER</a>,
        new_owner,
        0,
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;(),
        description,
        ttl_ms,
        ctx
    )
}
</code></pre>



</details>

<a name="0x2_multisig_propose_remove_owner"></a>

## Function `propose_remove_owner`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_remove_owner">propose_remove_owner</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, owner_to_remove: <b>address</b>, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, ttl_ms: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_remove_owner">propose_remove_owner</a>&lt;T&gt;(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;,
    owner_to_remove: <b>address</b>,
    description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
    ttl_ms: u64,
    ctx: &<b>mut</b> TxContext
): <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
    <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&wallet.owners) &gt; 1, <a href="multisig.md#0x2_multisig_E_CANNOT_REMOVE_LAST_OWNER">E_CANNOT_REMOVE_LAST_OWNER</a>);
    <b>assert</b>!(<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, owner_to_remove), <a href="multisig.md#0x2_multisig_E_OWNER_NOT_FOUND">E_OWNER_NOT_FOUND</a>);
    <a href="multisig.md#0x2_multisig_new_proposal">new_proposal</a>(
        wallet,
        <a href="multisig.md#0x2_multisig_TX_TYPE_REMOVE_OWNER">TX_TYPE_REMOVE_OWNER</a>,
        owner_to_remove,
        0,
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;(),
        description,
        ttl_ms,
        ctx
    )
}
</code></pre>



</details>

<a name="0x2_multisig_propose_change_threshold"></a>

## Function `propose_change_threshold`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_change_threshold">propose_change_threshold</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, new_threshold: u64, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, ttl_ms: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_change_threshold">propose_change_threshold</a>&lt;T&gt;(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;,
    new_threshold: u64,
    description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
    ttl_ms: u64,
    ctx: &<b>mut</b> TxContext
): <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
    <b>assert</b>!(new_threshold &gt; 0, <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
    <b>assert</b>!(new_threshold &lt;= <a href="multisig.md#0x2_multisig_owner_count">owner_count</a>(wallet), <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
    <a href="multisig.md#0x2_multisig_new_proposal">new_proposal</a>(
        wallet,
        <a href="multisig.md#0x2_multisig_TX_TYPE_CHANGE_THRESHOLD">TX_TYPE_CHANGE_THRESHOLD</a>,
        @0x0,
        0,
        <a href="dependencies/move-stdlib/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&new_threshold),
        description,
        ttl_ms,
        ctx
    )
}
</code></pre>



</details>

<a name="0x2_multisig_approve_transaction"></a>

## Function `approve_transaction`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_approve_transaction">approve_transaction</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, proposal: &<b>mut</b> <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_approve_transaction">approve_transaction</a>&lt;T&gt;(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;, proposal: &<b>mut</b> <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>, ctx: &<b>mut</b> TxContext
) {
    <b>let</b> sender = <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);
    <a href="multisig.md#0x2_multisig_assert_bound">assert_bound</a>(wallet, proposal);
    <b>assert</b>!(<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, sender), <a href="multisig.md#0x2_multisig_E_NOT_OWNER">E_NOT_OWNER</a>);
    <b>assert</b>!(!proposal.executed, <a href="multisig.md#0x2_multisig_E_TRANSACTION_ALREADY_EXECUTED">E_TRANSACTION_ALREADY_EXECUTED</a>);
    <b>assert</b>!(!<a href="multisig.md#0x2_multisig_is_expired">is_expired</a>(proposal, ctx), <a href="multisig.md#0x2_multisig_E_PROPOSAL_EXPIRED">E_PROPOSAL_EXPIRED</a>);
    <b>assert</b>!(!<a href="multisig.md#0x2_multisig_has_approved">has_approved</a>(proposal, sender), <a href="multisig.md#0x2_multisig_E_ALREADY_APPROVED">E_ALREADY_APPROVED</a>);

    <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> proposal.approvers, sender);
    <a href="event.md#0x2_event_emit">event::emit</a>(
        <a href="multisig.md#0x2_multisig_TransactionApprovedEvent">TransactionApprovedEvent</a> {
            wallet_id: <a href="multisig.md#0x2_multisig_wallet_address">wallet_address</a>(wallet),
            transaction_id: <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&<a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&proposal.id)),
            approver: sender,
            approval_count: (<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&proposal.approvers) <b>as</b> u64),
            threshold: wallet.threshold
        }
    );
}
</code></pre>



</details>

<a name="0x2_multisig_cancel_proposal"></a>

## Function `cancel_proposal`

Cancel a live proposal. Only the proposer may cancel.


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_cancel_proposal">cancel_proposal</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, proposal: <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>, ctx: &<a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_cancel_proposal">cancel_proposal</a>&lt;T&gt;(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;, proposal: <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>, ctx: &TxContext
) {
    <b>let</b> sender = <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);
    <a href="multisig.md#0x2_multisig_assert_bound">assert_bound</a>(wallet, &proposal);
    <b>assert</b>!(sender == proposal.proposer, <a href="multisig.md#0x2_multisig_E_NOT_PROPOSER">E_NOT_PROPOSER</a>);
    <b>assert</b>!(!proposal.executed, <a href="multisig.md#0x2_multisig_E_TRANSACTION_ALREADY_EXECUTED">E_TRANSACTION_ALREADY_EXECUTED</a>);

    <a href="event.md#0x2_event_emit">event::emit</a>(
        <a href="multisig.md#0x2_multisig_ProposalCancelledEvent">ProposalCancelledEvent</a> {
            wallet_id: <a href="multisig.md#0x2_multisig_wallet_address">wallet_address</a>(wallet),
            transaction_id: <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&<a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&proposal.id)),
            canceller: sender
        }
    );
    <b>let</b> <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
        id,
        wallet_id: _,
        tx_type: _,
        proposer: _,
        target_address: _,
        amount: _,
        payload: _,
        description: _,
        approvers: _,
        executed: _,
        created_at_ms: _,
        expires_at_ms: _
    } = proposal;
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
}
</code></pre>



</details>

<a name="0x2_multisig_delete_expired_proposal"></a>

## Function `delete_expired_proposal`

Garbage-collect an expired proposal. Anyone may call; reclaims storage.


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_delete_expired_proposal">delete_expired_proposal</a>(proposal: <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>, ctx: &<a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_delete_expired_proposal">delete_expired_proposal</a>(
    proposal: <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>, ctx: &TxContext
) {
    <b>assert</b>!(<a href="multisig.md#0x2_multisig_is_expired">is_expired</a>(&proposal, ctx), <a href="multisig.md#0x2_multisig_E_NOT_EXPIRED">E_NOT_EXPIRED</a>);
    <a href="event.md#0x2_event_emit">event::emit</a>(
        <a href="multisig.md#0x2_multisig_ProposalCancelledEvent">ProposalCancelledEvent</a> {
            wallet_id: <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&proposal.wallet_id),
            transaction_id: <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&<a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&proposal.id)),
            canceller: <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx)
        }
    );
    <b>let</b> <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
        id,
        wallet_id: _,
        tx_type: _,
        proposer: _,
        target_address: _,
        amount: _,
        payload: _,
        description: _,
        approvers: _,
        executed: _,
        created_at_ms: _,
        expires_at_ms: _
    } = proposal;
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
}
</code></pre>



</details>

<a name="0x2_multisig_execute_transaction"></a>

## Function `execute_transaction`

Execute a proposal whose threshold is met. Consumes the proposal.
All effects are re-validated here: balance, owner set, threshold bounds.


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_execute_transaction">execute_transaction</a>&lt;T&gt;(wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, proposal: <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_execute_transaction">execute_transaction</a>&lt;T&gt;(
    wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;, proposal: <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>, ctx: &<b>mut</b> TxContext
) {
    <b>let</b> sender = <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);
    <a href="multisig.md#0x2_multisig_assert_bound">assert_bound</a>(wallet, &proposal);
    <b>assert</b>!(<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, sender), <a href="multisig.md#0x2_multisig_E_NOT_OWNER">E_NOT_OWNER</a>);
    <b>assert</b>!(!proposal.executed, <a href="multisig.md#0x2_multisig_E_TRANSACTION_ALREADY_EXECUTED">E_TRANSACTION_ALREADY_EXECUTED</a>);
    <b>assert</b>!(!<a href="multisig.md#0x2_multisig_is_expired">is_expired</a>(&proposal, ctx), <a href="multisig.md#0x2_multisig_E_PROPOSAL_EXPIRED">E_PROPOSAL_EXPIRED</a>);
    <b>assert</b>!(<a href="multisig.md#0x2_multisig_has_enough_approvals">has_enough_approvals</a>(wallet, &proposal), <a href="multisig.md#0x2_multisig_E_THRESHOLD_NOT_MET">E_THRESHOLD_NOT_MET</a>);

    <b>let</b> wallet_id = <a href="multisig.md#0x2_multisig_wallet_address">wallet_address</a>(wallet);
    <b>let</b> proposal_id = <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&<a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&proposal.id));

    <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_TRANSFER">TX_TYPE_TRANSFER</a>) {
        <b>let</b> amount = proposal.amount;
        <b>assert</b>!(amount &gt; 0, <a href="multisig.md#0x2_multisig_E_ZERO_AMOUNT">E_ZERO_AMOUNT</a>);
        <b>assert</b>!(<a href="coin.md#0x2_coin_value">coin::value</a>(&wallet.funds) &gt;= amount, <a href="multisig.md#0x2_multisig_E_INSUFFICIENT_BALANCE">E_INSUFFICIENT_BALANCE</a>);
        <b>let</b> out = <a href="coin.md#0x2_coin_split">coin::split</a>(&<b>mut</b> wallet.funds, amount, ctx);
        <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(out, proposal.target_address);
    } <b>else</b> <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_ADD_OWNER">TX_TYPE_ADD_OWNER</a>) {
        <b>let</b> new_owner = proposal.target_address;
        // Re-check: another proposal may have added them first.
        <b>assert</b>!(!<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, new_owner), <a href="multisig.md#0x2_multisig_E_ALREADY_OWNER">E_ALREADY_OWNER</a>);
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> wallet.owners, new_owner);
        <a href="event.md#0x2_event_emit">event::emit</a>(<a href="multisig.md#0x2_multisig_OwnerChangedEvent">OwnerChangedEvent</a> { wallet_id, action: 0, owner: new_owner });
    } <b>else</b> <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_REMOVE_OWNER">TX_TYPE_REMOVE_OWNER</a>) {
        <b>let</b> doomed = proposal.target_address;
        <b>assert</b>!(<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, doomed), <a href="multisig.md#0x2_multisig_E_OWNER_NOT_FOUND">E_OWNER_NOT_FOUND</a>);
        <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&wallet.owners) &gt; 1, <a href="multisig.md#0x2_multisig_E_CANNOT_REMOVE_LAST_OWNER">E_CANNOT_REMOVE_LAST_OWNER</a>);
        <a href="multisig.md#0x2_multisig_remove_owner_addr">remove_owner_addr</a>(&<b>mut</b> wallet.owners, doomed);
        // Never leave the wallet in a state <b>where</b> the threshold is
        // unreachable; proposers must lower the threshold first.
        <b>assert</b>!(<a href="multisig.md#0x2_multisig_owner_count">owner_count</a>(wallet) &gt;= wallet.threshold, <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
        <a href="event.md#0x2_event_emit">event::emit</a>(<a href="multisig.md#0x2_multisig_OwnerChangedEvent">OwnerChangedEvent</a> { wallet_id, action: 1, owner: doomed });
    } <b>else</b> <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_CHANGE_THRESHOLD">TX_TYPE_CHANGE_THRESHOLD</a>) {
        <b>let</b> new_threshold = <a href="multisig.md#0x2_multisig_decode_threshold">decode_threshold</a>(&proposal.payload);
        <b>assert</b>!(new_threshold &gt; 0, <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
        <b>assert</b>!(new_threshold &lt;= <a href="multisig.md#0x2_multisig_owner_count">owner_count</a>(wallet), <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
        <b>let</b> <b>old</b> = wallet.threshold;
        wallet.threshold = new_threshold;
        <a href="event.md#0x2_event_emit">event::emit</a>(
            <a href="multisig.md#0x2_multisig_ThresholdChangedEvent">ThresholdChangedEvent</a> { wallet_id, old_threshold: <b>old</b>, new_threshold }
        );
    } <b>else</b> {
        <b>abort</b> <a href="multisig.md#0x2_multisig_E_INVALID_TRANSACTION_TYPE">E_INVALID_TRANSACTION_TYPE</a>
    };

    wallet.transaction_count = wallet.transaction_count + 1;
    <a href="event.md#0x2_event_emit">event::emit</a>(
        <a href="multisig.md#0x2_multisig_TransactionExecutedEvent">TransactionExecutedEvent</a> {
            wallet_id,
            transaction_id: proposal_id,
            executor: sender
        }
    );

    <b>let</b> <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
        id,
        wallet_id: _,
        tx_type: _,
        proposer: _,
        target_address: _,
        amount: _,
        payload: _,
        description: _,
        approvers: _,
        executed: _,
        created_at_ms: _,
        expires_at_ms: _
    } = proposal;
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
}
</code></pre>



</details>

<a name="0x2_multisig_execute_transfer_checked"></a>

## Function `execute_transfer_checked`

Regulated execution: like <code>execute_transaction</code>, but transfer proposals
additionally abort <code><a href="multisig.md#0x2_multisig_E_DENIED">E_DENIED</a></code> when the recipient is on <code>deny</code>.
Non-transfer proposals ignore the list. Use this entry point whenever
the wallet custodies a regulated coin.


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_execute_transfer_checked">execute_transfer_checked</a>&lt;T&gt;(wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, proposal: <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>, deny: &<a href="deny_list.md#0x2_deny_list_DenyList">deny_list::DenyList</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_execute_transfer_checked">execute_transfer_checked</a>&lt;T&gt;(
    wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;,
    proposal: <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>,
    deny: &DenyList,
    ctx: &<b>mut</b> TxContext
) {
    <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_TRANSFER">TX_TYPE_TRANSFER</a>) {
        <b>assert</b>!(!<a href="deny_list.md#0x2_deny_list_contains">deny_list::contains</a>(deny, proposal.target_address), <a href="multisig.md#0x2_multisig_E_DENIED">E_DENIED</a>);
    };
    <a href="multisig.md#0x2_multisig_execute_transaction">execute_transaction</a>(wallet, proposal, ctx);
}
</code></pre>



</details>

<a name="0x2_multisig_create_wallet_entry"></a>

## Function `create_wallet_entry`

Create a wallet funded with <code>amount</code> drawn from a coin object you
own. The coin stays in your wallet: <code>amount</code> is split off into the
new multisig wallet, the remainder is saved back. Pass the coin's
32-byte object ID as <code>funds_id</code> (declared in <code>object_inputs</code>).


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig.md#0x2_multisig_create_wallet_entry">create_wallet_entry</a>&lt;T&gt;(owners: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, threshold: u64, funds_id: <b>address</b>, amount: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig.md#0x2_multisig_create_wallet_entry">create_wallet_entry</a>&lt;T&gt;(
    owners: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    threshold: u64,
    funds_id: <b>address</b>,
    amount: u64,
    ctx: &<b>mut</b> TxContext
) {
    <b>let</b> source = <a href="object.md#0x2_object_borrow_global_mut">object::borrow_global_mut</a>&lt;Coin&lt;T&gt;&gt;(funds_id);
    <b>let</b> initial_funds = <a href="coin.md#0x2_coin_split">coin::split</a>(source, amount, ctx);
    <a href="object.md#0x2_object_save_object">object::save_object</a>(source);
    <b>let</b> wallet = <a href="multisig.md#0x2_multisig_create_wallet">create_wallet</a>(owners, threshold, initial_funds, ctx);
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(wallet, <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx));
}
</code></pre>



</details>

<a name="0x2_multisig_deposit_entry"></a>

## Function `deposit_entry`

Top up the wallet by splitting <code>amount</code> off a coin you own.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig.md#0x2_multisig_deposit_entry">deposit_entry</a>&lt;T&gt;(wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, funds_id: <b>address</b>, amount: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig.md#0x2_multisig_deposit_entry">deposit_entry</a>&lt;T&gt;(
    wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;,
    funds_id: <b>address</b>,
    amount: u64,
    ctx: &<b>mut</b> TxContext
) {
    <b>let</b> source = <a href="object.md#0x2_object_borrow_global_mut">object::borrow_global_mut</a>&lt;Coin&lt;T&gt;&gt;(funds_id);
    <b>let</b> funds = <a href="coin.md#0x2_coin_split">coin::split</a>(source, amount, ctx);
    <a href="object.md#0x2_object_save_object">object::save_object</a>(source);
    <a href="multisig.md#0x2_multisig_deposit">deposit</a>(wallet, funds, ctx);
}
</code></pre>



</details>

<a name="0x2_multisig_propose_transfer_entry"></a>

## Function `propose_transfer_entry`

Propose a transfer. The proposal object goes to the proposer.

Objects arrive by ID and are borrowed inside (the runtime's escrow
pattern): entry params that are object refs cannot be bound from raw
address args, so every entry below takes IDs and borrows.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig.md#0x2_multisig_propose_transfer_entry">propose_transfer_entry</a>&lt;T&gt;(wallet_id: <b>address</b>, target_address: <b>address</b>, amount: u64, description: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, ttl_ms: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig.md#0x2_multisig_propose_transfer_entry">propose_transfer_entry</a>&lt;T&gt;(
    wallet_id: <b>address</b>,
    target_address: <b>address</b>,
    amount: u64,
    description: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    ttl_ms: u64,
    ctx: &<b>mut</b> TxContext
) {
    <b>let</b> wallet = <a href="object.md#0x2_object_borrow_global">object::borrow_global</a>&lt;<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;&gt;(wallet_id);
    <b>let</b> proposal =
        <a href="multisig.md#0x2_multisig_propose_transfer">propose_transfer</a>(
            wallet,
            target_address,
            amount,
            <a href="dependencies/move-stdlib/string.md#0x1_string_utf8">string::utf8</a>(description),
            ttl_ms,
            ctx
        );
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(proposal, <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx));
}
</code></pre>



</details>

<a name="0x2_multisig_approve_entry"></a>

## Function `approve_entry`

Approve someone else's proposal (proposer auto-approved at creation).


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig.md#0x2_multisig_approve_entry">approve_entry</a>&lt;T&gt;(wallet_id: <b>address</b>, proposal_id: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig.md#0x2_multisig_approve_entry">approve_entry</a>&lt;T&gt;(
    wallet_id: <b>address</b>, proposal_id: <b>address</b>, ctx: &<b>mut</b> TxContext
) {
    <b>let</b> wallet = <a href="borrow.md#0x2_borrow_borrow">borrow::borrow</a>&lt;<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;&gt;(wallet_id);
    <b>let</b> proposal = <a href="object.md#0x2_object_borrow_global_mut">object::borrow_global_mut</a>&lt;<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>&gt;(proposal_id);
    <a href="multisig.md#0x2_multisig_approve_transaction">approve_transaction</a>(wallet, proposal, ctx);
    // Persist the new approval: without this the borrowed mutation only
    // lives in the VM writeback set for tracked borrows and the second
    // approval is lost on commit.
    <a href="borrow.md#0x2_borrow_save">borrow::save</a>(proposal);
}
</code></pre>



</details>

<a name="0x2_multisig_execute_entry"></a>

## Function `execute_entry`

Execute a proposal whose threshold is met. Marks the proposal executed
(tombstone) instead of deleting it: entry functions cannot move a
stored object by value, and the flag blocks any re-execution.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig.md#0x2_multisig_execute_entry">execute_entry</a>&lt;T&gt;(wallet_id: <b>address</b>, proposal_id: <b>address</b>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig.md#0x2_multisig_execute_entry">execute_entry</a>&lt;T&gt;(
    wallet_id: <b>address</b>, proposal_id: <b>address</b>, ctx: &<b>mut</b> TxContext
) {
    <a href="borrow.md#0x2_borrow_assert_distinct">borrow::assert_distinct</a>(wallet_id, proposal_id);
    <b>let</b> wallet = <a href="object.md#0x2_object_borrow_global_mut">object::borrow_global_mut</a>&lt;<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;&gt;(wallet_id);
    <b>let</b> proposal = <a href="object.md#0x2_object_borrow_global_mut">object::borrow_global_mut</a>&lt;<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>&gt;(proposal_id);
    <a href="multisig.md#0x2_multisig_execute_borrowed">execute_borrowed</a>(wallet, proposal, ctx);
    <a href="borrow.md#0x2_borrow_save">borrow::save</a>(wallet);
    <a href="borrow.md#0x2_borrow_save">borrow::save</a>(proposal);
}
</code></pre>



</details>

<a name="0x2_multisig_cancel_entry"></a>

## Function `cancel_entry`

Cancel your own live proposal. Tombstones like <code>execute_entry</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig.md#0x2_multisig_cancel_entry">cancel_entry</a>&lt;T&gt;(wallet_id: <b>address</b>, proposal_id: <b>address</b>, ctx: &<a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig.md#0x2_multisig_cancel_entry">cancel_entry</a>&lt;T&gt;(
    wallet_id: <b>address</b>, proposal_id: <b>address</b>, ctx: &TxContext
) {
    <b>let</b> wallet = <a href="borrow.md#0x2_borrow_borrow">borrow::borrow</a>&lt;<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;&gt;(wallet_id);
    <b>let</b> proposal = <a href="object.md#0x2_object_borrow_global_mut">object::borrow_global_mut</a>&lt;<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>&gt;(proposal_id);
    <a href="multisig.md#0x2_multisig_cancel_borrowed">cancel_borrowed</a>(wallet, proposal, ctx);
    <a href="borrow.md#0x2_borrow_save">borrow::save</a>(proposal);
}
</code></pre>



</details>

<a name="0x2_multisig_execute_borrowed"></a>

## Function `execute_borrowed`

Borrowed-ref variant of <code>execute_transaction</code> for entry calls.
Same checks and effects; tombstones instead of deleting.


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_execute_borrowed">execute_borrowed</a>&lt;T&gt;(wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, proposal: &<b>mut</b> <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_execute_borrowed">execute_borrowed</a>&lt;T&gt;(
    wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;, proposal: &<b>mut</b> <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>, ctx: &<b>mut</b> TxContext
) {
    <b>let</b> sender = <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);
    <a href="multisig.md#0x2_multisig_assert_bound">assert_bound</a>(wallet, proposal);
    <b>assert</b>!(<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, sender), <a href="multisig.md#0x2_multisig_E_NOT_OWNER">E_NOT_OWNER</a>);
    <b>assert</b>!(!proposal.executed, <a href="multisig.md#0x2_multisig_E_TRANSACTION_ALREADY_EXECUTED">E_TRANSACTION_ALREADY_EXECUTED</a>);
    <b>assert</b>!(!<a href="multisig.md#0x2_multisig_is_expired">is_expired</a>(proposal, ctx), <a href="multisig.md#0x2_multisig_E_PROPOSAL_EXPIRED">E_PROPOSAL_EXPIRED</a>);
    <b>assert</b>!(<a href="multisig.md#0x2_multisig_has_enough_approvals">has_enough_approvals</a>(wallet, proposal), <a href="multisig.md#0x2_multisig_E_THRESHOLD_NOT_MET">E_THRESHOLD_NOT_MET</a>);

    <b>let</b> wallet_id = <a href="multisig.md#0x2_multisig_wallet_address">wallet_address</a>(wallet);
    <b>let</b> proposal_id = <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&<a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&proposal.id));

    <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_TRANSFER">TX_TYPE_TRANSFER</a>) {
        <b>let</b> amount = proposal.amount;
        <b>assert</b>!(amount &gt; 0, <a href="multisig.md#0x2_multisig_E_ZERO_AMOUNT">E_ZERO_AMOUNT</a>);
        <b>assert</b>!(<a href="coin.md#0x2_coin_value">coin::value</a>(&wallet.funds) &gt;= amount, <a href="multisig.md#0x2_multisig_E_INSUFFICIENT_BALANCE">E_INSUFFICIENT_BALANCE</a>);
        <b>let</b> out = <a href="coin.md#0x2_coin_split">coin::split</a>(&<b>mut</b> wallet.funds, amount, ctx);
        <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(out, proposal.target_address);
    } <b>else</b> <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_ADD_OWNER">TX_TYPE_ADD_OWNER</a>) {
        <b>let</b> new_owner = proposal.target_address;
        <b>assert</b>!(!<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, new_owner), <a href="multisig.md#0x2_multisig_E_ALREADY_OWNER">E_ALREADY_OWNER</a>);
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> wallet.owners, new_owner);
        <a href="event.md#0x2_event_emit">event::emit</a>(<a href="multisig.md#0x2_multisig_OwnerChangedEvent">OwnerChangedEvent</a> { wallet_id, action: 0, owner: new_owner });
    } <b>else</b> <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_REMOVE_OWNER">TX_TYPE_REMOVE_OWNER</a>) {
        <b>let</b> doomed = proposal.target_address;
        <b>assert</b>!(<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, doomed), <a href="multisig.md#0x2_multisig_E_OWNER_NOT_FOUND">E_OWNER_NOT_FOUND</a>);
        <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&wallet.owners) &gt; 1, <a href="multisig.md#0x2_multisig_E_CANNOT_REMOVE_LAST_OWNER">E_CANNOT_REMOVE_LAST_OWNER</a>);
        <a href="multisig.md#0x2_multisig_remove_owner_addr">remove_owner_addr</a>(&<b>mut</b> wallet.owners, doomed);
        <b>assert</b>!(<a href="multisig.md#0x2_multisig_owner_count">owner_count</a>(wallet) &gt;= wallet.threshold, <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
        <a href="event.md#0x2_event_emit">event::emit</a>(<a href="multisig.md#0x2_multisig_OwnerChangedEvent">OwnerChangedEvent</a> { wallet_id, action: 1, owner: doomed });
    } <b>else</b> <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_CHANGE_THRESHOLD">TX_TYPE_CHANGE_THRESHOLD</a>) {
        <b>let</b> new_threshold = <a href="multisig.md#0x2_multisig_decode_threshold">decode_threshold</a>(&proposal.payload);
        <b>assert</b>!(new_threshold &gt; 0, <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
        <b>assert</b>!(new_threshold &lt;= <a href="multisig.md#0x2_multisig_owner_count">owner_count</a>(wallet), <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
        <b>let</b> <b>old</b> = wallet.threshold;
        wallet.threshold = new_threshold;
        <a href="event.md#0x2_event_emit">event::emit</a>(
            <a href="multisig.md#0x2_multisig_ThresholdChangedEvent">ThresholdChangedEvent</a> { wallet_id, old_threshold: <b>old</b>, new_threshold }
        );
    } <b>else</b> {
        <b>abort</b> <a href="multisig.md#0x2_multisig_E_INVALID_TRANSACTION_TYPE">E_INVALID_TRANSACTION_TYPE</a>
    };

    wallet.transaction_count = wallet.transaction_count + 1;
    proposal.executed = <b>true</b>;
    <a href="event.md#0x2_event_emit">event::emit</a>(
        <a href="multisig.md#0x2_multisig_TransactionExecutedEvent">TransactionExecutedEvent</a> {
            wallet_id,
            transaction_id: proposal_id,
            executor: sender
        }
    );
}
</code></pre>



</details>

<a name="0x2_multisig_cancel_borrowed"></a>

## Function `cancel_borrowed`

Borrowed-ref variant of <code>cancel_proposal</code> for entry calls.


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_cancel_borrowed">cancel_borrowed</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, proposal: &<b>mut</b> <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>, ctx: &<a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_cancel_borrowed">cancel_borrowed</a>&lt;T&gt;(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;, proposal: &<b>mut</b> <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>, ctx: &TxContext
) {
    <b>let</b> sender = <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);
    <a href="multisig.md#0x2_multisig_assert_bound">assert_bound</a>(wallet, proposal);
    <b>assert</b>!(sender == proposal.proposer, <a href="multisig.md#0x2_multisig_E_NOT_PROPOSER">E_NOT_PROPOSER</a>);
    <b>assert</b>!(!proposal.executed, <a href="multisig.md#0x2_multisig_E_TRANSACTION_ALREADY_EXECUTED">E_TRANSACTION_ALREADY_EXECUTED</a>);

    proposal.executed = <b>true</b>;
    <a href="event.md#0x2_event_emit">event::emit</a>(
        <a href="multisig.md#0x2_multisig_ProposalCancelledEvent">ProposalCancelledEvent</a> {
            wallet_id: <a href="multisig.md#0x2_multisig_wallet_address">wallet_address</a>(wallet),
            transaction_id: <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&<a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&proposal.id)),
            canceller: sender
        }
    );
}
</code></pre>



</details>

<a name="0x2_multisig_is_owner"></a>

## Function `is_owner`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_is_owner">is_owner</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_is_owner">is_owner</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;, addr: <b>address</b>): bool {
    <b>let</b> (len, i) = (<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&wallet.owners), 0);
    <b>while</b> (i &lt; len) {
        <b>if</b> (*<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&wallet.owners, i) == addr) {
            <b>return</b> <b>true</b>
        };
        i = i + 1;
    };
    <b>false</b>
}
</code></pre>



</details>

<a name="0x2_multisig_owner_count"></a>

## Function `owner_count`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_owner_count">owner_count</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_owner_count">owner_count</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;): u64 {
    (<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&wallet.owners) <b>as</b> u64)
}
</code></pre>



</details>

<a name="0x2_multisig_get_threshold"></a>

## Function `get_threshold`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_threshold">get_threshold</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_threshold">get_threshold</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;): u64 {
    wallet.threshold
}
</code></pre>



</details>

<a name="0x2_multisig_get_transaction_count"></a>

## Function `get_transaction_count`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_transaction_count">get_transaction_count</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_transaction_count">get_transaction_count</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;): u64 {
    wallet.transaction_count
}
</code></pre>



</details>

<a name="0x2_multisig_has_enough_approvals"></a>

## Function `has_enough_approvals`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_has_enough_approvals">has_enough_approvals</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_has_enough_approvals">has_enough_approvals</a>&lt;T&gt;(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;, proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>
): bool {
    (<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&proposal.approvers) <b>as</b> u64) &gt;= wallet.threshold
}
</code></pre>



</details>

<a name="0x2_multisig_get_approval_count"></a>

## Function `get_approval_count`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_approval_count">get_approval_count</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_approval_count">get_approval_count</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>): u64 {
    (<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&proposal.approvers) <b>as</b> u64)
}
</code></pre>



</details>

<a name="0x2_multisig_is_executed"></a>

## Function `is_executed`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_is_executed">is_executed</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_is_executed">is_executed</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>): bool {
    proposal.executed
}
</code></pre>



</details>

<a name="0x2_multisig_get_proposer"></a>

## Function `get_proposer`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_proposer">get_proposer</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_proposer">get_proposer</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>): <b>address</b> {
    proposal.proposer
}
</code></pre>



</details>

<a name="0x2_multisig_get_tx_type"></a>

## Function `get_tx_type`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_tx_type">get_tx_type</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_tx_type">get_tx_type</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>): u8 {
    proposal.tx_type
}
</code></pre>



</details>

<a name="0x2_multisig_get_target_address"></a>

## Function `get_target_address`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_target_address">get_target_address</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_target_address">get_target_address</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>): <b>address</b> {
    proposal.target_address
}
</code></pre>



</details>

<a name="0x2_multisig_get_amount"></a>

## Function `get_amount`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_amount">get_amount</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_amount">get_amount</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>): u64 {
    proposal.amount
}
</code></pre>



</details>

<a name="0x2_multisig_get_description"></a>

## Function `get_description`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_description">get_description</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>): &<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_description">get_description</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>): &<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a> {
    &proposal.description
}
</code></pre>



</details>

<a name="0x2_multisig_get_expires_at_ms"></a>

## Function `get_expires_at_ms`



<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_expires_at_ms">get_expires_at_ms</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_expires_at_ms">get_expires_at_ms</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>): u64 {
    proposal.expires_at_ms
}
</code></pre>



</details>

<a name="0x2_multisig_is_expired"></a>

## Function `is_expired`

True when the proposal can no longer be approved or executed.


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_is_expired">is_expired</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>, ctx: &<a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_is_expired">is_expired</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>, ctx: &TxContext): bool {
    proposal.expires_at_ms != <a href="multisig.md#0x2_multisig_NO_EXPIRY">NO_EXPIRY</a>
        && <a href="tx_context.md#0x2_tx_context_epoch_timestamp_ms">tx_context::epoch_timestamp_ms</a>(ctx) &gt;= proposal.expires_at_ms
}
</code></pre>



</details>

<a name="0x2_multisig_wallet_address"></a>

## Function `wallet_address`



<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_wallet_address">wallet_address</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_wallet_address">wallet_address</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;): <b>address</b> {
    <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&<a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&wallet.id))
}
</code></pre>



</details>

<a name="0x2_multisig_assert_bound"></a>

## Function `assert_bound`

A proposal approved for one wallet must never execute against another.


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_assert_bound">assert_bound</a>&lt;T&gt;(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>&lt;T&gt;, proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_assert_bound">assert_bound</a>&lt;T&gt;(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>&lt;T&gt;, proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>
) {
    <b>assert</b>!(
        proposal.wallet_id == <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&wallet.id),
        <a href="multisig.md#0x2_multisig_E_BINDING_MISMATCH">E_BINDING_MISMATCH</a>
    );
}
</code></pre>



</details>

<a name="0x2_multisig_check_duplicate_owners"></a>

## Function `check_duplicate_owners`



<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_check_duplicate_owners">check_duplicate_owners</a>(owners: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_check_duplicate_owners">check_duplicate_owners</a>(owners: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;) {
    <b>let</b> len = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(owners);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; len) {
        <b>let</b> addr_i = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(owners, i);
        <b>let</b> j = i + 1;
        <b>while</b> (j &lt; len) {
            <b>assert</b>!(*addr_i != *<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(owners, j), <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
            j = j + 1;
        };
        i = i + 1;
    };
}
</code></pre>



</details>

<a name="0x2_multisig_has_approved"></a>

## Function `has_approved`



<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_has_approved">has_approved</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>, addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_has_approved">has_approved</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>, addr: <b>address</b>): bool {
    <b>let</b> (len, i) = (<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&proposal.approvers), 0);
    <b>while</b> (i &lt; len) {
        <b>if</b> (*<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&proposal.approvers, i) == addr) {
            <b>return</b> <b>true</b>
        };
        i = i + 1;
    };
    <b>false</b>
}
</code></pre>



</details>

<a name="0x2_multisig_remove_owner_addr"></a>

## Function `remove_owner_addr`



<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_remove_owner_addr">remove_owner_addr</a>(owners: &<b>mut</b> <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, doomed: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_remove_owner_addr">remove_owner_addr</a>(owners: &<b>mut</b> <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, doomed: <b>address</b>) {
    <b>let</b> (len, i) = (<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(owners), 0);
    <b>while</b> (i &lt; len) {
        <b>if</b> (*<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(owners, i) == doomed) {
            <a href="dependencies/move-stdlib/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(owners, i);
            <b>return</b>
        };
        i = i + 1;
    };
    <b>abort</b> <a href="multisig.md#0x2_multisig_E_OWNER_NOT_FOUND">E_OWNER_NOT_FOUND</a>
}
</code></pre>



</details>

<a name="0x2_multisig_decode_threshold"></a>

## Function `decode_threshold`

Strict BCS u64 decode: exactly 8 bytes, no trailing data.


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_decode_threshold">decode_threshold</a>(payload: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_decode_threshold">decode_threshold</a>(payload: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64 {
    <b>assert</b>!(<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(payload) == 8, <a href="multisig.md#0x2_multisig_E_INVALID_PAYLOAD">E_INVALID_PAYLOAD</a>);
    <b>let</b> reader = bcs::new(*payload);
    <b>let</b> v = bcs::peel_u64(&<b>mut</b> reader);
    <b>assert</b>!(
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&bcs::into_remainder_bytes(reader)) == 0,
        <a href="multisig.md#0x2_multisig_E_INVALID_PAYLOAD">E_INVALID_PAYLOAD</a>
    );
    v
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
