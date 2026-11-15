// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Kanari Governance Module
///
/// On-chain governance with real, staked voting:
///
/// - Creating a proposal requires an escrowed token deposit
///   (`MIN_PROPOSAL_BALANCE`), returned to the creator after execution.
/// - Voting stakes tokens: the `Coin` passed to `cast_vote` is escrowed in the
///   proposal and becomes the voter's voting weight, so a vote is always
///   backed by locked value.
/// - One stake per voter per proposal: a voter may top up the same side, but
///   switching sides aborts.
/// - Stakes (and the deposit) are withdrawable once the voting window closes,
///   so participation costs time, not principal.
/// - Execution requires quorum: the total staked amount must reach
///   `total_supply * quorum_threshold` in basis points.
module kanari_governance::governance {

    use std::option::{Self, Option};
    use std::string::{Self, String};
    use std::vector;
    use kanari_system::balance::{Self, Balance};
    use kanari_system::coin::{Self, Coin};
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::event;
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::kanari::KANARI;


    // ========================================================================
    // Constants
    // ========================================================================

    /// Minimum escrowed token deposit required to create a proposal
    /// (10,000 whole units). The deposit is returned to the creator after the
    /// proposal is executed.
    const MIN_PROPOSAL_BALANCE: u64 = 10_000;

    /// Default voting period in seconds (7 days)
    const DEFAULT_VOTING_PERIOD: u64 = 604_800;

    /// Default quorum in basis points of `total_supply` (1000 = 10%).
    const QUORUM_THRESHOLD: u64 = 1000;

    // ========================================================================
    // Events
    // ========================================================================

    /// Emitted when a proposal is created
    struct ProposalCreated has copy, drop {
        proposal_id: u64,
        creator: address,
        title: String,
        description: String,
        deposit: u64,
        created_at: u64,
        voting_period: u64
    }

    /// Emitted when a vote is cast
    struct VoteCast has copy, drop {
        proposal_id: u64,
        voter: address,
        support: bool,
        amount: u64,
        voted_at: u64
    }

    /// Emitted when a proposal is executed
    struct ProposalExecuted has copy, drop {
        proposal_id: u64,
        executed: bool,
        executed_at: u64
    }

    // ========================================================================
    // Resources
    // ========================================================================

    /// Global governance state
    struct GovernanceState has key, store {
        id: UID,
        next_proposal_id: u64,
        total_proposals: u64,
        /// Total circulating supply of the governed token, used for quorum.
        /// Admin-maintained until a supply oracle exists.
        total_supply: u64,
        voting_period: u64,
        /// Quorum in basis points of `total_supply` (10000 = 100%).
        quorum_threshold: u64,
        is_paused: bool
    }

    /// One voter's escrowed stake on a proposal.
    struct VoteStake has store {
        voter: address,
        support: bool,
        amount: u64
    }

    /// Proposal details. `COIN` is the governed token type; the escrowed
    /// deposit and every vote stake are held in this object.
    struct Proposal has key, store {
        id: UID,
        /// Sequential proposal number assigned by `GovernanceState`.
        proposal_id: u64,
        creator: address,
        title: String,
        description: String,
        yes_votes: u64,
        no_votes: u64,
        /// Sum of all open stakes; the quorum measure.
        total_staked: u64,
        /// Per-voter escrowed stakes.
        stakes: vector<VoteStake>,
        /// Escrowed deposit + vote stakes, all withdrawable after the window.
        escrow: Balance<KANARI>,
        /// The creator's escrowed deposit amount.
        deposit: u64,
        /// Set once the creator has reclaimed the deposit.
        deposit_returned: bool,
        created_at: u64,
        voting_period: u64,
        executed: bool,
        execution_result: bool
    }

    /// Capability to manage governance (only one exists)
    struct GovernanceCap has key, store {
        id: UID
    }

    // ========================================================================
    // Initialization
    // ========================================================================

    #[allow(unused_function)]
    fun init(ctx: &mut TxContext) {
        let cap = GovernanceCap {
            id: object::new(ctx)
        };
        transfer::public_transfer(cap, tx_context::sender(ctx));
        transfer::share_object(
            GovernanceState {
                id: object::new(ctx),
                next_proposal_id: 0,
                total_proposals: 0,
                total_supply: 0,
                voting_period: DEFAULT_VOTING_PERIOD,
                quorum_threshold: QUORUM_THRESHOLD,
                is_paused: false
            }
        );
    }

    // ========================================================================
    // Public Entry Functions
    // ========================================================================

    /// Create a proposal, escrowing `deposit` (at least `MIN_PROPOSAL_BALANCE`)
    /// until the proposal is executed. Permissionless: no capability needed.
    public entry fun create_proposal(
        state: &mut GovernanceState,
        deposit: Coin<KANARI>,
        ctx: &mut TxContext,
        title: String,
        description: String,
        voting_period: u64
    ) {
        assert!(!state.is_paused, E_GOVERNANCE_PAUSED);
        assert!(string::length(&title) > 0, E_EMPTY_TITLE);
        assert!(string::length(&title) <= 256, E_TITLE_TOO_LONG);
        assert!(string::length(&description) <= 4096, E_DESCRIPTION_TOO_LONG);
        assert!(voting_period > 0, E_INVALID_VOTING_PERIOD);
        assert!(coin::value(&deposit) >= MIN_PROPOSAL_BALANCE, E_DEPOSIT_TOO_LOW);

        let sender = tx_context::sender(ctx);
        let deposit_amount = coin::value(&deposit);

        let proposal_id = state.next_proposal_id;
        state.next_proposal_id = state.next_proposal_id + 1;
        state.total_proposals = state.total_proposals + 1;

        // The event is emitted after the strings are moved into `Proposal`,
        // so keep copies for the payload.
        let title_copy = copy title;
        let description_copy = copy description;
        let created_at = tx_context::epoch_timestamp_ms(ctx);

        // Proposals are shared: any token holder can read and vote on them.
        transfer::share_object(
            Proposal {
                id: object::new(ctx),
                proposal_id,
                creator: sender,
                title,
                description,
                yes_votes: 0,
                no_votes: 0,
                total_staked: 0,
                stakes: vector::empty<VoteStake>(),
                escrow: coin::into_balance(deposit),
                deposit: deposit_amount,
                deposit_returned: false,
                created_at,
                voting_period,
                executed: false,
                execution_result: false
            }
        );

        event::emit(
            ProposalCreated {
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

    /// Cast a vote by staking tokens. The coin is escrowed in the proposal and
    /// becomes the voter's weight; it is withdrawable once the window closes.
    /// A voter may top up the same side, but cannot switch sides.
    public entry fun cast_vote(
        state: &mut GovernanceState,
        proposal: &mut Proposal,
        support: bool,
        coin: Coin<KANARI>,
        ctx: &mut TxContext
    ) {
        assert!(!state.is_paused, E_GOVERNANCE_PAUSED);
        assert!(!proposal.executed, E_PROPOSAL_ALREADY_EXECUTED);
        let amount = coin::value(&coin);
        assert!(amount > 0, E_ZERO_AMOUNT);
        assert!(
            tx_context::epoch_timestamp_ms(ctx) < proposal.created_at + proposal.voting_period,
            E_VOTING_ENDED
        );

        let voter = tx_context::sender(ctx);
        let idx = find_stake_index(&proposal.stakes, voter);
        if (idx < vector::length(&proposal.stakes)) {
            let existing = vector::borrow_mut(&mut proposal.stakes, idx);
            assert!(existing.support == support, E_ALREADY_VOTED);
            existing.amount = existing.amount + amount;
        } else {
            vector::push_back(
                &mut proposal.stakes,
                VoteStake {
                    voter,
                    support,
                    amount
                }
            );
        };

        if (support) {
            proposal.yes_votes = proposal.yes_votes + amount;
        } else {
            proposal.no_votes = proposal.no_votes + amount;
        };
        proposal.total_staked = proposal.total_staked + amount;

        // Escrow the coin inside the proposal.
        balance::merge(&mut proposal.escrow, coin::into_balance(coin));

        event::emit(
            VoteCast {
                proposal_id: proposal.proposal_id,
                voter,
                support,
                amount,
                voted_at: tx_context::epoch_timestamp_ms(ctx)
            }
        );
    }

    /// Reclaim one's escrowed stake after the voting window has closed.
    public entry fun withdraw_stake(proposal: &mut Proposal, ctx: &mut TxContext) {
        assert!(
            tx_context::epoch_timestamp_ms(ctx) >= proposal.created_at + proposal.voting_period,
            E_VOTING_NOT_ENDED
        );
        let voter = tx_context::sender(ctx);
        let idx = find_stake_index(&proposal.stakes, voter);
        assert!(idx < vector::length(&proposal.stakes), E_NO_STAKE);

        let VoteStake { voter, support, amount } = vector::remove(&mut proposal.stakes, idx);
        if (support) {
            proposal.yes_votes = proposal.yes_votes - amount;
        } else {
            proposal.no_votes = proposal.no_votes - amount;
        };
        proposal.total_staked = proposal.total_staked - amount;

        let returned = balance::split(&mut proposal.escrow, amount);
        transfer::public_transfer(coin::from_balance(returned, ctx), voter);
    }

    #[allow(unused_variable)]
    /// Execute a proposal after the voting period ends. Passing requires a
    /// yes-majority and quorum (`total_staked >= total_supply * threshold`
    /// in basis points). Marks the proposal executed so voters can withdraw.
    public entry fun execute_proposal(
        state: &GovernanceState,
        proposal: &mut Proposal,
        ctx: &mut TxContext
    ) {
        assert!(!proposal.executed, E_PROPOSAL_ALREADY_EXECUTED);
        assert!(
            tx_context::epoch_timestamp_ms(ctx) >= proposal.created_at + proposal.voting_period,
            E_VOTING_NOT_ENDED
        );

        // Quorum in basis points of `total_supply`. Divide first to avoid
        // `u64` overflow; the floor costs at most (threshold-1) units.
        let required = state.total_supply / 10000 * state.quorum_threshold;
        let quorum_met = proposal.total_staked >= required;
        let passed = proposal.yes_votes > proposal.no_votes && quorum_met;

        proposal.executed = true;
        proposal.execution_result = passed;

        event::emit(
            ProposalExecuted {
                proposal_id: proposal.proposal_id,
                executed: passed,
                executed_at: tx_context::epoch_timestamp_ms(ctx)
            }
        );
    }

    /// Reclaim the creator's escrowed deposit once the proposal is executed.
    public entry fun withdraw_deposit(proposal: &mut Proposal, ctx: &mut TxContext) {
        assert!(tx_context::sender(ctx) == proposal.creator, E_NOT_CREATOR);
        assert!(proposal.executed, E_PROPOSAL_NOT_EXECUTED);
        assert!(!proposal.deposit_returned, E_DEPOSIT_RETURNED);

        proposal.deposit_returned = true;
        let deposit = balance::split(&mut proposal.escrow, proposal.deposit);
        transfer::public_transfer(coin::from_balance(deposit, ctx), proposal.creator);
    }

    // ========================================================================
    // View Functions
    // ========================================================================

    public fun get_proposal_count(state: &GovernanceState): u64 {
        state.total_proposals
    }

    public fun get_voting_period(state: &GovernanceState): u64 {
        state.voting_period
    }

    public fun get_quorum_threshold(state: &GovernanceState): u64 {
        state.quorum_threshold
    }

    public fun get_total_supply(state: &GovernanceState): u64 {
        state.total_supply
    }

    public fun is_paused(state: &GovernanceState): bool {
        state.is_paused
    }

    /// Vote tallies as `(yes, no, total_staked)`.
    public fun vote_tally(proposal: &Proposal): (u64, u64, u64) {
        (proposal.yes_votes, proposal.no_votes, proposal.total_staked)
    }

    /// A voter's currently escrowed stake, or `none`.
    public fun stake_of(proposal: &Proposal, voter: address): Option<u64> {
        let idx = find_stake_index(&proposal.stakes, voter);
        if (idx < vector::length(&proposal.stakes)) {
            let stake = vector::borrow(&proposal.stakes, idx);
            option::some(stake.amount)
        } else {
            option::none()
        }
    }

    /// Whether the proposal has been executed (pass or fail).
    public fun is_executed(proposal: &Proposal): bool {
        proposal.executed
    }

    /// The recorded execution outcome (`true` = passed quorum and majority).
    public fun execution_result(proposal: &Proposal): bool {
        proposal.execution_result
    }

    /// The creator's escrowed deposit amount for this proposal.
    public fun deposit_of(proposal: &Proposal): u64 {
        proposal.deposit
    }

    // ========================================================================
    // Admin Functions
    // ========================================================================

    #[allow(unused_variable)]
    /// Pause governance (emergency stop)
    public entry fun pause(cap: &GovernanceCap, state: &mut GovernanceState) {
        state.is_paused = true;
    }

    #[allow(unused_variable)]
    /// Unpause governance
    public entry fun unpause(cap: &GovernanceCap, state: &mut GovernanceState) {
        state.is_paused = false;
    }

    #[allow(unused_variable)]
    /// Update the default voting period for new proposals
    public entry fun set_voting_period(
        cap: &GovernanceCap,
        state: &mut GovernanceState,
        period: u64
    ) {
        assert!(period > 0, E_INVALID_VOTING_PERIOD);
        state.voting_period = period;
    }

    #[allow(unused_variable)]
    /// Update the quorum threshold (basis points, at most 100%)
    public entry fun set_quorum_threshold(
        cap: &GovernanceCap,
        state: &mut GovernanceState,
        threshold: u64
    ) {
        assert!(threshold <= 10000, E_INVALID_THRESHOLD);
        state.quorum_threshold = threshold;
    }

    #[allow(unused_variable)]
    /// Maintain `total_supply` for quorum math until a supply oracle exists.
    public entry fun set_total_supply(
        cap: &GovernanceCap,
        state: &mut GovernanceState,
        supply: u64
    ) {
        state.total_supply = supply;
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    /// Index of `voter`'s stake, or `vector::length(stakes)` when absent.
    fun find_stake_index(stakes: &vector<VoteStake>, voter: address): u64 {
        let i = 0;
        let n = vector::length(stakes);
        while (i < n) {
            let stake = vector::borrow(stakes, i);
            if (stake.voter == voter) {
                return i
            };
            i = i + 1;
        };
        n
    }

    // ========================================================================
    // Error Codes
    // ========================================================================

    const E_GOVERNANCE_PAUSED: u64 = 0;
    const E_EMPTY_TITLE: u64 = 1;
    const E_TITLE_TOO_LONG: u64 = 2;
    const E_DESCRIPTION_TOO_LONG: u64 = 3;
    const E_INVALID_VOTING_PERIOD: u64 = 4;
    const E_PROPOSAL_ALREADY_EXECUTED: u64 = 5;
    const E_VOTING_ENDED: u64 = 6;
    const E_VOTING_NOT_ENDED: u64 = 7;
    const E_ZERO_AMOUNT: u64 = 9;
    const E_INVALID_THRESHOLD: u64 = 10;
    const E_ALREADY_VOTED: u64 = 11;
    const E_NO_STAKE: u64 = 12;
    const E_NOT_CREATOR: u64 = 13;
    const E_DEPOSIT_TOO_LOW: u64 = 14;
    const E_DEPOSIT_RETURNED: u64 = 15;
    const E_PROPOSAL_NOT_EXECUTED: u64 = 16;

    // ========================================================================
    // Test-only helpers
    // ========================================================================

    #[test_only]
    /// A fresh `GovernanceState` for tests; shared objects cannot be fetched
    /// from a unit-test context, so tests build the state directly.
    public fun new_state_for_testing(ctx: &mut TxContext): GovernanceState {
        GovernanceState {
            id: object::new(ctx),
            next_proposal_id: 0,
            total_proposals: 0,
            total_supply: 0,
            voting_period: DEFAULT_VOTING_PERIOD,
            quorum_threshold: QUORUM_THRESHOLD,
            is_paused: false
        }
    }

    #[test_only]
    /// A fresh `GovernanceCap` for tests. Has no `drop` — tests must dispose
    /// of it (e.g. `transfer::public_freeze_object`).
    public fun new_cap_for_testing(ctx: &mut TxContext): GovernanceCap {
        GovernanceCap {
            id: object::new(ctx)
        }
    }

    #[test_only]
    /// A fresh `Proposal` for tests. `create_proposal` shares the object, which
    /// a unit test cannot fetch back, so tests build proposals directly. The
    /// `deposit` coin is escrowed as the proposal deposit.
    public fun new_proposal_for_testing(
        proposal_id: u64,
        creator: address,
        deposit: Coin<KANARI>,
        created_at: u64,
        voting_period: u64,
        ctx: &mut TxContext
    ): Proposal {
        Proposal {
            id: object::new(ctx),
            proposal_id,
            creator,
            title: string::utf8(b"Test Proposal"),
            description: string::utf8(b"For testing"),
            yes_votes: 0,
            no_votes: 0,
            total_staked: 0,
            stakes: vector::empty<VoteStake>(),
            escrow: coin::into_balance(deposit),
            deposit: MIN_PROPOSAL_BALANCE,
            deposit_returned: false,
            created_at,
            voting_period,
            executed: false,
            execution_result: false
        }
    }

}
