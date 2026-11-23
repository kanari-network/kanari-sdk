# kanari-core (Lean model)

Lean 4 model of the Kanari blockchain engine for bug finding and machine-checked
safety facts.

## What is proved

`KanariCore/Safety.lean`, `KanariCore/MempoolSafety.lean` contain no `axiom`
and no `sorry`.

- Checkpoint monotonicity and isolation:
  `makeCheckpoint_seq`, `makeCheckpoint_increases`,
  `makeCheckpoint_preserves_accounts`, `makeCheckpoint_preserves_total`,
  `makeCheckpoint_preserves_objects`
- Transfer safety: `execTransfer_self`, `execTransfer_total`,
  `execTransfer_noop_of_insufficient`, `execTransfer_preserves_checkpoint`,
  `execTransfer_preserves_objects`
- Mint/burn safety: `execMint_total`, `execBurn_total`,
  `execBurn_noop_of_insufficient`, `execMint_preserves_checkpoint`,
  `execBurn_preserves_checkpoint`, object preservation theorems
- `execTx`/`applyTxs` framing: checkpoint/object preservation across txs and
  blocks, `applyTxs_append`, `applyTxs_preserves_checkpoint`,
  `applyTxs_preserves_objects`
- Object-version monotonicity:
  `object_version_positive`, `setObjectVersion_strictly_increases`, and
  preservation of accounts/total/checkpoint
- Finite conservation under scoped address decomposition:
  `conservation_transfer_decomp`, `conservation_mint_decomp`,
  `conservation_burn_decomp`, `conservation_execTx_of_covers`
- Generic block conservation:
  `conservation_applyTxs_of_inv` (caller supplies a step-preserved invariant)
- Admissibility helpers: `TxAdmissible`, `execTx_total_of_admissible`,
  `execBurn_noop_of_not_admissible`
- Mempool admission (`KanariCore/Mempool.lean` + `MempoolSafety.lean`):
  `Admissible`, `admissible_hash_not_pending`, `admissible_sender_cap`,
  `admissible_primary_cap`, `admissible_congestion_cap`,
  `admissible_nonce_not_stale`, `admit_increases_size`, `admit_increases_bytes`,
  `emptyMempool_inv`
- Checkpoint validation (`KanariCore/Checkpoint.lean`):
  `CheckpointValid`, `genesis_valid_on_empty`, `genesis_sequence`
- DAG vertex validation (`KanariCore/Dag.lean`):
  `ValidVertex`, `validVertex_chain`, `validVertex_round`,
  `validVertex_author`, `round_zero_invalid`, `unknown_author_invalid`
- Supply invariants (`KanariCore/Supply.lean`):
  `NativeSupplyInv`, `TokenSupplyInv`, `mint_preserves_token_inv`,
  `burn_preserves_token_inv`, `transfer_preserves_native`,
  `self_transfer_preserves_native`, `gas_fee_preserves_native`,
  `NoLostTokens`, `no_lost_implies_bound`, `checkpoint_supply_exact`,
  `non_mint_no_supply_increase`
- Gas accounting (`KanariCore/Gas.lean`):
  `GasCost`, `gas_cost_monotonic_limit`, `gas_cost_monotonic_price`,
  `gas_cost_zero_limit`, `gas_cost_zero_price`, `sufficient_remainder`
- Persistence/recovery (`KanariCore/Persistence.lean`):
  `SnapshotConsistent`, `MarkerRecoverable`, `marker_advances`,
  `GenesisRetained`, `genesis_in_singleton`
- Concurrency/access counts (`KanariCore/Concurrency.lean`):
  `increment`, `decrement`, `increment_count`, `increment_decrement`,
  `decrement_saturates`, `removeAll_zero`

## Scope / not claimed

This is an abstract model, not a proof of the Rust Kanari engine. It does not
model Move VM semantics, RocksDB internals, real concurrency/linearizability,
signature crypto, or the full Rust execution pipeline. Conservation theorems
are scoped to a finite, duplicate-free address view supplied by the caller.
The new modules (Mempool, Checkpoint, Dag, Supply, Gas, Persistence,
Concurrency) model the _rules_ of those layers as abstract state transitions;
they are not proofs that the Rust implementation follows them.

## Build

```powershell
cd crates/kanari-core/kanari-core
lake build
```
