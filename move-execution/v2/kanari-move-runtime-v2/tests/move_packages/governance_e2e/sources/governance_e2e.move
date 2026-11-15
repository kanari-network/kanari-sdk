/// Thin entry/view wrappers over `kanari_governance::governance` so the
/// runtime can drive the genesis-shared `GovernanceState` and the
/// stake-backed voting flow end to end. `TEST_COIN` is the governed token:
/// `create_currency` mints the `TreasuryCap`, `mint_to` hands stake coins to
/// individual voters, and every write below goes through the production
/// entry functions of the `0x3` governance package.
module tester::governance_e2e {

    use std::option;
    use std::string;
    use kanari_system::coin::{Self, Coin, TreasuryCap};
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    use kanari_governance::governance::{Self, GovernanceCap, GovernanceState, Proposal};

    struct TEST_COIN has drop {}

    /// Claim the `TEST_COIN` currency. The treasury cap goes to the sender so
    /// the later calls can mint with it as an owned object input.
    public entry fun create_currency(ctx: &mut TxContext) {
        let sender = kanari_system::tx_context::sender(ctx);
        let (treasury, metadata) = coin::create_currency<TEST_COIN>(
            TEST_COIN {},
            9,
            b"TEST",
            b"Test Coin",
            b"Governance e2e fixture",
            option::none(),
            ctx
        );
        transfer::public_transfer(treasury, sender);
        transfer::public_transfer(metadata, sender);
    }

    /// Mint `amount` stake coins to `recipient`.
    public entry fun mint_to(
        cap: &mut TreasuryCap<TEST_COIN>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        let stake = coin::mint<TEST_COIN>(cap, amount, ctx);
        transfer::public_transfer(stake, recipient);
    }

    /// Mint the proposal deposit and create a proposal. The new shared
    /// `Proposal` object id is read back from the changeset by the harness.
    public entry fun mint_and_propose(
        cap: &mut TreasuryCap<TEST_COIN>,
        state: &mut GovernanceState,
        amount: u64,
        title: vector<u8>,
        description: vector<u8>,
        voting_period: u64,
        ctx: &mut TxContext
    ) {
        let deposit = coin::mint<TEST_COIN>(cap, amount, ctx);
        governance::create_proposal<TEST_COIN>(
            state,
            deposit,
            ctx,
            string::utf8(title),
            string::utf8(description),
            voting_period
        );
    }

    /// Vote by escrowing a previously transferred stake coin. The voter is
    /// the transaction sender, so two senders cast independent stakes.
    public entry fun vote_with_coin(
        stake: Coin<TEST_COIN>,
        state: &mut GovernanceState,
        proposal: &mut Proposal<TEST_COIN>,
        support: bool,
        ctx: &mut TxContext
    ) {
        governance::cast_vote<TEST_COIN>(state, proposal, support, stake, ctx);
    }

    public entry fun execute(
        state: &GovernanceState,
        proposal: &mut Proposal<TEST_COIN>,
        ctx: &mut TxContext
    ) {
        governance::execute_proposal<TEST_COIN>(state, proposal, ctx);
    }

    public entry fun withdraw_stake(
        proposal: &mut Proposal<TEST_COIN>,
        ctx: &mut TxContext
    ) {
        governance::withdraw_stake<TEST_COIN>(proposal, ctx);
    }

    public entry fun withdraw_deposit(
        proposal: &mut Proposal<TEST_COIN>,
        ctx: &mut TxContext
    ) {
        governance::withdraw_deposit<TEST_COIN>(proposal, ctx);
    }

    /// Admin: set the `total_supply` figure used for quorum math.
    public entry fun set_total_supply(
        cap: &GovernanceCap,
        state: &mut GovernanceState,
        supply: u64
    ) {
        governance::set_total_supply(cap, state, supply)
    }

    // ---- views ------------------------------------------------------------

    public fun yes_votes(p: &Proposal<TEST_COIN>): u64 {
        let (yes, _no, _staked) = governance::vote_tally(p);
        yes
    }

    public fun no_votes(p: &Proposal<TEST_COIN>): u64 {
        let (_yes, no, _staked) = governance::vote_tally(p);
        no
    }

    public fun total_staked(p: &Proposal<TEST_COIN>): u64 {
        let (_yes, _no, staked) = governance::vote_tally(p);
        staked
    }

    public fun is_executed(p: &Proposal<TEST_COIN>): bool {
        governance::is_executed(p)
    }

    public fun execution_result(p: &Proposal<TEST_COIN>): bool {
        governance::execution_result(p)
    }

    public fun deposit_of(p: &Proposal<TEST_COIN>): u64 {
        governance::deposit_of(p)
    }

    public fun state_total_supply(state: &GovernanceState): u64 {
        governance::get_total_supply(state)
    }

}