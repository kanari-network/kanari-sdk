
<a name="@Kanari_Governance_Modules_0"></a>

# Kanari Governance Modules


This is the root document for the Kanari Governance module documentation. The Kanari Governance package (published at <code>0x3</code>) provides a staked-voting governance system built on top of the Kanari System token primitives.


<a name="@Overview_1"></a>

## Overview


Kanari Governance lets any coin holder propose and vote on changes with real economic weight: votes are backed by staked coins, not by free-form ballots. A proposal escrows a deposit from its creator, voters lock coins into the escrow as their stake, and the stake is returned once the voting window closes.


<a name="@Features_2"></a>

## Features


- **Staked Voting**: Voting power is proportional to the coins locked behind a vote
- **Proposal Escrow**: Proposals require a minimum deposit, returned after execution
- **Quorum Thresholds**: A proposal only passes when the yes stake reaches a quorum derived from the total supply (basis points)
- **Permissionless Proposals**: Anyone with enough deposit can create a proposal
- **Admin Controls**: Pause/unpause, voting period, quorum threshold, and total supply can be tuned by the <code>GovernanceCap</code> holder
- **Safety**: One stake per voter per proposal, side switching is rejected, stakes unlock only after the voting window closes


<a name="@Index_3"></a>

## Index


-  [`0x3::governance`](governance.md#0x3_governance)



<a name="@Getting_Started_4"></a>

## Getting Started


To use Kanari Governance modules in your Move code, add the following to your <code>Move.toml</code>:

```toml
[dependencies]
KanariGovernance = { local = "../kanari-governance" }
```

Then import the module you need:

```move
use kanari_governance::governance;
```


<a name="@Core_Flow_5"></a>

## Core Flow


1. <code><a href="governance.md#0x3_governance_create_proposal">governance::create_proposal</a></code> — escrow a deposit (at least <code>MIN_PROPOSAL_BALANCE</code>) and open a proposal
2. <code><a href="governance.md#0x3_governance_cast_vote">governance::cast_vote</a></code> — lock coins as stake for (yes) or against (no)
3. <code><a href="governance.md#0x3_governance_execute_proposal">governance::execute_proposal</a></code> — after the voting window closes, pass if yes stake meets quorum
4. <code><a href="governance.md#0x3_governance_withdraw_stake">governance::withdraw_stake</a></code> — reclaim your stake once voting ends
5. <code><a href="governance.md#0x3_governance_withdraw_deposit">governance::withdraw_deposit</a></code> — the creator reclaims the deposit after execution

Read-only helpers (<code>vote_tally</code>, <code>stake_of</code>, <code>is_executed</code>, <code>execution_result</code>, <code>deposit_of</code>, <code>get_quorum_threshold</code>, ...) expose the proposal state to off-chain clients.


<a name="@Support_6"></a>

## Support


For issues and questions, please visit our GitHub repository.


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/kanari-network/kanari-sdk
[Move Book]: https://move-language.github.io/move/
[Governance Module]: governance.md
