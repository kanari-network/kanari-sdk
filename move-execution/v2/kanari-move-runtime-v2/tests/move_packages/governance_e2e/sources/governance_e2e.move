/// Thin entry/view wrappers over `kanari_governance::governance` so the
/// runtime can drive the genesis-shared `GovernanceState` and the
/// stake-backed voting flow end to end. The governed token is Kanari
/// (`0x2::kanari::KANARI`): `mint_to` mints stake coins from the genesis
/// treasury cap, and every write below goes through the production entry
/// functions of the `0x3` governance package.
module tester::governance_e2e {

    use std::string;
    use kanari_system::coin::{Self, Coin, TreasuryCap};
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    use kanari_system::kanari::KANARI;
    use kanari_governance::governance::{Self, GovernanceCap, GovernanceState, Proposal};

    /// Mint `amount` Kanari stake coins to `recipient` from the (genesis)
    /// treasury cap. The sender must be the cap's owner.
    public entry fun mint_to(
        cap: &mut TreasuryCap<KANARI>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        let stake = coin::mint<KANARI>(cap, amount, ctx);
        transfer::public_transfer(stake, recipient);
    }

    /// Create a proposal by escrowing a previously minted deposit coin.
    public entry fun create_proposal_with_coin(
        deposit: Coin<KANARI>,
        state: &mut GovernanceState,
        title: vector<u8>,
        description: vector<u8>,
        voting_period: u64,
        ctx: &mut TxContext
    ) {
        governance::create_proposal(
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
        stake: Coin<KANARI>,
        state: &mut GovernanceState,
        proposal: &mut Proposal,
        support: bool,
        ctx: &mut TxContext
    ) {
        governance::cast_vote(state, proposal, support, stake, ctx);
    }

    public entry fun execute(
        state: &GovernanceState,
        proposal: &mut Proposal,
        ctx: &mut TxContext
    ) {
        governance::execute_proposal(state, proposal, ctx);
    }

    public entry fun withdraw_stake(
        proposal: &mut Proposal,
        ctx: &mut TxContext
    ) {
        governance::withdraw_stake(proposal, ctx);
    }

    public entry fun withdraw_deposit(
        proposal: &mut Proposal,
        ctx: &mut TxContext
    ) {
        governance::withdraw_deposit(proposal, ctx);
    }

    /// Admin: sync `total_supply` from the Kanari treasury cap (supply oracle).
    public entry fun set_total_supply(
        cap: &GovernanceCap,
        state: &mut GovernanceState,
        treasury_cap: &kanari_system::coin::TreasuryCap<KANARI>
    ) {
        governance::set_total_supply(cap, state, treasury_cap)
    }

    // ---- views ------------------------------------------------------------

    public fun yes_votes(p: &Proposal): u64 {
        let (yes, _no, _staked) = governance::vote_tally(p);
        yes
    }

    public fun no_votes(p: &Proposal): u64 {
        let (_yes, no, _staked) = governance::vote_tally(p);
        no
    }

    public fun total_staked(p: &Proposal): u64 {
        let (_yes, _no, staked) = governance::vote_tally(p);
        staked
    }

    public fun is_executed(p: &Proposal): bool {
        governance::is_executed(p)
    }

    public fun execution_result(p: &Proposal): bool {
        governance::execution_result(p)
    }

    public fun deposit_of(p: &Proposal): u64 {
        governance::deposit_of(p)
    }

    public fun state_total_supply(state: &GovernanceState): u64 {
        governance::get_total_supply(state)
    }

}
