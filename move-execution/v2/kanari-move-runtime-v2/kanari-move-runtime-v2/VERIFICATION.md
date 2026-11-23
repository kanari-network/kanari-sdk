# Verification scope and attestation

**Status: scoped attestation, not a whole-runtime proof.** Last verified
2026-11-23.

This document records exactly what is machine-checked, what is modelled and
tested, and what is explicitly out of scope for the Lean work in this
directory. It exists so that the result is never over-claimed.

## 1. The bounded claim

> For the **supply-relevant path** of `StateManager::apply_changeset`, every
> state returned by the validated pipeline satisfies the runtime bound
>
> ```text
> ∀ T ∈ tracked,
>   wallet_visible_supply(T) ≤ total_supply(T)
>     ∧ wallet_visible_supply(T) + object_locked_supply(T) ≤ total_supply(T)
> ```
>
> provided the preconditions in §3 hold. This is a theorem about the Lean
> model of those phases, corroborated by a differential test against the real
> Rust per-token check. It is not a claim about code outside
> `KanariMoveRuntime`.

A separate, independent machine-checked result covers the scheduling
algorithm in `src/scheduler.rs`: the waves produced by
`TransactionScheduler::schedule` partition the input in order, are non-empty,
respect `MAX_SPECULATIVE_WAVE_SIZE = 64`, and never group transactions that
share a conflict key. This is a theorem about a faithful model of the
`schedule` function, not about runtime concurrency or linearizability.

A related model-level result covers the scheduler's wave consumer loop. Under
`EngineOK` (extensional read/write manifest completeness) and `Nodup` waves,
`engineFold_eq_seqFold` proves that the modelled commit-or-replay loop equals
the strict serial fold. It is a conditional equivalence about the model; it
does not prove that Rust manifests are complete, that epoch fences or the VM
are correct, or that the runtime is concurrent/linearizable.

A separate correspondence artifact records 21 file/line anchors over the
supply-relevant parts of `state/apply` and `state/supply`, mirrors Rust's
derived tracked set and locked-record sums, and reduces the mirrored phase
order to `applyFull_safe`. A drift checker detects shifted or deleted anchors.
This is a line-anchored Lean mirror, not a proof about Rust execution or about
code outside the declared ranges.

The allowed one-sentence summary is:

> _Inside `KanariMoveRuntime`, the supply-relevant apply path is covered by
> machine-checked Lean proofs and a Lean/Rust differential test for the
> per-token bound, a drift-checked line-anchored mirror records the Rust apply
> correspondence, the `schedule` wave-partition algorithm is machine-checked,
> and the modelled wave consumer loop is conditionally proved equivalent to
> the serial fold._

The following claims are **not** made:

- `kanari-move-runtime-v2` is fully formally verified.
- Every Rust line in the apply path is proof-carrying.
- The opaque `hcover` predicate in `Supply/Store.lean` has been globally
  eliminated; only its concrete `CoverInv` replacement for modelled normal
  write paths is proved.
- The Rust engine, epoch fence, VM, or runtime concurrency is formally
  verified.

## 2. What is machine-checked

All items below are `sorry`/`admit`/`axiom`-free and built by Lean
`v4.37.0-rc1` with Mathlib pinned in `lake-manifest.json`.

| Area                  | Artifact                                 | Checked statement                                                                                                                                                            |
| --------------------- | ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Per-token bound       | `Supply/Store.lean`                      | `rustTokenBound_sound`, `rustTokenBound_complete`: `rustTokenBound` mirrors `validate_token_supply_bound`, including `u64::checked_add` overflow rejection                   |
| Cap-based mint/burn   | `Supply/Proof.lean`, `Supply/Trace.lean` | `mint_preserves`, `Ledger.burn_preserves`, `coinSupplyOf_transfer_pair`, `reachable_invariant`                                                                               |
| Negative case         | `Supply/Counterexample.lean`             | `fundOutside_breaks`, `invariant_fails`, `check_rejects`                                                                                                                     |
| Native apply          | `Runtime/ApplyProof.lean`                | `applyChangesetNative_preserves`, `applyChangesetValidated_safe`: checked `i128` net delta, one checked `u64` total update, covered owner debits, full bound                 |
| Gate                  | `Runtime/FullApplyProof.lean`            | `fullGate_sound`, `fullGate_complete`                                                                                                                                        |
| Treasury caps         | `Runtime/FullApplyProof.lean`            | `writeCap_nativeCapAgrees`, `writeCaps_nativeCapAgrees`                                                                                                                      |
| Gas object            | `Runtime/FullApplyProof.lean`            | `debitCoin_spec`, `debitCoin_none_of_insufficient`, `applyGas_walletCache_eq_sum`, `applyGas_neg_le`                                                                         |
| Native bridge         | `Runtime/FullApplyProof.lean`            | `applyNativePhase_gas_bound`, `applyNativePhase_nativeCapAgrees`, `writeNativeTotal_nativeCapAgrees`                                                                         |
| Locked reconciliation | `Runtime/FullApplyProof.lean`            | `reconcileLocked_bound`, `reconcileAll_preserves_bound` using delta `(issued_after - issued_before) - (visible_after - visible_before)`                                      |
| Legacy repair         | `Runtime/FullApplyProof.lean`            | `applyLegacyRepair_preserves_bound`                                                                                                                                          |
| Non-supply writes     | `Runtime/FullApplyProof.lean`            | `applyNonSupply_preserves_bound`                                                                                                                                             |
| Composition           | `Runtime/FullApplyProof.lean`            | `applyFullCore_preserves`, `applyFull_eq_repaired_of_bound`, **`applyFull_safe`**                                                                                            |
| Scheduler             | `Runtime/SchedulerProof.lean`            | `schedule_flatten`, `schedule_waves_nonempty`, `schedule_wave_size`, `schedule_wave_disjoint` — properties of the model of `src/scheduler.rs` (no runtime concurrency claim) |
| Cover                 | `Supply/Cover.lean`                      | `CoverInv`, `coveredCheck_sound`, and preservation under modelled treasury/native/wallet/locked writes — concrete startup replacement for opaque `hcover`                    |
| Engine loop           | `Runtime/EngineLoop.lean`                | `engineFold_eq_seqFold` — conditional equality of modelled wave execution and serial fold under `EngineOK` and `Nodup` waves                                                 |
| Rust apply anchors    | `Runtime/RustApplyCorrespondence.lean`   | `rustAnchors_wellFormed`, `rustApplyFull_safe`, `rustApplyFull_safe_derived` — 21 line anchors, derived tracked set, and reduction to `applyFull_safe`                       |

`applyFull_safe` is the terminal guarantee: if the validated full pipeline
returns a state, that state passes the modelled runtime supply check.
`coveredCheck_sound` is the cover-side guarantee: over the concrete derived
tracked set, acceptance bounds every token with a persisted cap, provided
`CoverInv` holds on the startup database and normal writes preserve it.
`engineFold_eq_seqFold` is conditional: it gives equality of modelled wave
execution and serial execution only when `EngineOK` and `Nodup` are supplied.
`rustApplyFull_safe` is only a reduction: if the mirrored Rust phase order
returns a state, the existing `applyFull_safe` theorem supplies the bound. It
does not establish that the Rust source, compiler, DB, VM, or runtime semantics
match the mirror.

## 3. Preconditions and modelling assumptions

The bounded claim holds only together with these conditions:

1. **Cover of persisted caps.** `Supply/Store.lean` still records the legacy
   opaque predicate
   ```lean
   hcover : ∀ (T : TokenType) (c : TreasuryCap), capOf T = some c → T ∈ tracked
   ```
   for its original checker theorem. `Supply/Cover.lean` replaces it for a
   concrete `TrackedStore` with `CoverInv`, a checkable invariant over the
   finite inputs Rust unions into `tracked_fungible_token_types`:
   `treasury_index`, wallet-cache keys, object-locked keys and `GAS_COIN`.
   `coveredCheck_sound` needs no opaque cover hypothesis, and
   `writeTreasuryCap_preserves_CoverInv`,
   `writeNativeSupply_preserves_CoverInv`,
   `addWalletKey_preserves_CoverInv` and `addLockedKey_preserves_CoverInv`
   prove preservation under the modelled normal writes. The residual risk is
   therefore a startup/API check: legacy databases with an unindexed
   `supply:<T>` cap, or changesets that write `supply:` keys through the
   generic `move_writes` escape hatch, can violate `CoverInv` and are not
   covered by this theorem.
2. **Tracked set contains the gas coin.** `applyLegacyRepair_preserves_bound`
   and the cached checker assume `gasCoin ∈ tracked`.
3. **Input plausibility.** The changeset model receives the same supply inputs
   Rust computes: owner deltas, created cap writes, gas-object change, the
   canonical candidate-clone index value (`nativeIndexVisible`), and the
   pre-apply snapshots used for reconciliation.
4. **Checked arithmetic.** Native net deltas fit `i128`; totals, caps, and
   locked amounts fit `u64`; rejected overflow is modeled as `none`.
5. **Gate correspondence.** `fullGate` models
   `validate_cached_supply_invariants`; persisted native supply agreement is
   checked by `validate_persisted_native_supply` in the full Rust path.
6. **Supply-field fidelity.** `FullState` encodes the fields that affect the
   bound. Other fields are present to prove write phases supply-neutral, not
   to verify their exact Rust serialization.

## 4. What is modelled inside `KanariMoveRuntime` only

Non-supply writes (index, Move-write and access-version fields in `FullState`)
are represented explicitly and proved supply-neutral by
`applyNonSupply_preserves_bound`. Their concrete byte layout, serialization and
storage behaviour are **not** part of this Lean library.

The scheduler model is also inside `KanariMoveRuntime`, but it proves only the
properties of the single-threaded `schedule` function listed in §2. It does
not prove anything about parallel execution, conflict retries, or
linearizability of the engine that consumes the waves.

`Runtime/EngineLoop.lean` models that consumer as commit-or-replay and proves
`engineFold_eq_seqFold` only under `EngineOK` and `Nodup` waves. `EngineOK` is
an extensional locality hypothesis: executions agree whenever they agree on
their declared read keys, and every key actually written appears in the
declared write set. It is not a proof that Rust produces complete manifests,
that epoch fences or the VM behave correctly, or that the retry policy is
linearizable. The modelled fallback is serial replay of an anomalous wave,
not a proof about real concurrent execution.

`Runtime/RustApplyCorrespondence.lean` is inside `KanariMoveRuntime` only. Its
anchors cover the supply-relevant ranges of `state/apply` and `state/supply`;
profiling, logging, error-message text, RocksDB/overlay plumbing, Move VM and
entry loading, serialization beyond the modelled balance/cap extraction,
event/NFT/collection/module effects, the sibling batch-burn API
(`mod.rs:667-857`), and all concurrency claims are outside the mirror. The
drift checker verifies that anchors still point at live source ranges, but it
cannot prove the initial transcription faithful.

No claim is made here about anything outside `KanariMoveRuntime`.

The Rust test run below is only a cross-check of the per-token bound, not a
claim about other runtime components:

```text
cargo test -p kanari-move-runtime-v2 --no-fail-fast
  => 208 passed; 0 failed
     (144 unit + 64 integration + 0 doc)
```

## 5. Differential test

`testdata/token_bound_vectors.tsv` contains 216 rows generated by
`scripts/GenerateVectors.lean`. Each row is
`total_supply <TAB> wallet_visible <TAB> object_locked <TAB> accept`, where
`accept = 1` exactly when the Lean `rustTokenBound` accepts.

`src/state/supply/supply_validation.rs::tests::lean_vectors_match_rust`
(included under `#[cfg(test)]`) asserts that the real
`StateManager::validate_token_supply_bound` accepts exactly the same rows,
including `u64::MAX` overflow cases. The Rust runtime path is unchanged; only
a test module was added.

## 6. Reproduction

From this directory (`.../kanari-move-runtime-v2/kanari-move-runtime-v2`):

```powershell
lake build
# Build completed successfully (1235 jobs), 0 errors, 0 warnings

lake env lean --run scripts/CheckRustAnchors.lean
# Rust anchor check passed: 21 anchors

lake env lean --run scripts/GenerateVectors.lean
# 216 rows
```

From the workspace root (`D:\move-jamae\kanari-sdk`):

```powershell
cargo test -p kanari-move-runtime-v2 --lib lean_vectors_match_rust
# 1 passed; 0 failed

cargo test -p kanari-move-runtime-v2 --no-fail-fast
# 208 passed; 0 failed

rustfmt --edition 2024 --check `
  move-execution/v2/kanari-move-runtime-v2/src/state/supply/supply_validation.rs
# clean
```

Also checked:

```powershell
# no sorry/admit/axiom declarations under KanariMoveRuntime/
# (historical mentions in Supply/Proof.lean comments describe removed holes)
# all .lake/packages/** working trees clean
```

## 7. How to cite this work

Use:

> Machine-checked Lean proofs cover the supply-relevant apply path
> (`Runtime.ApplyProof`, `Runtime.FullApplyProof`) with theorem `applyFull_safe`,
> the concrete startup cover invariant (`Supply.Cover`), the `schedule`
> wave-partition algorithm (`Runtime.SchedulerProof`), and the conditional
> wave-engine equivalence (`Runtime.EngineLoop`), plus a 216-vector Lean/Rust
> differential test. The remainder of `kanari-move-runtime-v2` is covered by
> the Rust test suite, not formally verified.

Do **not** shorten this to “the runtime is formally verified”.
