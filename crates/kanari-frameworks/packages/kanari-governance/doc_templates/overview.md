# Kanari Governance Modules

This is the root document for the Kanari Governance module documentation. The Kanari Governance package (published at `0x3`) provides a staked-voting governance system built on top of the Kanari System token primitives.

## Overview

Kanari Governance lets any coin holder propose and vote on changes with real economic weight: votes are backed by staked coins, not by free-form ballots. A proposal escrows a deposit from its creator, voters lock coins into the escrow as their stake, and the stake is returned once the voting window closes.

## Features

- **Staked Voting**: Voting power is proportional to the coins locked behind a vote
- **Proposal Escrow**: Proposals require a minimum deposit, returned after execution
- **Quorum Thresholds**: A proposal only passes when the yes stake reaches a quorum derived from the total supply (basis points)
- **Permissionless Proposals**: Anyone with enough deposit can create a proposal
- **Admin Controls**: Pause/unpause, voting period, quorum threshold, and total supply can be tuned by the `GovernanceCap` holder
- **Safety**: One stake per voter per proposal, side switching is rejected, stakes unlock only after the voting window closes

## Index

> {{move-index}}

## Getting Started

To use Kanari Governance modules in your Move code, add the following to your `Move.toml`:

```toml
[dependencies]
KanariGovernance = { local = "../kanari-governance" }
```

Then import the module you need:

```move
use kanari_governance::governance;
```

## Core Flow

1. `governance::create_proposal` — escrow a deposit (at least `MIN_PROPOSAL_BALANCE`) and open a proposal
2. `governance::cast_vote` — lock coins as stake for (yes) or against (no)
3. `governance::execute_proposal` — after the voting window closes, pass if yes stake meets quorum
4. `governance::withdraw_stake` — reclaim your stake once voting ends
5. `governance::withdraw_deposit` — the creator reclaims the deposit after execution

Read-only helpers (`vote_tally`, `stake_of`, `is_executed`, `execution_result`, `deposit_of`, `get_quorum_threshold`, ...) expose the proposal state to off-chain clients.

## Support

For issues and questions, please visit our GitHub repository.
