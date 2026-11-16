# Kanari Governance — Usage Guide

On-chain governance for the Kanari network, implemented as a staked-voting
system in the framework package `0x3` (`KanariGovernance`). Every vote is
backed by locked Kanari coins; proposals require an escrowed deposit; execution
requires quorum.

## Overview

| Property              | Value                                    |
| --------------------- | ---------------------------------------- |
| Package address       | `0x3`                                    |
| Module                | `governance`                             |
| Governed coin         | `0x2::kanari::KANARI` (Kanari only)      |
| Decimals              | 9 (1 KANARI = 10^9 MIST)                 |
| Min proposal deposit  | 10,000 KANARI                            |
| Default voting period | 604,800 s (7 days)                       |
| Default quorum        | 1,000 basis points (10% of total supply) |

## Core Concepts

- **Proposal** — a shared object holding title, description, vote tallies,
  per-voter escrowed stakes, the creator's deposit, and execution state.
- **GovernanceState** — shared singleton tracking proposal count, voting
  period, quorum threshold, paused flag, and total supply (for quorum math).
- **GovernanceCap** — capability object held by the admin (created at genesis,
  transferred to the deployer). Required for pause/unpause and parameter
  changes.
- **VoteStake** — a voter's escrowed stake on a proposal (support flag +
  amount). One per voter per proposal; top-up on the same side is allowed,
  switching sides aborts.
- **Staked voting** — `cast_vote` escrows the passed `Coin<KANARI>` into the
  proposal's `escrow: Balance<KANARI>`. The coin becomes the voter's weight.
  Stakes are withdrawable once the voting window closes.

## Lifecycle

```
create_proposal  -->  cast_vote (×N)  -->  execute_proposal  -->  withdraw_stake / withdraw_deposit
   (deposit)          (stake coins)        (after window)        (after window / after execute)
```

1. **Create** — creator escrows a deposit (`>= MIN_PROPOSAL_BALANCE`). The
   proposal is shared; anyone can read and vote.
2. **Vote** — voters stake Kanari coins for (`support = true`) or against
   (`support = false`). Coins are escrowed in the proposal.
3. **Execute** — after the voting window closes, anyone may call
   `execute_proposal`. It passes only if `yes_votes > no_votes` **and**
   `total_staked >= total_supply * quorum_threshold / 10000`.
4. **Withdraw** — voters reclaim their stake (`withdraw_stake`); the creator
   reclaims the deposit (`withdraw_deposit`, only after execution).

## Move API (`0x3::governance`)

### Entry functions

```move
public entry fun create_proposal(
    state: &mut GovernanceState,
    deposit: Coin<KANARI>,
    ctx: &mut TxContext,
    title: String,
    description: String,
    voting_period: u64,
);

public entry fun cast_vote(
    state: &mut GovernanceState,
    proposal: &mut Proposal,
    support: bool,
    coin: Coin<KANARI>,
    ctx: &mut TxContext,
);

public entry fun execute_proposal(
    state: &GovernanceState,
    proposal: &mut Proposal,
    ctx: &mut TxContext,
);

public entry fun withdraw_stake(proposal: &mut Proposal, ctx: &mut TxContext);

public entry fun withdraw_deposit(proposal: &mut Proposal, ctx: &mut TxContext);
```

### Admin functions (require `GovernanceCap`)

```move
public entry fun pause(cap: &GovernanceCap, state: &mut GovernanceState);
public entry fun unpause(cap: &GovernanceCap, state: &mut GovernanceState);
public entry fun set_voting_period(cap, state, period: u64);
public entry fun set_quorum_threshold(cap, state, threshold: u64); // basis points, <= 10000
public entry fun set_total_supply(cap, state, treasury_cap: &TreasuryCap<KANARI>); // supply oracle
```

### View functions

```move
public fun get_proposal_count(state): u64;
public fun get_voting_period(state): u64;
public fun get_quorum_threshold(state): u64;
public fun get_total_supply(state): u64;
public fun is_paused(state): bool;
public fun vote_tally(proposal): (u64, u64, u64);   // (yes, no, total_staked)
public fun stake_of(proposal, voter): Option<u64>;
public fun is_executed(proposal): bool;
public fun execution_result(proposal): bool;
public fun deposit_of(proposal): u64;
```

## CLI Usage (`kanari client governance`)

### Create a proposal

```powershell
kanari client governance create-proposal `
  --state <GOVERNANCE_STATE_ID> `
  --coin <COIN_OBJECT_ID> `
  --title "Treasury diversification" `
  --description "Move 5% of treasury into stables" `
  --voting-period 604800000 `
  --from <ADDR> --password <PWD>
```

### Cast a vote

```powershell
# Vote in favour
kanari client governance cast-vote `
  --state <STATE_ID> --proposal <PROPOSAL_ID> `
  --support --coin <COIN_OBJECT_ID> `
  --from <VOTER> --password <PWD>

# Vote against
kanari client governance cast-vote `
  --state <STATE_ID> --proposal <PROPOSAL_ID> `
  --coin <COIN_OBJECT_ID> `
  --from <VOTER> --password <PWD>
```

### Execute a proposal

```powershell
kanari client governance execute-proposal `
  --state <STATE_ID> --proposal <PROPOSAL_ID> `
  --from <ADDR> --password <PWD>
```

### Withdraw

```powershell
kanari client governance withdraw-stake --proposal <PROPOSAL_ID> --from <VOTER> --password <PWD>
kanari client governance withdraw-deposit --proposal <PROPOSAL_ID> --from <CREATOR> --password <PWD>
```

### Admin operations (require GovernanceCap)

```powershell
kanari client governance pause --cap <CAP_ID> --state <STATE_ID> --from <ADMIN> --password <PWD>
kanari client governance unpause --cap <CAP_ID> --state <STATE_ID> --from <ADMIN> --password <PWD>
kanari client governance set-voting-period --cap <CAP_ID> --state <STATE_ID> --period-ms 86400000 --from <ADMIN> --password <PWD>
kanari client governance set-quorum-threshold --cap <CAP_ID> --state <STATE_ID> --threshold-bps 1500 --from <ADMIN> --password <PWD>
kanari client governance set-total-supply --cap <CAP_ID> --state <STATE_ID> --treasury-cap <TREASURY_CAP_ID> --from <ADMIN> --password <PWD>
```

## Read-only views

```powershell
kanari client view --package 0x3 --module governance --function vote_tally --arg <PROPOSAL_ID>
kanari client view --package 0x3 --module governance --function stake_of --arg <PROPOSAL_ID> --arg <VOTER_ADDR>
kanari client view --package 0x3 --module governance --function is_executed --arg <PROPOSAL_ID>
kanari client view --package 0x3 --module governance --function get_total_supply --arg <STATE_ID>
```

## Using governance in a Move package

### Move.toml

```toml
[dependencies]
KanariGovernance = { local = "../kanari-governance" }
```

### Move code

```move
use kanari_governance::governance;

// Create a proposal (deposit >= 10,000 KANARI)
governance::create_proposal(state, deposit_coin, ctx, title, description, voting_period);

// Vote
governance::cast_vote(state, proposal, true, stake_coin, ctx);

// Execute after window closes
governance::execute_proposal(state, proposal, ctx);

// Withdraw
governance::withdraw_stake(proposal, ctx);
governance::withdraw_deposit(proposal, ctx);
```

## Error Codes

| Code | Constant                      | Meaning                                   |
| ---- | ----------------------------- | ----------------------------------------- |
| 0    | `E_GOVERNANCE_PAUSED`         | Governance is paused                      |
| 1    | `E_EMPTY_TITLE`               | Title is empty                            |
| 2    | `E_TITLE_TOO_LONG`            | Title exceeds 256 chars                   |
| 3    | `E_DESCRIPTION_TOO_LONG`      | Description exceeds 4096 chars            |
| 4    | `E_INVALID_VOTING_PERIOD`     | Voting period is 0                        |
| 5    | `E_PROPOSAL_ALREADY_EXECUTED` | Proposal already executed                 |
| 6    | `E_VOTING_ENDED`              | Voting window closed                      |
| 7    | `E_VOTING_NOT_ENDED`          | Voting window still open                  |
| 9    | `E_ZERO_AMOUNT`               | Vote amount is 0                          |
| 10   | `E_INVALID_THRESHOLD`         | Quorum threshold > 100%                   |
| 11   | `E_ALREADY_VOTED`             | Voter already voted on the other side     |
| 12   | `E_NO_STAKE`                  | Voter has no stake to withdraw            |
| 13   | `E_NOT_CREATOR`               | Only the creator can withdraw the deposit |
| 14   | `E_DEPOSIT_TOO_LOW`           | Deposit below MIN_PROPOSAL_BALANCE        |
| 15   | `E_DEPOSIT_RETURNED`          | Deposit already returned                  |
| 16   | `E_PROPOSAL_NOT_EXECUTED`     | Proposal not yet executed                 |

## Notes

- Governance uses **Kanari coins only** (`0x2::kanari::KANARI`). Other coin types are not supported.
- `total_supply` starts at 0 (quorum passes trivially) until admin calls
  `set_total_supply` with the treasury cap (supply oracle).
- Admin cap (`GovernanceCap`) is created at genesis and transferred to the
  deployer. Only the cap holder can pause/unpause and change parameters.
- For testing: use `kanari move test` in `packages/kanari-governance` (27 tests)
  or the e2e test `governance_vote_flow` (full flow).
