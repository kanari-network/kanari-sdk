// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Governance module tests.
//
// Voting is stake-based: every `cast_vote` escrows the passed `Coin` inside
// the proposal, so all vote weights below are real locked token amounts minted
// by the test's own `TESTCOIN` currency.
//
// `create_proposal` shares the `Proposal` object, and the shared
// `GovernanceState` cannot be fetched from a unit-test context either, so the
// tests build these objects through the module's `#[test_only]` helpers and
// exercise the logic directly.

#[test_only]
module kanari_governance::governance_tests {

    use std::option;
    use std::string;
    use kanari_governance::governance;
    use kanari_system::coin::{Self, TreasuryCap};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::kanari::KANARI;

    const ALICE: address = @0xA11CE;
    const BOB: address = @0xB0B;


    #[test_only]
    /// One canonical currency per test run: mint coins as needed, then the
    /// treasury must be disposed of (it has no `drop`).
    fun new_treasury(ctx: &mut TxContext): TreasuryCap<KANARI> {
        kanari_system::kanari::new_treasury_for_testing(ctx)
    }

    // ========================================================================
    // create_proposal
    // ========================================================================

    #[test]
    fun test_create_proposal_escrows_minimum_deposit() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);

        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        governance::create_proposal(
            &mut state,
            deposit,
            &mut ctx,
            string::utf8(b"Treasury diversification"),
            string::utf8(b"Move 5% of the treasury into stables"),
            100
        );

        assert!(governance::get_proposal_count(&state) == 1, 0);

        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 14)]
    fun test_create_proposal_rejects_small_deposit() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);

        let deposit = coin::mint<KANARI>(&mut treasury, 9_999, &mut ctx);
        governance::create_proposal(
            &mut state,
            deposit,
            &mut ctx,
            string::utf8(b"Title"),
            string::utf8(b"desc"),
            100
        );

        // Unreachable: the call above aborts. Non-`drop` objects must still be
        // stated so the compiler sees every path dispose of them.
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 1)]
    fun test_create_proposal_rejects_empty_title() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);

        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        governance::create_proposal(
            &mut state,
            deposit,
            &mut ctx,
            string::utf8(b""),
            string::utf8(b"desc"),
            100
        );

        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 0)]
    fun test_create_proposal_aborts_while_paused() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let cap = governance::new_cap_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);

        governance::pause(&cap, &mut state);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        governance::create_proposal(
            &mut state,
            deposit,
            &mut ctx,
            string::utf8(b"Title"),
            string::utf8(b"desc"),
            100
        );

        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 4)]
    fun test_create_proposal_rejects_zero_voting_period() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);

        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        governance::create_proposal(
            &mut state,
            deposit,
            &mut ctx,
            string::utf8(b"Title"),
            string::utf8(b"desc"),
            0
        );

        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    // ========================================================================
    // cast_vote
    // ========================================================================

    #[test]
    fun test_cast_vote_weights_are_escrowed_stakes() {
        // ALICE votes "yes", BOB votes "no": each ctx has its own sender,
        // which becomes the staked voter identity.
        let ctx = tx_context::new_from_hint(ALICE, 0, 0, 0, 0);
        let bob_ctx = tx_context::new_from_hint(BOB, 1, 0, 0, 0);
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            0,
            100,
            &mut ctx
        );

        governance::cast_vote(
            &mut state,
            &mut proposal,
            true,
            coin::mint<KANARI>(&mut treasury, 100, &mut ctx),
            &mut ctx
        );
        governance::cast_vote(
            &mut state,
            &mut proposal,
            false,
            coin::mint<KANARI>(&mut treasury, 40, &mut ctx),
            &mut bob_ctx
        );

        let (yes, no, total) = governance::vote_tally(&proposal);
        assert!(yes == 100, 0);
        assert!(no == 40, 1);
        assert!(total == 140, 2);
        // Escrow holds deposit + both stakes.
        assert!(
            *option::borrow(&governance::stake_of(&proposal, ALICE)) == 100,
            3
        );
        assert!(*option::borrow(&governance::stake_of(&proposal, BOB)) == 40, 4);

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    fun test_cast_vote_top_up_same_side() {
        let ctx = tx_context::new_from_hint(ALICE, 0, 0, 0, 0);
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            0,
            100,
            &mut ctx
        );

        governance::cast_vote(
            &mut state,
            &mut proposal,
            true,
            coin::mint<KANARI>(&mut treasury, 100, &mut ctx),
            &mut ctx
        );
        governance::cast_vote(
            &mut state,
            &mut proposal,
            true,
            coin::mint<KANARI>(&mut treasury, 50, &mut ctx),
            &mut ctx
        );

        let (yes, _no, total) = governance::vote_tally(&proposal);
        assert!(yes == 150, 0);
        assert!(total == 150, 1);
        // Still one stake, now larger.
        assert!(*option::borrow(&governance::stake_of(&proposal, ALICE)) == 150, 2);

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 11)]
    fun test_cast_vote_cannot_switch_sides() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            0,
            100,
            &mut ctx
        );

        governance::cast_vote(
            &mut state,
            &mut proposal,
            true,
            coin::mint<KANARI>(&mut treasury, 100, &mut ctx),
            &mut ctx
        );
        // Same voter flips to "no": aborts instead of silently moving weight.
        governance::cast_vote(
            &mut state,
            &mut proposal,
            false,
            coin::mint<KANARI>(&mut treasury, 10, &mut ctx),
            &mut ctx
        );

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 9)]
    fun test_cast_vote_rejects_zero_amount() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            0,
            100,
            &mut ctx
        );

        // A zero-value coin carries no weight, so it cannot buy a vote.
        governance::cast_vote(
            &mut state,
            &mut proposal,
            true,
            coin::zero<KANARI>(&mut ctx),
            &mut ctx
        );

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 6)]
    fun test_cast_vote_rejects_vote_after_voting_period() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        // `voting_period = 0` means the window closed at creation time, since
        // the test clock (`dummy` context) never advances past 0.
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            0,
            0,
            &mut ctx
        );

        governance::cast_vote(
            &mut state,
            &mut proposal,
            true,
            coin::mint<KANARI>(&mut treasury, 100, &mut ctx),
            &mut ctx
        );

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 0)]
    fun test_cast_vote_aborts_while_paused() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let cap = governance::new_cap_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            0,
            100,
            &mut ctx
        );

        governance::pause(&cap, &mut state);
        governance::cast_vote(
            &mut state,
            &mut proposal,
            true,
            coin::mint<KANARI>(&mut treasury, 100, &mut ctx),
            &mut ctx
        );

        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    // ========================================================================
    // execute_proposal
    // ========================================================================

    #[test]
    fun test_execute_proposal_passes_with_majority() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let bob_ctx = tx_context::new_from_hint(BOB, 1, 0, 0, 0);
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            0,
            100,
            &mut ctx
        );

        governance::cast_vote(
            &mut state,
            &mut proposal,
            true,
            coin::mint<KANARI>(&mut treasury, 600, &mut ctx),
            &mut ctx
        );
        governance::cast_vote(
            &mut state,
            &mut proposal,
            false,
            coin::mint<KANARI>(&mut treasury, 300, &mut ctx),
            &mut bob_ctx
        );

        // Advance the test clock past the voting window end (t=100).
        tx_context::increment_epoch_timestamp(&mut ctx, 100);
        governance::execute_proposal(&state, &mut proposal, &mut ctx);

        assert!(governance::is_executed(&proposal), 0);
        assert!(governance::execution_result(&proposal), 1);

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    fun test_execute_proposal_fails_when_no_votes_win() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let bob_ctx = tx_context::new_from_hint(BOB, 1, 0, 0, 0);
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            0,
            100,
            &mut ctx
        );

        governance::cast_vote(
            &mut state,
            &mut proposal,
            true,
            coin::mint<KANARI>(&mut treasury, 100, &mut ctx),
            &mut ctx
        );
        governance::cast_vote(
            &mut state,
            &mut proposal,
            false,
            coin::mint<KANARI>(&mut treasury, 600, &mut ctx),
            &mut bob_ctx
        );

        // Advance the test clock past the voting window end (t=100).
        tx_context::increment_epoch_timestamp(&mut ctx, 100);
        governance::execute_proposal(&state, &mut proposal, &mut ctx);

        assert!(governance::is_executed(&proposal), 0);
        assert!(!governance::execution_result(&proposal), 1);

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 7)]
    fun test_execute_proposal_aborts_before_voting_ends() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        // Created in the future relative to the (0) test clock, with a long
        // window: execution must wait for the voting period to end.
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            1_000_000,
            1_000,
            &mut ctx
        );

        governance::execute_proposal(&state, &mut proposal, &mut ctx);

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 5)]
    fun test_execute_proposal_cannot_run_twice() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            0,
            0,
            &mut ctx
        );

        governance::execute_proposal(&state, &mut proposal, &mut ctx);
        // Second run aborts: the proposal is already executed.
        governance::execute_proposal(&state, &mut proposal, &mut ctx);

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    fun test_execute_proposal_requires_quorum() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            0,
            100,
            &mut ctx
        );

        // Supply 1_000_000 at the default threshold (1000 bps = 10%) demands
        // 100_000 staked; a lopsided 500 vs 0 still misses quorum, so the
        // proposal executes as *failed* despite winning the vote count.
        let cap = governance::new_cap_for_testing(&mut ctx);
        governance::set_total_supply(&cap, &mut state, 1_000_000);
        governance::cast_vote(
            &mut state,
            &mut proposal,
            true,
            coin::mint<KANARI>(&mut treasury, 500, &mut ctx),
            &mut ctx
        );

        // Advance the test clock past the voting window end (t=100).
        tx_context::increment_epoch_timestamp(&mut ctx, 100);
        governance::execute_proposal(&state, &mut proposal, &mut ctx);

        assert!(governance::is_executed(&proposal), 0);
        assert!(!governance::execution_result(&proposal), 1);

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    // ========================================================================
    // withdraw_stake
    // ========================================================================

    #[test]
    fun test_withdraw_stake_returns_the_escrowed_amount() {
        let ctx = tx_context::new_from_hint(ALICE, 0, 0, 0, 0);
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            0,
            100,
            &mut ctx
        );

        governance::cast_vote(
            &mut state,
            &mut proposal,
            true,
            coin::mint<KANARI>(&mut treasury, 100, &mut ctx),
            &mut ctx
        );
        let (yes, _no, _total) = governance::vote_tally(&proposal);
        assert!(yes == 100, 0);

        // Advance the test clock past the voting window end (t=100).
        tx_context::increment_epoch_timestamp(&mut ctx, 100);
        governance::withdraw_stake(&mut proposal, &mut ctx);

        // The stake (and its weight) is gone...
        assert!(option::is_none(&governance::stake_of(&proposal, ALICE)), 1);
        let (yes, no, total) = governance::vote_tally(&proposal);
        assert!(yes == 0, 2);
        assert!(no == 0, 3);
        assert!(total == 0, 4);
        // ...and only the creator's deposit remains escrowed.
        assert!(governance::deposit_of(&proposal) == 10_000, 5);

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 7)]
    fun test_withdraw_stake_aborts_while_voting_open() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            1_000_000,
            1_000,
            &mut ctx
        );

        governance::withdraw_stake(&mut proposal, &mut ctx);

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 12)]
    fun test_withdraw_stake_aborts_without_a_stake() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            0,
            0,
            &mut ctx
        );

        governance::withdraw_stake(&mut proposal, &mut ctx);

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    // ========================================================================
    // withdraw_deposit
    // ========================================================================

    #[test]
    fun test_withdraw_deposit_returns_deposit_after_execution() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let sender = tx_context::sender(&ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let proposal = governance::new_proposal_for_testing(
            1,
            sender,
            deposit,
            0,
            0,
            &mut ctx
        );

        governance::execute_proposal(&state, &mut proposal, &mut ctx);
        governance::withdraw_deposit(&mut proposal, &mut ctx);

        assert!(governance::deposit_of(&proposal) == 10_000, 0);

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 16)]
    fun test_withdraw_deposit_aborts_before_execution() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let sender = tx_context::sender(&ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let proposal = governance::new_proposal_for_testing(
            1,
            sender,
            deposit,
            1_000_000,
            1_000,
            &mut ctx
        );

        governance::withdraw_deposit(&mut proposal, &mut ctx);

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 13)]
    fun test_withdraw_deposit_only_the_creator() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let treasury = new_treasury(&mut ctx);
        let deposit = coin::mint<KANARI>(&mut treasury, 10_000, &mut ctx);
        let proposal = governance::new_proposal_for_testing(
            1,
            ALICE,
            deposit,
            0,
            0,
            &mut ctx
        );

        governance::execute_proposal(&state, &mut proposal, &mut ctx);
        // The dummy sender is not the creator: the deposit is not theirs.
        governance::withdraw_deposit(&mut proposal, &mut ctx);

        transfer::public_freeze_object(proposal);
        transfer::public_freeze_object(state);
        transfer::public_freeze_object(treasury);
    }

    // ========================================================================
    // Admin functions
    // ========================================================================

    #[test]
    fun test_pause_and_unpause() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let cap = governance::new_cap_for_testing(&mut ctx);

        governance::pause(&cap, &mut state);
        assert!(governance::is_paused(&state), 0);

        governance::unpause(&cap, &mut state);
        assert!(!governance::is_paused(&state), 1);

        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(state);
    }

    #[test]
    fun test_set_voting_period() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let cap = governance::new_cap_for_testing(&mut ctx);

        governance::set_voting_period(&cap, &mut state, 86_400);
        assert!(governance::get_voting_period(&state) == 86_400, 0);

        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(state);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 4)]
    fun test_set_voting_period_rejects_zero() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let cap = governance::new_cap_for_testing(&mut ctx);

        governance::set_voting_period(&cap, &mut state, 0);

        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(state);
    }

    #[test]
    fun test_set_quorum_threshold() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let cap = governance::new_cap_for_testing(&mut ctx);

        governance::set_quorum_threshold(&cap, &mut state, 2_500);
        assert!(governance::get_quorum_threshold(&state) == 2_500, 0);

        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(state);
    }

    #[test]
    #[expected_failure(location = kanari_governance::governance, abort_code = 10)]
    fun test_set_quorum_threshold_rejects_above_100_percent() {
        let ctx = tx_context::dummy();
        let state = governance::new_state_for_testing(&mut ctx);
        let cap = governance::new_cap_for_testing(&mut ctx);

        governance::set_quorum_threshold(&cap, &mut state, 10_001);

        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(state);
    }

}
