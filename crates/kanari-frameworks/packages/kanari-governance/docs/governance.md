
<a name="0x3_governance"></a>

# Module `0x3::governance`

Kanari Governance Module

On-chain governance with real, staked voting:

- Creating a proposal requires an escrowed token deposit
(<code><a href="governance.md#0x3_governance_MIN_PROPOSAL_BALANCE">MIN_PROPOSAL_BALANCE</a></code>), returned to the creator after execution.
- Voting stakes tokens: the <code>Coin</code> passed to <code>cast_vote</code> is escrowed in the
proposal and becomes the voter's voting weight, so a vote is always
backed by locked value.
- One stake per voter per proposal: a voter may top up the same side, but
switching sides aborts.
- Stakes (and the deposit) are withdrawable once the voting window closes,
so participation costs time, not principal.
- Execution requires quorum: the total staked amount must reach
<code>total_supply * quorum_threshold</code> in basis points.


-  [Struct `ProposalCreated`](#0x3_governance_ProposalCreated)
-  [Struct `VoteCast`](#0x3_governance_VoteCast)
-  [Struct `ProposalExecuted`](#0x3_governance_ProposalExecuted)
-  [Resource `GovernanceState`](#0x3_governance_GovernanceState)
-  [Struct `VoteStake`](#0x3_governance_VoteStake)
-  [Resource `Proposal`](#0x3_governance_Proposal)
-  [Resource `GovernanceCap`](#0x3_governance_GovernanceCap)
-  [Constants](#@Constants_0)
-  [Function `init`](#0x3_governance_init)
-  [Function `create_proposal`](#0x3_governance_create_proposal)
-  [Function `cast_vote`](#0x3_governance_cast_vote)
-  [Function `withdraw_stake`](#0x3_governance_withdraw_stake)
-  [Function `execute_proposal`](#0x3_governance_execute_proposal)
-  [Function `withdraw_deposit`](#0x3_governance_withdraw_deposit)
-  [Function `get_proposal_count`](#0x3_governance_get_proposal_count)
-  [Function `get_voting_period`](#0x3_governance_get_voting_period)
-  [Function `get_quorum_threshold`](#0x3_governance_get_quorum_threshold)
-  [Function `get_total_supply`](#0x3_governance_get_total_supply)
-  [Function `is_paused`](#0x3_governance_is_paused)
-  [Function `vote_tally`](#0x3_governance_vote_tally)
-  [Function `stake_of`](#0x3_governance_stake_of)
-  [Function `is_executed`](#0x3_governance_is_executed)
-  [Function `execution_result`](#0x3_governance_execution_result)
-  [Function `deposit_of`](#0x3_governance_deposit_of)
-  [Function `pause`](#0x3_governance_pause)
-  [Function `unpause`](#0x3_governance_unpause)
-  [Function `set_voting_period`](#0x3_governance_set_voting_period)
-  [Function `set_quorum_threshold`](#0x3_governance_set_quorum_threshold)
-  [Function `set_total_supply`](#0x3_governance_set_total_supply)
-  [Function `find_stake_index`](#0x3_governance_find_stake_index)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="dependencies/move-stdlib/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="dependencies/move-stdlib/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="dependencies/kanari-system/balance.md#0x2_balance">0x2::balance</a>;
<b>use</b> <a href="dependencies/kanari-system/coin.md#0x2_coin">0x2::coin</a>;
<b>use</b> <a href="dependencies/kanari-system/event.md#0x2_event">0x2::event</a>;
<b>use</b> <a href="dependencies/kanari-system/kanari.md#0x2_kanari">0x2::kanari</a>;
<b>use</b> <a href="dependencies/kanari-system/object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="dependencies/kanari-system/transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context">0x2::tx_context</a>;
</code></pre>



<a name="0x3_governance_ProposalCreated"></a>

## Struct `ProposalCreated`

Emitted when a proposal is created


<pre><code><b>struct</b> <a href="governance.md#0x3_governance_ProposalCreated">ProposalCreated</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>title: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>deposit: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>created_at: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voting_period: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x3_governance_VoteCast"></a>

## Struct `VoteCast`

Emitted when a vote is cast


<pre><code><b>struct</b> <a href="governance.md#0x3_governance_VoteCast">VoteCast</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>support: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voted_at: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x3_governance_ProposalExecuted"></a>

## Struct `ProposalExecuted`

Emitted when a proposal is executed


<pre><code><b>struct</b> <a href="governance.md#0x3_governance_ProposalExecuted">ProposalExecuted</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>executed: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>executed_at: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x3_governance_GovernanceState"></a>

## Resource `GovernanceState`

Global governance state


<pre><code><b>struct</b> <a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="dependencies/kanari-system/object.md#0x2_object_UID">object::UID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>next_proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_proposals: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_supply: u64</code>
</dt>
<dd>
 Total circulating supply of the governed token, used for quorum.
 Admin-maintained until a supply oracle exists.
</dd>
<dt>
<code>voting_period: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>quorum_threshold: u64</code>
</dt>
<dd>
 Quorum in basis points of <code>total_supply</code> (10000 = 100%).
</dd>
<dt>
<code>is_paused: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x3_governance_VoteStake"></a>

## Struct `VoteStake`

One voter's escrowed stake on a proposal.


<pre><code><b>struct</b> <a href="governance.md#0x3_governance_VoteStake">VoteStake</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>support: bool</code>
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

<a name="0x3_governance_Proposal"></a>

## Resource `Proposal`

Proposal details. <code>COIN</code> is the governed token type; the escrowed
deposit and every vote stake are held in this object.


<pre><code><b>struct</b> <a href="governance.md#0x3_governance_Proposal">Proposal</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="dependencies/kanari-system/object.md#0x2_object_UID">object::UID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>
 Sequential proposal number assigned by <code><a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a></code>.
</dd>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>title: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>yes_votes: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>no_votes: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_staked: u64</code>
</dt>
<dd>
 Sum of all open stakes; the quorum measure.
</dd>
<dt>
<code>stakes: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="governance.md#0x3_governance_VoteStake">governance::VoteStake</a>&gt;</code>
</dt>
<dd>
 Per-voter escrowed stakes.
</dd>
<dt>
<code>escrow: <a href="dependencies/kanari-system/balance.md#0x2_balance_Balance">balance::Balance</a>&lt;<a href="dependencies/kanari-system/kanari.md#0x2_kanari_KANARI">kanari::KANARI</a>&gt;</code>
</dt>
<dd>
 Escrowed deposit + vote stakes, all withdrawable after the window.
</dd>
<dt>
<code>deposit: u64</code>
</dt>
<dd>
 The creator's escrowed deposit amount.
</dd>
<dt>
<code>deposit_returned: bool</code>
</dt>
<dd>
 Set once the creator has reclaimed the deposit.
</dd>
<dt>
<code>created_at: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voting_period: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>executed: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>execution_result: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x3_governance_GovernanceCap"></a>

## Resource `GovernanceCap`

Capability to manage governance (only one exists)


<pre><code><b>struct</b> <a href="governance.md#0x3_governance_GovernanceCap">GovernanceCap</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="dependencies/kanari-system/object.md#0x2_object_UID">object::UID</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x3_governance_E_ZERO_AMOUNT"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_ZERO_AMOUNT">E_ZERO_AMOUNT</a>: u64 = 9;
</code></pre>



<a name="0x3_governance_DEFAULT_VOTING_PERIOD"></a>

Default voting period in seconds (7 days)


<pre><code><b>const</b> <a href="governance.md#0x3_governance_DEFAULT_VOTING_PERIOD">DEFAULT_VOTING_PERIOD</a>: u64 = 604800;
</code></pre>



<a name="0x3_governance_E_ALREADY_VOTED"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_ALREADY_VOTED">E_ALREADY_VOTED</a>: u64 = 11;
</code></pre>



<a name="0x3_governance_E_DEPOSIT_RETURNED"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_DEPOSIT_RETURNED">E_DEPOSIT_RETURNED</a>: u64 = 15;
</code></pre>



<a name="0x3_governance_E_DEPOSIT_TOO_LOW"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_DEPOSIT_TOO_LOW">E_DEPOSIT_TOO_LOW</a>: u64 = 14;
</code></pre>



<a name="0x3_governance_E_DESCRIPTION_TOO_LONG"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_DESCRIPTION_TOO_LONG">E_DESCRIPTION_TOO_LONG</a>: u64 = 3;
</code></pre>



<a name="0x3_governance_E_EMPTY_TITLE"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_EMPTY_TITLE">E_EMPTY_TITLE</a>: u64 = 1;
</code></pre>



<a name="0x3_governance_E_GOVERNANCE_PAUSED"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_GOVERNANCE_PAUSED">E_GOVERNANCE_PAUSED</a>: u64 = 0;
</code></pre>



<a name="0x3_governance_E_INVALID_THRESHOLD"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>: u64 = 10;
</code></pre>



<a name="0x3_governance_E_INVALID_VOTING_PERIOD"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_INVALID_VOTING_PERIOD">E_INVALID_VOTING_PERIOD</a>: u64 = 4;
</code></pre>



<a name="0x3_governance_E_NOT_CREATOR"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_NOT_CREATOR">E_NOT_CREATOR</a>: u64 = 13;
</code></pre>



<a name="0x3_governance_E_NO_STAKE"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_NO_STAKE">E_NO_STAKE</a>: u64 = 12;
</code></pre>



<a name="0x3_governance_E_PROPOSAL_ALREADY_EXECUTED"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_PROPOSAL_ALREADY_EXECUTED">E_PROPOSAL_ALREADY_EXECUTED</a>: u64 = 5;
</code></pre>



<a name="0x3_governance_E_PROPOSAL_NOT_EXECUTED"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_PROPOSAL_NOT_EXECUTED">E_PROPOSAL_NOT_EXECUTED</a>: u64 = 16;
</code></pre>



<a name="0x3_governance_E_TITLE_TOO_LONG"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_TITLE_TOO_LONG">E_TITLE_TOO_LONG</a>: u64 = 2;
</code></pre>



<a name="0x3_governance_E_VOTING_ENDED"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_VOTING_ENDED">E_VOTING_ENDED</a>: u64 = 6;
</code></pre>



<a name="0x3_governance_E_VOTING_NOT_ENDED"></a>



<pre><code><b>const</b> <a href="governance.md#0x3_governance_E_VOTING_NOT_ENDED">E_VOTING_NOT_ENDED</a>: u64 = 7;
</code></pre>



<a name="0x3_governance_MIN_PROPOSAL_BALANCE"></a>

Minimum escrowed token deposit required to create a proposal
(10,000 whole units). The deposit is returned to the creator after the
proposal is executed.


<pre><code><b>const</b> <a href="governance.md#0x3_governance_MIN_PROPOSAL_BALANCE">MIN_PROPOSAL_BALANCE</a>: u64 = 10000;
</code></pre>



<a name="0x3_governance_QUORUM_THRESHOLD"></a>

Default quorum in basis points of <code>total_supply</code> (1000 = 10%).


<pre><code><b>const</b> <a href="governance.md#0x3_governance_QUORUM_THRESHOLD">QUORUM_THRESHOLD</a>: u64 = 1000;
</code></pre>



<a name="0x3_governance_init"></a>

## Function `init`



<pre><code><b>fun</b> <a href="governance.md#0x3_governance_init">init</a>(ctx: &<b>mut</b> <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="governance.md#0x3_governance_init">init</a>(ctx: &<b>mut</b> TxContext) {
    <b>let</b> cap = <a href="governance.md#0x3_governance_GovernanceCap">GovernanceCap</a> {
        id: <a href="dependencies/kanari-system/object.md#0x2_object_new">object::new</a>(ctx)
    };
    <a href="dependencies/kanari-system/transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(cap, <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx));
    <a href="dependencies/kanari-system/transfer.md#0x2_transfer_share_object">transfer::share_object</a>(
        <a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a> {
            id: <a href="dependencies/kanari-system/object.md#0x2_object_new">object::new</a>(ctx),
            next_proposal_id: 0,
            total_proposals: 0,
            total_supply: 0,
            voting_period: <a href="governance.md#0x3_governance_DEFAULT_VOTING_PERIOD">DEFAULT_VOTING_PERIOD</a>,
            quorum_threshold: <a href="governance.md#0x3_governance_QUORUM_THRESHOLD">QUORUM_THRESHOLD</a>,
            is_paused: <b>false</b>
        }
    );
}
</code></pre>



</details>

<a name="0x3_governance_create_proposal"></a>

## Function `create_proposal`

Create a proposal, escrowing <code>deposit</code> (at least <code><a href="governance.md#0x3_governance_MIN_PROPOSAL_BALANCE">MIN_PROPOSAL_BALANCE</a></code>)
until the proposal is executed. Permissionless: no capability needed.


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_create_proposal">create_proposal</a>(state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">governance::GovernanceState</a>, deposit: <a href="dependencies/kanari-system/coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="dependencies/kanari-system/kanari.md#0x2_kanari_KANARI">kanari::KANARI</a>&gt;, ctx: &<b>mut</b> <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>, title: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, description: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, voting_period: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_create_proposal">create_proposal</a>(
    state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a>,
    deposit: Coin&lt;KANARI&gt;,
    ctx: &<b>mut</b> TxContext,
    title: String,
    description: String,
    voting_period: u64
) {
    <b>assert</b>!(!state.is_paused, <a href="governance.md#0x3_governance_E_GOVERNANCE_PAUSED">E_GOVERNANCE_PAUSED</a>);
    <b>assert</b>!(<a href="dependencies/move-stdlib/string.md#0x1_string_length">string::length</a>(&title) &gt; 0, <a href="governance.md#0x3_governance_E_EMPTY_TITLE">E_EMPTY_TITLE</a>);
    <b>assert</b>!(<a href="dependencies/move-stdlib/string.md#0x1_string_length">string::length</a>(&title) &lt;= 256, <a href="governance.md#0x3_governance_E_TITLE_TOO_LONG">E_TITLE_TOO_LONG</a>);
    <b>assert</b>!(<a href="dependencies/move-stdlib/string.md#0x1_string_length">string::length</a>(&description) &lt;= 4096, <a href="governance.md#0x3_governance_E_DESCRIPTION_TOO_LONG">E_DESCRIPTION_TOO_LONG</a>);
    <b>assert</b>!(voting_period &gt; 0, <a href="governance.md#0x3_governance_E_INVALID_VOTING_PERIOD">E_INVALID_VOTING_PERIOD</a>);
    <b>assert</b>!(<a href="dependencies/kanari-system/coin.md#0x2_coin_value">coin::value</a>(&deposit) &gt;= <a href="governance.md#0x3_governance_MIN_PROPOSAL_BALANCE">MIN_PROPOSAL_BALANCE</a>, <a href="governance.md#0x3_governance_E_DEPOSIT_TOO_LOW">E_DEPOSIT_TOO_LOW</a>);

    <b>let</b> sender = <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);
    <b>let</b> deposit_amount = <a href="dependencies/kanari-system/coin.md#0x2_coin_value">coin::value</a>(&deposit);

    <b>let</b> proposal_id = state.next_proposal_id;
    state.next_proposal_id = state.next_proposal_id + 1;
    state.total_proposals = state.total_proposals + 1;

    // The <a href="dependencies/kanari-system/event.md#0x2_event">event</a> is emitted after the strings are moved into `<a href="governance.md#0x3_governance_Proposal">Proposal</a>`,
    // so keep copies for the payload.
    <b>let</b> title_copy = <b>copy</b> title;
    <b>let</b> description_copy = <b>copy</b> description;
    <b>let</b> created_at = <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_epoch_timestamp_ms">tx_context::epoch_timestamp_ms</a>(ctx);

    // Proposals are shared: any token holder can read and vote on them.
    <a href="dependencies/kanari-system/transfer.md#0x2_transfer_share_object">transfer::share_object</a>(
        <a href="governance.md#0x3_governance_Proposal">Proposal</a> {
            id: <a href="dependencies/kanari-system/object.md#0x2_object_new">object::new</a>(ctx),
            proposal_id,
            creator: sender,
            title,
            description,
            yes_votes: 0,
            no_votes: 0,
            total_staked: 0,
            stakes: <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="governance.md#0x3_governance_VoteStake">VoteStake</a>&gt;(),
            escrow: <a href="dependencies/kanari-system/coin.md#0x2_coin_into_balance">coin::into_balance</a>(deposit),
            deposit: deposit_amount,
            deposit_returned: <b>false</b>,
            created_at,
            voting_period,
            executed: <b>false</b>,
            execution_result: <b>false</b>
        }
    );

    <a href="dependencies/kanari-system/event.md#0x2_event_emit">event::emit</a>(
        <a href="governance.md#0x3_governance_ProposalCreated">ProposalCreated</a> {
            proposal_id,
            creator: sender,
            title: title_copy,
            description: description_copy,
            deposit: deposit_amount,
            created_at,
            voting_period
        }
    );
}
</code></pre>



</details>

<a name="0x3_governance_cast_vote"></a>

## Function `cast_vote`

Cast a vote by staking tokens. The coin is escrowed in the proposal and
becomes the voter's weight; it is withdrawable once the window closes.
A voter may top up the same side, but cannot switch sides.


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_cast_vote">cast_vote</a>(state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">governance::GovernanceState</a>, proposal: &<b>mut</b> <a href="governance.md#0x3_governance_Proposal">governance::Proposal</a>, support: bool, <a href="dependencies/kanari-system/coin.md#0x2_coin">coin</a>: <a href="dependencies/kanari-system/coin.md#0x2_coin_Coin">coin::Coin</a>&lt;<a href="dependencies/kanari-system/kanari.md#0x2_kanari_KANARI">kanari::KANARI</a>&gt;, ctx: &<b>mut</b> <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_cast_vote">cast_vote</a>(
    state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a>,
    proposal: &<b>mut</b> <a href="governance.md#0x3_governance_Proposal">Proposal</a>,
    support: bool,
    <a href="dependencies/kanari-system/coin.md#0x2_coin">coin</a>: Coin&lt;KANARI&gt;,
    ctx: &<b>mut</b> TxContext
) {
    <b>assert</b>!(!state.is_paused, <a href="governance.md#0x3_governance_E_GOVERNANCE_PAUSED">E_GOVERNANCE_PAUSED</a>);
    <b>assert</b>!(!proposal.executed, <a href="governance.md#0x3_governance_E_PROPOSAL_ALREADY_EXECUTED">E_PROPOSAL_ALREADY_EXECUTED</a>);
    <b>let</b> amount = <a href="dependencies/kanari-system/coin.md#0x2_coin_value">coin::value</a>(&<a href="dependencies/kanari-system/coin.md#0x2_coin">coin</a>);
    <b>assert</b>!(amount &gt; 0, <a href="governance.md#0x3_governance_E_ZERO_AMOUNT">E_ZERO_AMOUNT</a>);
    <b>assert</b>!(
        <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_epoch_timestamp_ms">tx_context::epoch_timestamp_ms</a>(ctx) &lt; proposal.created_at + proposal.voting_period,
        <a href="governance.md#0x3_governance_E_VOTING_ENDED">E_VOTING_ENDED</a>
    );

    <b>let</b> voter = <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);
    <b>let</b> idx = <a href="governance.md#0x3_governance_find_stake_index">find_stake_index</a>(&proposal.stakes, voter);
    <b>if</b> (idx &lt; <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&proposal.stakes)) {
        <b>let</b> existing = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> proposal.stakes, idx);
        <b>assert</b>!(existing.support == support, <a href="governance.md#0x3_governance_E_ALREADY_VOTED">E_ALREADY_VOTED</a>);
        existing.amount = existing.amount + amount;
    } <b>else</b> {
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(
            &<b>mut</b> proposal.stakes,
            <a href="governance.md#0x3_governance_VoteStake">VoteStake</a> {
                voter,
                support,
                amount
            }
        );
    };

    <b>if</b> (support) {
        proposal.yes_votes = proposal.yes_votes + amount;
    } <b>else</b> {
        proposal.no_votes = proposal.no_votes + amount;
    };
    proposal.total_staked = proposal.total_staked + amount;

    // Escrow the <a href="dependencies/kanari-system/coin.md#0x2_coin">coin</a> inside the proposal.
    <a href="dependencies/kanari-system/balance.md#0x2_balance_merge">balance::merge</a>(&<b>mut</b> proposal.escrow, <a href="dependencies/kanari-system/coin.md#0x2_coin_into_balance">coin::into_balance</a>(<a href="dependencies/kanari-system/coin.md#0x2_coin">coin</a>));

    <a href="dependencies/kanari-system/event.md#0x2_event_emit">event::emit</a>(
        <a href="governance.md#0x3_governance_VoteCast">VoteCast</a> {
            proposal_id: proposal.proposal_id,
            voter,
            support,
            amount,
            voted_at: <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_epoch_timestamp_ms">tx_context::epoch_timestamp_ms</a>(ctx)
        }
    );
}
</code></pre>



</details>

<a name="0x3_governance_withdraw_stake"></a>

## Function `withdraw_stake`

Reclaim one's escrowed stake after the voting window has closed.


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_withdraw_stake">withdraw_stake</a>(proposal: &<b>mut</b> <a href="governance.md#0x3_governance_Proposal">governance::Proposal</a>, ctx: &<b>mut</b> <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_withdraw_stake">withdraw_stake</a>(proposal: &<b>mut</b> <a href="governance.md#0x3_governance_Proposal">Proposal</a>, ctx: &<b>mut</b> TxContext) {
    <b>assert</b>!(
        <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_epoch_timestamp_ms">tx_context::epoch_timestamp_ms</a>(ctx) &gt;= proposal.created_at + proposal.voting_period,
        <a href="governance.md#0x3_governance_E_VOTING_NOT_ENDED">E_VOTING_NOT_ENDED</a>
    );
    <b>let</b> voter = <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx);
    <b>let</b> idx = <a href="governance.md#0x3_governance_find_stake_index">find_stake_index</a>(&proposal.stakes, voter);
    <b>assert</b>!(idx &lt; <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&proposal.stakes), <a href="governance.md#0x3_governance_E_NO_STAKE">E_NO_STAKE</a>);

    <b>let</b> <a href="governance.md#0x3_governance_VoteStake">VoteStake</a> { voter, support, amount } = <a href="dependencies/move-stdlib/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> proposal.stakes, idx);
    <b>if</b> (support) {
        proposal.yes_votes = proposal.yes_votes - amount;
    } <b>else</b> {
        proposal.no_votes = proposal.no_votes - amount;
    };
    proposal.total_staked = proposal.total_staked - amount;

    <b>let</b> returned = <a href="dependencies/kanari-system/balance.md#0x2_balance_split">balance::split</a>(&<b>mut</b> proposal.escrow, amount);
    <a href="dependencies/kanari-system/transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(<a href="dependencies/kanari-system/coin.md#0x2_coin_from_balance">coin::from_balance</a>(returned, ctx), voter);
}
</code></pre>



</details>

<a name="0x3_governance_execute_proposal"></a>

## Function `execute_proposal`

Execute a proposal after the voting period ends. Passing requires a
yes-majority and quorum (<code>total_staked &gt;= total_supply * threshold</code>
in basis points). Marks the proposal executed so voters can withdraw.


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_execute_proposal">execute_proposal</a>(state: &<a href="governance.md#0x3_governance_GovernanceState">governance::GovernanceState</a>, proposal: &<b>mut</b> <a href="governance.md#0x3_governance_Proposal">governance::Proposal</a>, ctx: &<b>mut</b> <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_execute_proposal">execute_proposal</a>(
    state: &<a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a>,
    proposal: &<b>mut</b> <a href="governance.md#0x3_governance_Proposal">Proposal</a>,
    ctx: &<b>mut</b> TxContext
) {
    <b>assert</b>!(!proposal.executed, <a href="governance.md#0x3_governance_E_PROPOSAL_ALREADY_EXECUTED">E_PROPOSAL_ALREADY_EXECUTED</a>);
    <b>assert</b>!(
        <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_epoch_timestamp_ms">tx_context::epoch_timestamp_ms</a>(ctx) &gt;= proposal.created_at + proposal.voting_period,
        <a href="governance.md#0x3_governance_E_VOTING_NOT_ENDED">E_VOTING_NOT_ENDED</a>
    );

    // Quorum in basis points of `total_supply`. Divide first <b>to</b> avoid
    // `u64` overflow; the floor costs at most (threshold-1) units.
    <b>let</b> required = state.total_supply / 10000 * state.quorum_threshold;
    <b>let</b> quorum_met = proposal.total_staked &gt;= required;
    <b>let</b> passed = proposal.yes_votes &gt; proposal.no_votes && quorum_met;

    proposal.executed = <b>true</b>;
    proposal.execution_result = passed;

    <a href="dependencies/kanari-system/event.md#0x2_event_emit">event::emit</a>(
        <a href="governance.md#0x3_governance_ProposalExecuted">ProposalExecuted</a> {
            proposal_id: proposal.proposal_id,
            executed: passed,
            executed_at: <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_epoch_timestamp_ms">tx_context::epoch_timestamp_ms</a>(ctx)
        }
    );
}
</code></pre>



</details>

<a name="0x3_governance_withdraw_deposit"></a>

## Function `withdraw_deposit`

Reclaim the creator's escrowed deposit once the proposal is executed.


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_withdraw_deposit">withdraw_deposit</a>(proposal: &<b>mut</b> <a href="governance.md#0x3_governance_Proposal">governance::Proposal</a>, ctx: &<b>mut</b> <a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_withdraw_deposit">withdraw_deposit</a>(proposal: &<b>mut</b> <a href="governance.md#0x3_governance_Proposal">Proposal</a>, ctx: &<b>mut</b> TxContext) {
    <b>assert</b>!(<a href="dependencies/kanari-system/tx_context.md#0x2_tx_context_sender">tx_context::sender</a>(ctx) == proposal.creator, <a href="governance.md#0x3_governance_E_NOT_CREATOR">E_NOT_CREATOR</a>);
    <b>assert</b>!(proposal.executed, <a href="governance.md#0x3_governance_E_PROPOSAL_NOT_EXECUTED">E_PROPOSAL_NOT_EXECUTED</a>);
    <b>assert</b>!(!proposal.deposit_returned, <a href="governance.md#0x3_governance_E_DEPOSIT_RETURNED">E_DEPOSIT_RETURNED</a>);

    proposal.deposit_returned = <b>true</b>;
    <b>let</b> deposit = <a href="dependencies/kanari-system/balance.md#0x2_balance_split">balance::split</a>(&<b>mut</b> proposal.escrow, proposal.deposit);
    <a href="dependencies/kanari-system/transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(<a href="dependencies/kanari-system/coin.md#0x2_coin_from_balance">coin::from_balance</a>(deposit, ctx), proposal.creator);
}
</code></pre>



</details>

<a name="0x3_governance_get_proposal_count"></a>

## Function `get_proposal_count`



<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_get_proposal_count">get_proposal_count</a>(state: &<a href="governance.md#0x3_governance_GovernanceState">governance::GovernanceState</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_get_proposal_count">get_proposal_count</a>(state: &<a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a>): u64 {
    state.total_proposals
}
</code></pre>



</details>

<a name="0x3_governance_get_voting_period"></a>

## Function `get_voting_period`



<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_get_voting_period">get_voting_period</a>(state: &<a href="governance.md#0x3_governance_GovernanceState">governance::GovernanceState</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_get_voting_period">get_voting_period</a>(state: &<a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a>): u64 {
    state.voting_period
}
</code></pre>



</details>

<a name="0x3_governance_get_quorum_threshold"></a>

## Function `get_quorum_threshold`



<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_get_quorum_threshold">get_quorum_threshold</a>(state: &<a href="governance.md#0x3_governance_GovernanceState">governance::GovernanceState</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_get_quorum_threshold">get_quorum_threshold</a>(state: &<a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a>): u64 {
    state.quorum_threshold
}
</code></pre>



</details>

<a name="0x3_governance_get_total_supply"></a>

## Function `get_total_supply`



<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_get_total_supply">get_total_supply</a>(state: &<a href="governance.md#0x3_governance_GovernanceState">governance::GovernanceState</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_get_total_supply">get_total_supply</a>(state: &<a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a>): u64 {
    state.total_supply
}
</code></pre>



</details>

<a name="0x3_governance_is_paused"></a>

## Function `is_paused`



<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_is_paused">is_paused</a>(state: &<a href="governance.md#0x3_governance_GovernanceState">governance::GovernanceState</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_is_paused">is_paused</a>(state: &<a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a>): bool {
    state.is_paused
}
</code></pre>



</details>

<a name="0x3_governance_vote_tally"></a>

## Function `vote_tally`

Vote tallies as <code>(yes, no, total_staked)</code>.


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_vote_tally">vote_tally</a>(proposal: &<a href="governance.md#0x3_governance_Proposal">governance::Proposal</a>): (u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_vote_tally">vote_tally</a>(proposal: &<a href="governance.md#0x3_governance_Proposal">Proposal</a>): (u64, u64, u64) {
    (proposal.yes_votes, proposal.no_votes, proposal.total_staked)
}
</code></pre>



</details>

<a name="0x3_governance_stake_of"></a>

## Function `stake_of`

A voter's currently escrowed stake, or <code>none</code>.


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_stake_of">stake_of</a>(proposal: &<a href="governance.md#0x3_governance_Proposal">governance::Proposal</a>, voter: <b>address</b>): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_stake_of">stake_of</a>(proposal: &<a href="governance.md#0x3_governance_Proposal">Proposal</a>, voter: <b>address</b>): Option&lt;u64&gt; {
    <b>let</b> idx = <a href="governance.md#0x3_governance_find_stake_index">find_stake_index</a>(&proposal.stakes, voter);
    <b>if</b> (idx &lt; <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&proposal.stakes)) {
        <b>let</b> stake = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&proposal.stakes, idx);
        <a href="dependencies/move-stdlib/option.md#0x1_option_some">option::some</a>(stake.amount)
    } <b>else</b> {
        <a href="dependencies/move-stdlib/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a name="0x3_governance_is_executed"></a>

## Function `is_executed`

Whether the proposal has been executed (pass or fail).


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_is_executed">is_executed</a>(proposal: &<a href="governance.md#0x3_governance_Proposal">governance::Proposal</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_is_executed">is_executed</a>(proposal: &<a href="governance.md#0x3_governance_Proposal">Proposal</a>): bool {
    proposal.executed
}
</code></pre>



</details>

<a name="0x3_governance_execution_result"></a>

## Function `execution_result`

The recorded execution outcome (<code><b>true</b></code> = passed quorum and majority).


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_execution_result">execution_result</a>(proposal: &<a href="governance.md#0x3_governance_Proposal">governance::Proposal</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_execution_result">execution_result</a>(proposal: &<a href="governance.md#0x3_governance_Proposal">Proposal</a>): bool {
    proposal.execution_result
}
</code></pre>



</details>

<a name="0x3_governance_deposit_of"></a>

## Function `deposit_of`

The creator's escrowed deposit amount for this proposal.


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_deposit_of">deposit_of</a>(proposal: &<a href="governance.md#0x3_governance_Proposal">governance::Proposal</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governance.md#0x3_governance_deposit_of">deposit_of</a>(proposal: &<a href="governance.md#0x3_governance_Proposal">Proposal</a>): u64 {
    proposal.deposit
}
</code></pre>



</details>

<a name="0x3_governance_pause"></a>

## Function `pause`

Pause governance (emergency stop)


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_pause">pause</a>(cap: &<a href="governance.md#0x3_governance_GovernanceCap">governance::GovernanceCap</a>, state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">governance::GovernanceState</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_pause">pause</a>(cap: &<a href="governance.md#0x3_governance_GovernanceCap">GovernanceCap</a>, state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a>) {
    state.is_paused = <b>true</b>;
}
</code></pre>



</details>

<a name="0x3_governance_unpause"></a>

## Function `unpause`

Unpause governance


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_unpause">unpause</a>(cap: &<a href="governance.md#0x3_governance_GovernanceCap">governance::GovernanceCap</a>, state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">governance::GovernanceState</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_unpause">unpause</a>(cap: &<a href="governance.md#0x3_governance_GovernanceCap">GovernanceCap</a>, state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a>) {
    state.is_paused = <b>false</b>;
}
</code></pre>



</details>

<a name="0x3_governance_set_voting_period"></a>

## Function `set_voting_period`

Update the default voting period for new proposals


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_set_voting_period">set_voting_period</a>(cap: &<a href="governance.md#0x3_governance_GovernanceCap">governance::GovernanceCap</a>, state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">governance::GovernanceState</a>, period: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_set_voting_period">set_voting_period</a>(
    cap: &<a href="governance.md#0x3_governance_GovernanceCap">GovernanceCap</a>,
    state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a>,
    period: u64
) {
    <b>assert</b>!(period &gt; 0, <a href="governance.md#0x3_governance_E_INVALID_VOTING_PERIOD">E_INVALID_VOTING_PERIOD</a>);
    state.voting_period = period;
}
</code></pre>



</details>

<a name="0x3_governance_set_quorum_threshold"></a>

## Function `set_quorum_threshold`

Update the quorum threshold (basis points, at most 100%)


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_set_quorum_threshold">set_quorum_threshold</a>(cap: &<a href="governance.md#0x3_governance_GovernanceCap">governance::GovernanceCap</a>, state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">governance::GovernanceState</a>, threshold: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_set_quorum_threshold">set_quorum_threshold</a>(
    cap: &<a href="governance.md#0x3_governance_GovernanceCap">GovernanceCap</a>,
    state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a>,
    threshold: u64
) {
    <b>assert</b>!(threshold &lt;= 10000, <a href="governance.md#0x3_governance_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>);
    state.quorum_threshold = threshold;
}
</code></pre>



</details>

<a name="0x3_governance_set_total_supply"></a>

## Function `set_total_supply`

Sync <code>total_supply</code> from the Kanari treasury cap (supply oracle). The
admin supplies the treasury cap; governance reads the authoritative
total supply directly instead of trusting a manually entered value.


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_set_total_supply">set_total_supply</a>(cap: &<a href="governance.md#0x3_governance_GovernanceCap">governance::GovernanceCap</a>, state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">governance::GovernanceState</a>, treasury_cap: &<a href="dependencies/kanari-system/coin.md#0x2_coin_TreasuryCap">coin::TreasuryCap</a>&lt;<a href="dependencies/kanari-system/kanari.md#0x2_kanari_KANARI">kanari::KANARI</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="governance.md#0x3_governance_set_total_supply">set_total_supply</a>(
    cap: &<a href="governance.md#0x3_governance_GovernanceCap">GovernanceCap</a>,
    state: &<b>mut</b> <a href="governance.md#0x3_governance_GovernanceState">GovernanceState</a>,
    treasury_cap: &TreasuryCap&lt;KANARI&gt;,
) {
    state.total_supply = <a href="dependencies/kanari-system/coin.md#0x2_coin_total_supply">coin::total_supply</a>&lt;KANARI&gt;(treasury_cap);
}
</code></pre>



</details>

<a name="0x3_governance_find_stake_index"></a>

## Function `find_stake_index`

Index of <code>voter</code>'s stake, or <code><a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(stakes)</code> when absent.


<pre><code><b>fun</b> <a href="governance.md#0x3_governance_find_stake_index">find_stake_index</a>(stakes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="governance.md#0x3_governance_VoteStake">governance::VoteStake</a>&gt;, voter: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="governance.md#0x3_governance_find_stake_index">find_stake_index</a>(stakes: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="governance.md#0x3_governance_VoteStake">VoteStake</a>&gt;, voter: <b>address</b>): u64 {
    <b>let</b> i = 0;
    <b>let</b> n = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(stakes);
    <b>while</b> (i &lt; n) {
        <b>let</b> stake = <a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(stakes, i);
        <b>if</b> (stake.voter == voter) {
            <b>return</b> i
        };
        i = i + 1;
    };
    n
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/kanari-network/kanari-sdk
[Move Book]: https://move-language.github.io/move/
[Governance Module]: governance.md
