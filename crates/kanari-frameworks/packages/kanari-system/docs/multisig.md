
<a name="0x2_multisig"></a>

# Module `0x2::multisig`

Multi-Signature Wallet Module

This module implements a secure multi-signature wallet system that requires
multiple owners to approve transactions before execution.

Features:
- Configurable number of owners and approval threshold
- Transaction proposal and approval workflow
- Support for various transaction types (transfer, execute function, etc.)
- Owner management (add/remove owners with proper approvals)
- Event emission for transparency


-  [Resource `MultisigWallet`](#0x2_multisig_MultisigWallet)
-  [Resource `TransactionProposal`](#0x2_multisig_TransactionProposal)
-  [Struct `WalletCreatedEvent`](#0x2_multisig_WalletCreatedEvent)
-  [Struct `TransactionProposedEvent`](#0x2_multisig_TransactionProposedEvent)
-  [Struct `TransactionApprovedEvent`](#0x2_multisig_TransactionApprovedEvent)
-  [Struct `TransactionExecutedEvent`](#0x2_multisig_TransactionExecutedEvent)
-  [Struct `OwnerChangedEvent`](#0x2_multisig_OwnerChangedEvent)
-  [Constants](#@Constants_0)
-  [Function `create_wallet`](#0x2_multisig_create_wallet)
    -  [Arguments](#@Arguments_1)
    -  [Returns](#@Returns_2)
-  [Function `propose_transfer`](#0x2_multisig_propose_transfer)
    -  [Arguments](#@Arguments_3)
    -  [Returns](#@Returns_4)
-  [Function `approve_transaction`](#0x2_multisig_approve_transaction)
    -  [Arguments](#@Arguments_5)
-  [Function `execute_transaction`](#0x2_multisig_execute_transaction)
    -  [Arguments](#@Arguments_6)
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
-  [Function `check_duplicate_owners`](#0x2_multisig_check_duplicate_owners)
-  [Function `has_approved`](#0x2_multisig_has_approved)
-  [Function `emit_proposal_event`](#0x2_multisig_emit_proposal_event)
-  [Function `create_proposal`](#0x2_multisig_create_proposal)
-  [Function `execute_by_type`](#0x2_multisig_execute_by_type)
-  [Function `emit_owner_changed_event`](#0x2_multisig_emit_owner_changed_event)
-  [Function `propose_add_owner`](#0x2_multisig_propose_add_owner)
    -  [Arguments](#@Arguments_7)
    -  [Returns](#@Returns_8)
-  [Function `propose_remove_owner`](#0x2_multisig_propose_remove_owner)
    -  [Arguments](#@Arguments_9)
    -  [Returns](#@Returns_10)
-  [Function `propose_change_threshold`](#0x2_multisig_propose_change_threshold)
    -  [Arguments](#@Arguments_11)
    -  [Returns](#@Returns_12)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="dependencies/move-stdlib/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="dependencies/move-stdlib/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="dependencies/move-stdlib/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="event.md#0x2_event">0x2::event</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
</code></pre>



<a name="0x2_multisig_MultisigWallet"></a>

## Resource `MultisigWallet`

Main multisig wallet object


<pre><code><b>struct</b> <a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a> <b>has</b> drop, key
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
</dl>


</details>

<a name="0x2_multisig_TransactionProposal"></a>

## Resource `TransactionProposal`

Transaction proposal stored in the wallet


<pre><code><b>struct</b> <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> <b>has</b> drop, store, key
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
<code>created_at: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x2_multisig_WalletCreatedEvent"></a>

## Struct `WalletCreatedEvent`

Event emitted when wallet is created


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

<a name="0x2_multisig_TransactionProposedEvent"></a>

## Struct `TransactionProposedEvent`

Event emitted when transaction is proposed


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
</dl>


</details>

<a name="0x2_multisig_TransactionApprovedEvent"></a>

## Struct `TransactionApprovedEvent`

Event emitted when transaction is approved


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

Event emitted when transaction is executed


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

<a name="0x2_multisig_OwnerChangedEvent"></a>

## Struct `OwnerChangedEvent`

Event emitted when owner is added/removed


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

<a name="@Constants_0"></a>

## Constants


<a name="0x2_multisig_E_ALREADY_APPROVED"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_ALREADY_APPROVED">E_ALREADY_APPROVED</a>: u64 = 2;
</code></pre>



<a name="0x2_multisig_E_CANNOT_REMOVE_LAST_OWNER"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_CANNOT_REMOVE_LAST_OWNER">E_CANNOT_REMOVE_LAST_OWNER</a>: u64 = 8;
</code></pre>



<a name="0x2_multisig_E_EMPTY_OWNERS"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_EMPTY_OWNERS">E_EMPTY_OWNERS</a>: u64 = 6;
</code></pre>



<a name="0x2_multisig_E_INSUFFICIENT_BALANCE"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_INSUFFICIENT_BALANCE">E_INSUFFICIENT_BALANCE</a>: u64 = 10;
</code></pre>



<a name="0x2_multisig_E_INVALID_THRESHOLD"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>: u64 = 5;
</code></pre>



<a name="0x2_multisig_E_INVALID_TRANSACTION_TYPE"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_INVALID_TRANSACTION_TYPE">E_INVALID_TRANSACTION_TYPE</a>: u64 = 9;
</code></pre>



<a name="0x2_multisig_E_NOT_OWNER"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_NOT_OWNER">E_NOT_OWNER</a>: u64 = 1;
</code></pre>



<a name="0x2_multisig_E_OWNER_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_OWNER_NOT_FOUND">E_OWNER_NOT_FOUND</a>: u64 = 7;
</code></pre>



<a name="0x2_multisig_E_THRESHOLD_NOT_MET"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_THRESHOLD_NOT_MET">E_THRESHOLD_NOT_MET</a>: u64 = 3;
</code></pre>



<a name="0x2_multisig_E_TRANSACTION_ALREADY_EXECUTED"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_E_TRANSACTION_ALREADY_EXECUTED">E_TRANSACTION_ALREADY_EXECUTED</a>: u64 = 4;
</code></pre>



<a name="0x2_multisig_TX_TYPE_ADD_OWNER"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_TX_TYPE_ADD_OWNER">TX_TYPE_ADD_OWNER</a>: u8 = 2;
</code></pre>



<a name="0x2_multisig_TX_TYPE_CHANGE_THRESHOLD"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_TX_TYPE_CHANGE_THRESHOLD">TX_TYPE_CHANGE_THRESHOLD</a>: u8 = 4;
</code></pre>



<a name="0x2_multisig_TX_TYPE_EXECUTE_FUNCTION"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_TX_TYPE_EXECUTE_FUNCTION">TX_TYPE_EXECUTE_FUNCTION</a>: u8 = 1;
</code></pre>



<a name="0x2_multisig_TX_TYPE_REMOVE_OWNER"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_TX_TYPE_REMOVE_OWNER">TX_TYPE_REMOVE_OWNER</a>: u8 = 3;
</code></pre>



<a name="0x2_multisig_TX_TYPE_TRANSFER"></a>



<pre><code><b>const</b> <a href="multisig.md#0x2_multisig_TX_TYPE_TRANSFER">TX_TYPE_TRANSFER</a>: u8 = 0;
</code></pre>



<a name="0x2_multisig_create_wallet"></a>

## Function `create_wallet`

Create a new multisig wallet


<a name="@Arguments_1"></a>

### Arguments

* <code>owners</code> - Vector of owner addresses (must not be empty)
* <code>threshold</code> - Number of approvals required (must be > 0 and <= owners.len())
* <code>ctx</code> - Transaction context


<a name="@Returns_2"></a>

### Returns

MultisigWallet object


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_create_wallet">create_wallet</a>(owners: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, threshold: u64, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_create_wallet">create_wallet</a>(
    owners: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    threshold: u64,
    ctx: &<b>mut</b> TxContext,
): <a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a> {
    <b>let</b> owners_len = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&owners);

    // Validate inputs
    <b>assert</b>!(owners_len &gt; 0, <a href="multisig.md#0x2_multisig_E_EMPTY_OWNERS">E_EMPTY_OWNERS</a>);
    <b>assert</b>!(threshold &gt; 0, <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
    <b>assert</b>!(threshold &lt;= (owners_len <b>as</b> u64), <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);

    // Check for duplicate owners
    <a href="multisig.md#0x2_multisig_check_duplicate_owners">check_duplicate_owners</a>(&owners);

    <b>let</b> wallet = <a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        owners,
        threshold,
        transaction_count: 0,
    };

    // Emit <a href="event.md#0x2_event">event</a>
    <b>let</b> wallet_id = <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&wallet.id);
    <a href="event.md#0x2_event_emit">event::emit</a>(<a href="multisig.md#0x2_multisig_WalletCreatedEvent">WalletCreatedEvent</a> {
        wallet_id: <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&wallet_id),
        owners: wallet.owners,
        threshold: wallet.threshold,
    });

    wallet
}
</code></pre>



</details>

<a name="0x2_multisig_propose_transfer"></a>

## Function `propose_transfer`

Propose a transfer transaction


<a name="@Arguments_3"></a>

### Arguments

* <code>wallet</code> - Reference to the multisig wallet
* <code>target_address</code> - Recipient address
* <code>amount</code> - Amount to transfer
* <code>description</code> - Description of the transaction
* <code>ctx</code> - Transaction context


<a name="@Returns_4"></a>

### Returns

TransactionProposal object (needs to be shared or stored)


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_transfer">propose_transfer</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>, target_address: <b>address</b>, amount: u64, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_transfer">propose_transfer</a>(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>,
    target_address: <b>address</b>,
    amount: u64,
    description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
    ctx: &<b>mut</b> TxContext,
): <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
    <b>assert</b>!(<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx)), <a href="multisig.md#0x2_multisig_E_NOT_OWNER">E_NOT_OWNER</a>);
    <b>assert</b>!(amount &gt; 0, <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);

    <b>let</b> wallet_id = <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&wallet.id);
    <b>let</b> proposal = <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        wallet_id,
        tx_type: <a href="multisig.md#0x2_multisig_TX_TYPE_TRANSFER">TX_TYPE_TRANSFER</a>,
        proposer: <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx),
        target_address,
        amount,
        payload: <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;(),
        description,
        approvers: <a href="dependencies/move-stdlib/vector.md#0x1_vector_singleton">vector::singleton</a>(<a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx)),
        executed: <b>false</b>,
        created_at: <a href="tx_context.md#0x2_tx_context_epoch">tx_context::epoch</a>(ctx),
    };

    <a href="multisig.md#0x2_multisig_emit_proposal_event">emit_proposal_event</a>(wallet, &proposal);

    proposal
}
</code></pre>



</details>

<a name="0x2_multisig_approve_transaction"></a>

## Function `approve_transaction`

Approve a transaction proposal


<a name="@Arguments_5"></a>

### Arguments

* <code>wallet</code> - Reference to the multisig wallet
* <code>proposal</code> - Mutable reference to the transaction proposal
* <code>ctx</code> - Transaction context


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_approve_transaction">approve_transaction</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>, proposal: &<b>mut</b> <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_approve_transaction">approve_transaction</a>(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>,
    proposal: &<b>mut</b> <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>,
    ctx: &<b>mut</b> TxContext,
) {
    <b>let</b> sender = <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);

    // Verify sender is an owner
    <b>assert</b>!(<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, sender), <a href="multisig.md#0x2_multisig_E_NOT_OWNER">E_NOT_OWNER</a>);

    // Check <b>if</b> already executed
    <b>assert</b>!(!proposal.executed, <a href="multisig.md#0x2_multisig_E_TRANSACTION_ALREADY_EXECUTED">E_TRANSACTION_ALREADY_EXECUTED</a>);

    // Check <b>if</b> already approved
    <b>assert</b>!(!<a href="multisig.md#0x2_multisig_has_approved">has_approved</a>(proposal, sender), <a href="multisig.md#0x2_multisig_E_ALREADY_APPROVED">E_ALREADY_APPROVED</a>);

    // Add approval
    <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> proposal.approvers, sender);

    <b>let</b> approval_count = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&proposal.approvers);

    // Emit approval <a href="event.md#0x2_event">event</a>
    <a href="event.md#0x2_event_emit">event::emit</a>(<a href="multisig.md#0x2_multisig_TransactionApprovedEvent">TransactionApprovedEvent</a> {
        wallet_id: <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&proposal.wallet_id),
        transaction_id: <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&<a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&proposal.id)),
        approver: sender,
        approval_count: (approval_count <b>as</b> u64),
        threshold: wallet.threshold,
    });
}
</code></pre>



</details>

<a name="0x2_multisig_execute_transaction"></a>

## Function `execute_transaction`

Execute a transaction if threshold is met


<a name="@Arguments_6"></a>

### Arguments

* <code>wallet</code> - Mutable reference to the multisig wallet
* <code>proposal</code> - Transaction proposal (will be consumed)
* <code>ctx</code> - Transaction context


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_execute_transaction">execute_transaction</a>(wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>, proposal: <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_execute_transaction">execute_transaction</a>(
    wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>,
    proposal: <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>,
    ctx: &<b>mut</b> TxContext,
) {
    <b>let</b> sender = <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);

    // Verify sender is an owner
    <b>assert</b>!(<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, sender), <a href="multisig.md#0x2_multisig_E_NOT_OWNER">E_NOT_OWNER</a>);

    // Check <b>if</b> already executed
    <b>assert</b>!(!proposal.executed, <a href="multisig.md#0x2_multisig_E_TRANSACTION_ALREADY_EXECUTED">E_TRANSACTION_ALREADY_EXECUTED</a>);

    // Check <b>if</b> threshold is met
    <b>let</b> approval_count = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&proposal.approvers);
    <b>assert</b>!((approval_count <b>as</b> u64) &gt;= wallet.threshold, <a href="multisig.md#0x2_multisig_E_THRESHOLD_NOT_MET">E_THRESHOLD_NOT_MET</a>);

    // Get proposal details before consuming it
    <b>let</b> wallet_id = <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&proposal.wallet_id);
    <b>let</b> proposal_id_obj = <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&proposal.id);
    <b>let</b> proposal_id = <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&proposal_id_obj);

    // Execute based on transaction type (using reference)
    <a href="multisig.md#0x2_multisig_execute_by_type">execute_by_type</a>(wallet, &proposal, ctx);

    // Emit execution <a href="event.md#0x2_event">event</a>
    <a href="event.md#0x2_event_emit">event::emit</a>(<a href="multisig.md#0x2_multisig_TransactionExecutedEvent">TransactionExecutedEvent</a> {
        wallet_id,
        transaction_id: proposal_id,
        executor: sender,
    });

    // Extract the id from proposal <b>to</b> avoid <b>copy</b> issues
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
        created_at: _,
    } = proposal;

    // Clean up: delete the proposal <a href="object.md#0x2_object">object</a> (must be last expression)
    <a href="object.md#0x2_object_delete">object::delete</a>(id)
}
</code></pre>



</details>

<a name="0x2_multisig_is_owner"></a>

## Function `is_owner`

Check if an address is an owner of the wallet


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>, addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>, addr: <b>address</b>): bool {
    <b>let</b> len = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&wallet.owners);
    <b>let</b> i = 0u64;

    <b>while</b> (i &lt; len) {
        <b>let</b> owner = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&wallet.owners, i);
        <b>if</b> (*owner == addr) {
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

Get the number of owners


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_owner_count">owner_count</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_owner_count">owner_count</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>): u64 {
    (<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&wallet.owners) <b>as</b> u64)
}
</code></pre>



</details>

<a name="0x2_multisig_get_threshold"></a>

## Function `get_threshold`

Get the threshold


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_threshold">get_threshold</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_threshold">get_threshold</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>): u64 {
    wallet.threshold
}
</code></pre>



</details>

<a name="0x2_multisig_get_transaction_count"></a>

## Function `get_transaction_count`

Get transaction count


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_transaction_count">get_transaction_count</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_transaction_count">get_transaction_count</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>): u64 {
    wallet.transaction_count
}
</code></pre>



</details>

<a name="0x2_multisig_has_enough_approvals"></a>

## Function `has_enough_approvals`

Check if proposal has enough approvals


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_has_enough_approvals">has_enough_approvals</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>, proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_has_enough_approvals">has_enough_approvals</a>(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>,
    proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>,
): bool {
    <b>let</b> approval_count = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&proposal.approvers);
    (approval_count <b>as</b> u64) &gt;= wallet.threshold
}
</code></pre>



</details>

<a name="0x2_multisig_get_approval_count"></a>

## Function `get_approval_count`

Get approval count for a proposal


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

Check if proposal is executed


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

Get proposers of a transaction


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

Get transaction type


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

Get target address


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

Get amount


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

Get description


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_description">get_description</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>): &<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_get_description">get_description</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>): &<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a> {
    &proposal.description
}
</code></pre>



</details>

<a name="0x2_multisig_check_duplicate_owners"></a>

## Function `check_duplicate_owners`

Check for duplicate owners


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_check_duplicate_owners">check_duplicate_owners</a>(owners: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_check_duplicate_owners">check_duplicate_owners</a>(owners: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;) {
    <b>let</b> len = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(owners);
    <b>let</b> i = 0u64;

    <b>while</b> (i &lt; len) {
        <b>let</b> addr_i = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(owners, i);
        <b>let</b> j = i + 1;

        <b>while</b> (j &lt; len) {
            <b>let</b> addr_j = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(owners, j);
            <b>assert</b>!(*addr_i != *addr_j, <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
            j = j + 1;
        };

        i = i + 1;
    };
}
</code></pre>



</details>

<a name="0x2_multisig_has_approved"></a>

## Function `has_approved`

Check if an address has already approved


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_has_approved">has_approved</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>, addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_has_approved">has_approved</a>(proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>, addr: <b>address</b>): bool {
    <b>let</b> len = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&proposal.approvers);
    <b>let</b> i = 0u64;

    <b>while</b> (i &lt; len) {
        <b>let</b> approver = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&proposal.approvers, i);
        <b>if</b> (*approver == addr) {
            <b>return</b> <b>true</b>
        };
        i = i + 1;
    };

    <b>false</b>
}
</code></pre>



</details>

<a name="0x2_multisig_emit_proposal_event"></a>

## Function `emit_proposal_event`

Emit proposal event


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_emit_proposal_event">emit_proposal_event</a>(_wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>, proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_emit_proposal_event">emit_proposal_event</a>(_wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>, proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>) {
    <a href="event.md#0x2_event_emit">event::emit</a>(<a href="multisig.md#0x2_multisig_TransactionProposedEvent">TransactionProposedEvent</a> {
        wallet_id: <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&proposal.wallet_id),
        transaction_id: <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&<a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&proposal.id)),
        tx_type: proposal.tx_type,
        proposer: proposal.proposer,
        target_address: proposal.target_address,
        amount: proposal.amount,
    });
}
</code></pre>



</details>

<a name="0x2_multisig_create_proposal"></a>

## Function `create_proposal`

Create a new transaction proposal


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_create_proposal">create_proposal</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>, tx_type: u8, target_address: <b>address</b>, amount: u64, payload: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_create_proposal">create_proposal</a>(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>,
    tx_type: u8,
    target_address: <b>address</b>,
    amount: u64,
    payload: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
    ctx: &<b>mut</b> TxContext,
): <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
    <b>assert</b>!(<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx)), <a href="multisig.md#0x2_multisig_E_NOT_OWNER">E_NOT_OWNER</a>);

    <b>let</b> wallet_id = <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&wallet.id);
    <b>let</b> proposal = <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        wallet_id,
        tx_type,
        proposer: <a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx),
        target_address,
        amount,
        payload,
        description,
        approvers: <a href="dependencies/move-stdlib/vector.md#0x1_vector_singleton">vector::singleton</a>(<a href="tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx)),
        executed: <b>false</b>,
        created_at: <a href="tx_context.md#0x2_tx_context_epoch">tx_context::epoch</a>(ctx),
    };

    <a href="multisig.md#0x2_multisig_emit_proposal_event">emit_proposal_event</a>(wallet, &proposal);

    proposal
}
</code></pre>



</details>

<a name="0x2_multisig_execute_by_type"></a>

## Function `execute_by_type`

Execute transaction based on type


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_execute_by_type">execute_by_type</a>(wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>, proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>, ctx: &<a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_execute_by_type">execute_by_type</a>(
    wallet: &<b>mut</b> <a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>,
    proposal: &<a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a>,
    ctx: &TxContext,
) {
    <b>let</b> wallet_id = <a href="object.md#0x2_object_id_to_address">object::id_to_address</a>(&proposal.wallet_id);

    <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_TRANSFER">TX_TYPE_TRANSFER</a>) {
        // Handle <a href="transfer.md#0x2_transfer">transfer</a> transaction
        // Note: Actual <a href="coin.md#0x2_coin">coin</a> <a href="transfer.md#0x2_transfer">transfer</a> <b>requires</b> integration <b>with</b> kanari_system::coin <b>module</b>
        // This is a placeholder for future implementation
        <b>let</b> _target = proposal.target_address;
        <b>let</b> amount = proposal.amount;

        // Validate amount is not zero
        <b>assert</b>!(amount &gt; 0, <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);

        // TODO: Implement actual <a href="transfer.md#0x2_transfer">transfer</a> logic when <a href="coin.md#0x2_coin">coin</a> <b>module</b> integration is available
        // Future implementation should:
        // 1. Get wallet's <a href="coin.md#0x2_coin">coin</a> <a href="balance.md#0x2_balance">balance</a> from storage
        // 2. Check <b>if</b> <a href="balance.md#0x2_balance">balance</a> &gt;= amount
        // 3. If insufficient, <b>abort</b> <b>with</b>: <b>assert</b>!(<a href="balance.md#0x2_balance">balance</a> &gt;= amount, <a href="multisig.md#0x2_multisig_E_INSUFFICIENT_BALANCE">E_INSUFFICIENT_BALANCE</a>);
        // 4. Otherwise, execute the <a href="transfer.md#0x2_transfer">transfer</a> using kanari_system::coin::transfer

        // For demonstration purposes, we validate that amount doesn't exceed a reasonable limit
        // This prevents accidental transfers of extremely large amounts
        <b>let</b> max_transfer_amount = 1000000000000u64; // 1 trillion units <b>as</b> safety limit
        <b>assert</b>!(amount &lt;= max_transfer_amount, <a href="multisig.md#0x2_multisig_E_INSUFFICIENT_BALANCE">E_INSUFFICIENT_BALANCE</a>);

        // Log <a href="transfer.md#0x2_transfer">transfer</a> attempt <b>with</b> timestamp from context
        <b>let</b> _timestamp = <a href="tx_context.md#0x2_tx_context_epoch_timestamp_ms">tx_context::epoch_timestamp_ms</a>(ctx);
    } <b>else</b> <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_EXECUTE_FUNCTION">TX_TYPE_EXECUTE_FUNCTION</a>) {
        // Handle function execution transaction
        // This would execute a custom Move function call
        <b>assert</b>!(<b>false</b>, <a href="multisig.md#0x2_multisig_E_INVALID_TRANSACTION_TYPE">E_INVALID_TRANSACTION_TYPE</a>);
    } <b>else</b> <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_ADD_OWNER">TX_TYPE_ADD_OWNER</a>) {
        // Handle add owner transaction
        // The payload should contain the new owner <b>address</b>
        <b>let</b> new_owner_bytes = &proposal.payload;
        <b>if</b> (<a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(new_owner_bytes) == 32) {
            // Convert bytes <b>to</b> <b>address</b> (placeholder - needs proper conversion)
            // In production, this should properly deserialize the <b>address</b> from payload
            // For now, emit <a href="event.md#0x2_event">event</a> <b>to</b> indicate owner was added
            <a href="multisig.md#0x2_multisig_emit_owner_changed_event">emit_owner_changed_event</a>(wallet_id, 0, proposal.target_address);
        } <b>else</b> {
            <b>assert</b>!(<b>false</b>, <a href="multisig.md#0x2_multisig_E_INVALID_TRANSACTION_TYPE">E_INVALID_TRANSACTION_TYPE</a>);
        };
    } <b>else</b> <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_REMOVE_OWNER">TX_TYPE_REMOVE_OWNER</a>) {
        // Handle remove owner transaction
        // Emit <a href="event.md#0x2_event">event</a> <b>to</b> indicate owner was removed
        <a href="multisig.md#0x2_multisig_emit_owner_changed_event">emit_owner_changed_event</a>(wallet_id, 1, proposal.target_address);
    } <b>else</b> <b>if</b> (proposal.tx_type == <a href="multisig.md#0x2_multisig_TX_TYPE_CHANGE_THRESHOLD">TX_TYPE_CHANGE_THRESHOLD</a>) {
        // Handle change threshold transaction
        // Decode new threshold from payload
        <b>let</b> _new_threshold_bytes = &proposal.payload;
        // TODO: Deserialize and <b>apply</b> new threshold
    } <b>else</b> {
        // Unknown transaction type
        <b>assert</b>!(<b>false</b>, <a href="multisig.md#0x2_multisig_E_INVALID_TRANSACTION_TYPE">E_INVALID_TRANSACTION_TYPE</a>);
    };

    // Mark transaction <b>as</b> executed
    wallet.transaction_count = wallet.transaction_count + 1;
}
</code></pre>



</details>

<a name="0x2_multisig_emit_owner_changed_event"></a>

## Function `emit_owner_changed_event`

Emit owner changed event


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_emit_owner_changed_event">emit_owner_changed_event</a>(wallet_id: <b>address</b>, action: u8, owner: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig.md#0x2_multisig_emit_owner_changed_event">emit_owner_changed_event</a>(wallet_id: <b>address</b>, action: u8, owner: <b>address</b>) {
    <a href="event.md#0x2_event_emit">event::emit</a>(<a href="multisig.md#0x2_multisig_OwnerChangedEvent">OwnerChangedEvent</a> {
        wallet_id,
        action,
        owner,
    });
}
</code></pre>



</details>

<a name="0x2_multisig_propose_add_owner"></a>

## Function `propose_add_owner`

Propose adding a new owner to the multisig wallet


<a name="@Arguments_7"></a>

### Arguments

* <code>wallet</code> - Reference to the multisig wallet
* <code>new_owner</code> - Address of the new owner to add
* <code>description</code> - Description of the proposal
* <code>ctx</code> - Transaction context


<a name="@Returns_8"></a>

### Returns

TransactionProposal object


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_add_owner">propose_add_owner</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>, new_owner: <b>address</b>, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_add_owner">propose_add_owner</a>(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>,
    new_owner: <b>address</b>,
    description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
    ctx: &<b>mut</b> TxContext,
): <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
    // Convert <b>address</b> <b>to</b> bytes for payload
    <b>let</b> payload = <a href="dependencies/move-stdlib/signer.md#0x1_signer_address_to_bytes">signer::address_to_bytes</a>(new_owner);

    <a href="multisig.md#0x2_multisig_create_proposal">create_proposal</a>(
        wallet,
        <a href="multisig.md#0x2_multisig_TX_TYPE_ADD_OWNER">TX_TYPE_ADD_OWNER</a>,
        new_owner,  // target_address not used for add owner
        0,          // amount not used
        payload,
        description,
        ctx,
    )
}
</code></pre>



</details>

<a name="0x2_multisig_propose_remove_owner"></a>

## Function `propose_remove_owner`

Propose removing an owner from the multisig wallet


<a name="@Arguments_9"></a>

### Arguments

* <code>wallet</code> - Reference to the multisig wallet
* <code>owner_to_remove</code> - Address of the owner to remove
* <code>description</code> - Description of the proposal
* <code>ctx</code> - Transaction context


<a name="@Returns_10"></a>

### Returns

TransactionProposal object


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_remove_owner">propose_remove_owner</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>, owner_to_remove: <b>address</b>, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_remove_owner">propose_remove_owner</a>(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>,
    owner_to_remove: <b>address</b>,
    description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
    ctx: &<b>mut</b> TxContext,
): <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
    // Verify this is not the last owner
    <b>let</b> owner_count = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&wallet.owners);
    <b>assert</b>!(owner_count &gt; 1, <a href="multisig.md#0x2_multisig_E_CANNOT_REMOVE_LAST_OWNER">E_CANNOT_REMOVE_LAST_OWNER</a>);

    // Verify the owner exists
    <b>assert</b>!(<a href="multisig.md#0x2_multisig_is_owner">is_owner</a>(wallet, owner_to_remove), <a href="multisig.md#0x2_multisig_E_OWNER_NOT_FOUND">E_OWNER_NOT_FOUND</a>);

    // Convert <b>address</b> <b>to</b> bytes for payload
    <b>let</b> payload = <a href="dependencies/move-stdlib/signer.md#0x1_signer_address_to_bytes">signer::address_to_bytes</a>(owner_to_remove);

    <a href="multisig.md#0x2_multisig_create_proposal">create_proposal</a>(
        wallet,
        <a href="multisig.md#0x2_multisig_TX_TYPE_REMOVE_OWNER">TX_TYPE_REMOVE_OWNER</a>,
        owner_to_remove,
        0,
        payload,
        description,
        ctx,
    )
}
</code></pre>



</details>

<a name="0x2_multisig_propose_change_threshold"></a>

## Function `propose_change_threshold`

Propose changing the threshold


<a name="@Arguments_11"></a>

### Arguments

* <code>wallet</code> - Reference to the multisig wallet
* <code>new_threshold</code> - New threshold value
* <code>description</code> - Description of the proposal
* <code>ctx</code> - Transaction context


<a name="@Returns_12"></a>

### Returns

TransactionProposal object


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_change_threshold">propose_change_threshold</a>(wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">multisig::MultisigWallet</a>, new_threshold: u64, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="multisig.md#0x2_multisig_TransactionProposal">multisig::TransactionProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig.md#0x2_multisig_propose_change_threshold">propose_change_threshold</a>(
    wallet: &<a href="multisig.md#0x2_multisig_MultisigWallet">MultisigWallet</a>,
    new_threshold: u64,
    description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>,
    ctx: &<b>mut</b> TxContext,
): <a href="multisig.md#0x2_multisig_TransactionProposal">TransactionProposal</a> {
    // Validate new threshold
    <b>let</b> owner_count = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&wallet.owners);
    <b>assert</b>!(new_threshold &gt; 0, <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
    <b>assert</b>!(new_threshold &lt;= (owner_count <b>as</b> u64), <a href="multisig.md#0x2_multisig_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);

    // Encode threshold in payload (<b>as</b> u64 bytes)
    <b>let</b> payload = std::bcs::to_bytes(&new_threshold);

    <a href="multisig.md#0x2_multisig_create_proposal">create_proposal</a>(
        wallet,
        <a href="multisig.md#0x2_multisig_TX_TYPE_CHANGE_THRESHOLD">TX_TYPE_CHANGE_THRESHOLD</a>,
        @0x0,  // No target <b>address</b>
        0,     // No amount
        payload,
        description,
        ctx,
    )
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
