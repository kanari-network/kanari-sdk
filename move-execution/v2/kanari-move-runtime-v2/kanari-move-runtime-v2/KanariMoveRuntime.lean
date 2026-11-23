module

public import KanariMoveRuntime.Model.Balance
public import KanariMoveRuntime.Model.Treasury
public import KanariMoveRuntime.Model.State
public import KanariMoveRuntime.Supply.Statement
public import KanariMoveRuntime.Supply.Proof
public import KanariMoveRuntime.Supply.Trace
public import KanariMoveRuntime.Supply.Counterexample
public import KanariMoveRuntime.Supply.Store
public import KanariMoveRuntime.Supply.Cover
public import KanariMoveRuntime.Runtime.Pipeline
public import KanariMoveRuntime.Runtime.ApplyProof
public import KanariMoveRuntime.Runtime.FullApplyProof
public import KanariMoveRuntime.Runtime.BatchBurnProof
public import KanariMoveRuntime.Runtime.ResidualGuards
public import KanariMoveRuntime.Runtime.RustApplyCorrespondence
public import KanariMoveRuntime.Runtime.SchedulerProof
public import KanariMoveRuntime.Runtime.EngineLoop

/-!
# Kanari Move Runtime v2

Lean 4 formalization of the supply invariant in `kanari-move-runtime-v2`.

Scope: this is a machine-checked model of the **supply-relevant apply path**,
of **`TransactionScheduler::schedule`**, and of the scheduler's wave consumer
loop under explicit locality hypotheses. It also contains a **line-anchored
supply-relevant correspondence** for the Rust apply ranges: named file/line
anchors, a drift checker, and a terminal theorem reducing the mirrored pipeline
to `applyFull_safe`. Supply models and guards cover the sibling batch-burn
fast path, legacy raw supply-key startup checks, and generic Move writes that
would overwrite supply fields. It is not a line-by-line verification of the
whole Rust runtime. The bounded attestation, its preconditions, and the parts
that are only modelled and tested are recorded in `VERIFICATION.md`.

This library formalizes the supply invariant enforced by
`validate_cached_supply_invariants` in
`crates/kanari-move-runtime-v2/src/state/apply/mod.rs`:

    ∀ T ∈ tracked, wallet_visible_supply(T) ≤ total_supply(T)
        ∧ wallet_visible_supply(T) + object_locked_supply(T) ≤ total_supply(T)

which is the invariant that caught `usd_shared_cap_mint_pays_out_twice_to_any_recipient`
in `crates/kanari-core/tests/unit/engine_tests.rs`.

## What is proved (sorry-free)

* `Supply.mint_preserves` — minting through the treasury cap registered for
  `T` increments wallet-visible supply and cap total by the same amount, so
  the invariant is preserved.
* `Supply.coinSupplyOf_transfer_pair` — transfers between wallets of the same
  token type do not change the wallet-visible sum.
* `Supply.tokenBoundHolds_sound`, `Supply.cachedSupplyCheck_sound` — the
  runtime's executable check never accepts a state that violates the
  invariant; `Supply.holds` is the runtime-facing form of that guarantee.
* `Supply.cachedSupplyCheck_complete` — the check accepts every state whose
  tracked tokens satisfy the accounted bound.
* `Supply.reachable_invariant` — the invariant is inductive over an arbitrary
  execution trace of mint / burn / transfer steps.
* `Supply.Ledger.fundOutside_breaks` and `Counterexample.invariant_fails`,
  `Counterexample.check_rejects` — funding a wallet outside the treasury cap
  breaks the invariant and is rejected by the check.
* `Supply.rustCachedSupplyCheck_sound`, `Supply.ofState_check_sound` — the
  same result for the faithful model of `issued_supply_for_token` and
  `u64::checked_add` in `Supply/Store.lean`; `Supply.fallback_check_accepts`
  records that a token with no persisted total has no fixed cap.
* `Supply.Cover.CoverInv`, `Supply.Cover.coveredCheck_sound` and the four
  preservation lemmas — a concrete startup invariant over the finite inputs
  Rust uses to derive `tracked`. `Supply.Cover.coverCheckStartup_sound` turns
  acceptance of an executable startup check into `CoverInv`, and
  `Supply.Cover.coverCheckStartup_raw_bound` bounds legacy raw `supply:<T>`
  caps once they are forced into the derived tracked set. Generic `move_writes`
  supply writes are still
  an API risk; `Runtime.ResidualGuards` provides a guard for them.
* `Runtime.ApplyProof.applyChangesetNative_preserves`,
  `Runtime.ApplyProof.applyChangesetValidated_safe` — the compositional native
  apply proof: checked `i128` net delta, a single checked `u64` total update,
  covered owner debits, and the full runtime bound accepted by the check gate.
* `Runtime.FullApplyProof.reconcileLocked_bound`,
  `Runtime.FullApplyProof.applyFullCore_preserves`,
  `Runtime.FullApplyProof.applyFull_safe` — the supply-relevant full apply
  pipeline: treasury write-back, gas-object adjustment and owner recompute,
  object-locked reconciliation with the signed issued/visible delta, candidate
  legacy repair, supply-neutral index / Move / access-version writes, and
  validate-and-reject.
* `Runtime.BatchBurn.batchBurnNative_safe`,
  `Runtime.BatchBurn.batchBurnNative_total_nonincreasing` — the supply model of
  the sibling batch-native-burn fast path: eligibility gates, checked net delta,
  a positive-delta rejection, and reuse of the already-proved checked native
  transition. Object bytes, versions and index writes are not proved.
* `Runtime.ResidualGuards.guardedApplyMoveWrites_no_critical`,
  `Runtime.ResidualGuards.guardedApplyMoveWrites_preserves_bound` — a guard that
  rejects generic Move writes classified as `total_supply`,
  `global_token_supplies`, `supply:<T>` or `treasury:<T>` keys; successful
  writes are supply-neutral. It depends on a faithful key classifier supplied
  by the caller.
* `Runtime.SchedulerProof.schedule_flatten`,
  `Runtime.SchedulerProof.schedule_waves_nonempty`,
  `Runtime.SchedulerProof.schedule_wave_size`,
  `Runtime.SchedulerProof.schedule_wave_disjoint` — machine-checked properties
  of a faithful model of `src/scheduler.rs`: waves partition the input in
  order, are non-empty, respect `MAX_SPECULATIVE_WAVE_SIZE`, and never group
  transactions with conflicting keys. This is a proof about the model, not a
  line-by-line verification of the Rust source.
* `Runtime.RustApply.rustAnchors_wellFormed`,
  `Runtime.RustApply.batchBurnAnchors_wellFormed`,
  `Runtime.RustApply.rustTreasuryCapWrite_nativeCapAgrees`,
  `Runtime.RustApply.rustLockedReconciliation_preserves_bound`,
  `Runtime.RustApply.rustNonSupplyPass_preserves_bound`,
  `Runtime.RustApply.rustLegacyRepair_preserves_bound`,
  `Runtime.RustApply.rustApplyFull_safe` — a line-anchored mirror of the
  21 apply anchors and 4 batch-burn anchors, with
  `scripts/CheckRustAnchors.lean` detecting shifted anchors. The terminal
  theorem reduces the mirrored pipeline to `applyFull_safe`; it is not a proof
  about Rust execution.
* `Runtime.EngineLoop.engineFold_eq_seqFold` — under `EngineOK` (read/write
  manifest completeness) and `Nodup` waves, the modelled wave engine equals
  the strict serial fold. This is conditional model equivalence, not a proof
  of the Rust engine, epoch fence, VM, concurrency, or linearizability.

`testdata/token_bound_vectors.tsv` is generated by `scripts/GenerateVectors.lean`
and checked against the real Rust `validate_token_supply_bound` by
`src/state/supply/supply_validation.rs::tests::lean_vectors_match_rust`.
`scripts/CheckResidualGuards.lean` additionally checks Lean-model consistency
for native/batch apply, the Move-write guard and the startup cover check; it is
not a Rust differential test.

Note that the invariant is *not* a theorem about arbitrary `State` values:
`Counterexample.invariant_fails` proves the negation for a concrete state.
That is the point of the runtime check, and the formalization records it
explicitly instead of assuming it away.
-/

@[expose] public section

namespace KanariMoveRuntime

end KanariMoveRuntime
