module

public import KanariMoveRuntime.Runtime.FullApplyProof
public import KanariMoveRuntime.Supply.Cover

/-!
# Line-anchored supply-relevant Rust apply correspondence

This module does **not** claim that the Rust apply path is formally verified.
Lean's kernel checks Lean terms; a hand-written mirror is not a proof about a
Rust file unless an independent Rust semantics/extractor is supplied. What is
provided here is the strongest honest artifact available inside
`KanariMoveRuntime`:

* explicit `RustAnchor` records naming the Rust file and line range mirrored
  by each supply-relevant step;
* a mirror of the Rust-derived tracked set and object-locked record sums;
* named phase aliases whose bodies are the already machine-checked
  `Runtime.FullApplyProof` phase functions;
* a terminal theorem reducing the mirrored pipeline to `applyFull_safe`.

Included ranges (all under `move-execution/v2/kanari-move-runtime-v2/src/`):

* `state/apply/mod.rs`: 17-44, 62-85, 87-131, 154-177, 198-256, 258-272,
  274-451, 454-559, 562-630, 632-642;
* `state/apply/apply_phases.rs`: 16-282;
* `state/apply/apply_snapshots.rs`: 9-183;
* `state/apply/apply_treasury.rs`: 9-77;
* `state/apply/apply_balances.rs`: 9-122;
* `state/apply/apply_clock.rs`: 9-46;
* `state/supply/supply_validation.rs`: 12-75, 118-150;
* `state/supply/supply_store.rs`: 11-152;
* `state/supply/supply_repairs.rs`: 14-77, 92-118;
* `state/supply/supply_tracking.rs`: 38-140.

The sibling batch-burn fast path (`mod.rs:667-857`) has its own supply model in
`Runtime.BatchBurnProof` plus four range anchors in `batchBurnAnchors`; only its
supply arithmetic is mirrored, not its structural eligibility predicates or
object writes.

Explicitly skipped: profiling/logging/error-message text, RocksDB and overlay
plumbing, Move VM/entry loading, serialization byte layout beyond the already
modelled balance/cap extraction, event/NFT/collection/module effects, and all
concurrency claims.

The theorem `rustApplyFull_safe` is about this mirror and reduces to the
existing `Runtime.FullApplyProof.applyFull_safe`. It is not a theorem that the
Rust compiler, VM, database, or Rust source semantics are correct. -/

@[expose] public section

namespace KanariMoveRuntime
namespace Runtime
namespace RustApply

open KanariMoveRuntime
open KanariMoveRuntime.Supply

/-! ## Machine-readable line anchors -/

/-- A source anchor for one mirrored or explicitly skipped Rust range. The
`marker` is a stable token that must occur inside the range; the drift checker
uses it to detect shifted line numbers. -/
structure RustAnchor where
  file : String
  first : Nat
  last : Nat
  label : String
  marker : String

/-- Entry size guards and the conditional full pre-validation. -/
def anchorEntryGuards : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 17, 44,
    "entry guards and conditional full validation", "pub fn apply_changeset"⟩

/-- Candidate clone, unvalidated core, repair, gate, and commit-back. -/
def anchorValidatedWrapper : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 62, 85,
    "candidate clone / repair / validate / commit", "apply_changeset_with_options"⟩

/-- Checked owner-delta fold and single native-total update. -/
def anchorNativeDelta : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 87, 131,
    "checked i128 delta fold and u64 total update", "let supply_delta"⟩

/-- Gas payer selection and native-balance snapshots. -/
def anchorGasSelection : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 132, 177,
    "gas payer selection and snapshot_native_balances", "snapshot_native_balances"⟩

/-- Owner state pass and global supply cache writes. -/
def anchorOwnerStatePass : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 198, 256,
    "owner balances, supply cache, native total write", "capture_supply_changed"⟩

/-- Treasury/NFT phase. -/
def anchorTreasuryPhase : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 258, 272,
    "treasury cap write-back and NFT hints", "apply_treasury_nft_balances"⟩

/-- Deleted and created object passes. -/
def anchorObjectPasses : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 274, 451,
    "deleted and created object passes", "created_objects"⟩

/-- Explicit gas-object pass. -/
def anchorGasObjectPass : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 454, 559,
    "explicit gas-object balance adjustment", "write_balance_to_object_bytes"⟩

/-- Owner recompute loop. -/
def anchorRecompute : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 569, 620,
    "owner balance recompute", "recompute_token_balances_for_owner"⟩

/-- Locked reconciliation and supply-cache flush. -/
def anchorLockedAndFlush : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 623, 630,
    "locked reconciliation and global supply flush", "global_token_supplies"⟩

/-- Generic Move writes, dynamic fields, access versions. -/
def anchorNonSupplyTail : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 632, 642,
    "move writes / dynamic fields / access versions", "apply_move_writes"⟩

/-- Snapshot and debit validation helpers. -/
def anchorSnapshotHelpers : RustAnchor :=
  ⟨"src/state/apply/apply_phases.rs", 16, 86,
    "native snapshots and debit validation", "validate_owner_debits"⟩

/-- Treasury, deleted objects, indexes, Move writes, dynamic fields. -/
def anchorPhaseHelpers : RustAnchor :=
  ⟨"src/state/apply/apply_phases.rs", 90, 282,
    "treasury / deletes / indexes / move / dynamic fields", "apply_treasury_nft_balances"⟩

/-- Tracked types, visible snapshots, locked-record reconciliation. -/
def anchorSnapshotRecords : RustAnchor :=
  ⟨"src/state/apply/apply_snapshots.rs", 9, 183,
    "supply snapshots and locked records", "reconcile_object_locked_coin_records"⟩

/-- Persisted cap write-back and owned-object id helpers. -/
def anchorTreasuryHelpers : RustAnchor :=
  ⟨"src/state/apply/apply_treasury.rs", 9, 77,
    "persist_treasury_cap_state", "persist_treasury_cap_state"⟩

/-- Reconciliation trigger and object-balance probes. -/
def anchorBalanceProbes : RustAnchor :=
  ⟨"src/state/apply/apply_balances.rs", 9, 122,
    "locked trigger and object balance probes", "needs_object_locked_reconciliation"⟩

/-- Clock and locked-holder type predicates. -/
def anchorClockPredicates : RustAnchor :=
  ⟨"src/state/apply/apply_clock.rs", 9, 46,
    "clock and locked-holder predicates", "is_system_clock_type"⟩

/-- Native cap check, tracked set, per-token bound, cached gate. -/
def anchorValidation : RustAnchor :=
  ⟨"src/state/supply/supply_validation.rs", 12, 150,
    "tracked set, bound check, cached gate", "validate_token_supply_bound"⟩

/-- Supply key construction, native total write, issued supply lookup. -/
def anchorSupplyStore : RustAnchor :=
  ⟨"src/state/supply/supply_store.rs", 11, 152,
    "supply keys and issued supply", "issued_supply_for_token"⟩

/-- Legacy native repair and locked-record load/sum/save. -/
def anchorRepair : RustAnchor :=
  ⟨"src/state/supply/supply_repairs.rs", 14, 118,
    "legacy repair and locked records", "repair_legacy_native_wallet_overcount"⟩

/-- Global supply adjustment, capture, and owner recompute. -/
def anchorSupplyTracking : RustAnchor :=
  ⟨"src/state/supply/supply_tracking.rs", 38, 140,
    "supply capture and owner recompute", "capture_supply_changed"⟩

/-- All included anchors. -/
def rustAnchors : List RustAnchor :=
  [ anchorEntryGuards, anchorValidatedWrapper, anchorNativeDelta, anchorGasSelection,
    anchorOwnerStatePass, anchorTreasuryPhase, anchorObjectPasses, anchorGasObjectPass,
    anchorRecompute, anchorLockedAndFlush, anchorNonSupplyTail, anchorSnapshotHelpers,
    anchorPhaseHelpers, anchorSnapshotRecords, anchorTreasuryHelpers, anchorBalanceProbes,
    anchorClockPredicates, anchorValidation, anchorSupplyStore, anchorRepair,
    anchorSupplyTracking ]

theorem rustAnchors_wellFormed : ∀ a ∈ rustAnchors, a.first ≤ a.last := by
  decide

/-- Batch-burn eligibility gates and the rejected-extras check. -/
def anchorBatchBurnEligibility : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 667, 698,
    "batch-burn entry and structural eligibility gates",
    "try_apply_owned_native_burn_batch_without_supply_validation"⟩

/-- Checked owner-delta and supply-delta fold in the batch fast path. -/
def anchorBatchBurnDeltas : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 700, 770,
    "batch-burn canonical gas, debit checks and checked delta folds",
    "Native owned batch supply delta overflow"⟩

/-- The positive-delta rejection and single checked burn of total and cache. -/
def anchorBatchBurnSupply : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 783, 817,
    "batch-burn net-delta gate, total burn and visible-supply burn",
    "let burn_amount"⟩

/-- Owner balance writes, object writes, index and access-version tail. -/
def anchorBatchBurnCommit : RustAnchor :=
  ⟨"src/state/apply/mod.rs", 819, 856,
    "batch-burn owner recompute, object writes and access versions",
    "advance_access_versions"⟩

/-- All batch-burn anchors. -/
def batchBurnAnchors : List RustAnchor :=
  [anchorBatchBurnEligibility, anchorBatchBurnDeltas, anchorBatchBurnSupply,
    anchorBatchBurnCommit]

theorem batchBurnAnchors_wellFormed : ∀ a ∈ batchBurnAnchors, a.first ≤ a.last := by
  decide

/-- The supply-relevant phases in Rust apply order. -/
inductive RustPhase where
  | entryGuards
  | validatedWrapper
  | nativeDeltaFold
  | nativeTotalWrite
  | ownerStatePass
  | treasuryCapWrite
  | objectPasses
  | gasObjectPass
  | recomputeOwners
  | lockedReconciliation
  | nonSupplyTail
  | legacyRepair
  | cachedGate

/-- Anchor attached to each mirrored phase. -/
def phaseAnchor : RustPhase → RustAnchor
  | .entryGuards => anchorEntryGuards
  | .validatedWrapper => anchorValidatedWrapper
  | .nativeDeltaFold => anchorNativeDelta
  | .nativeTotalWrite => anchorNativeDelta
  | .ownerStatePass => anchorOwnerStatePass
  | .treasuryCapWrite => anchorTreasuryPhase
  | .objectPasses => anchorObjectPasses
  | .gasObjectPass => anchorGasObjectPass
  | .recomputeOwners => anchorRecompute
  | .lockedReconciliation => anchorLockedAndFlush
  | .nonSupplyTail => anchorNonSupplyTail
  | .legacyRepair => anchorRepair
  | .cachedGate => anchorValidation

/-- Phase anchors are a subset of the declared anchor list. -/
theorem phaseAnchor_mem_rustAnchors (p : RustPhase) : phaseAnchor p ∈ rustAnchors := by
  cases p <;> simp [phaseAnchor, rustAnchors]

/-! ## Rust-derived locked records and tracked set -/

/-- Mirror of `ObjectLockedCoinRecord` (`state.rs:164-171`). -/
structure LockedCoinRecord where
  holderObjectId : Nat
  holderType : TokenType
  owner : Nat
  tokenType : TokenType
  amount : Nat

/-- Total of a list of amounts. Kept recursive so the proof needs no big-op import. -/
def sumAmounts : List LockedCoinRecord → Nat
  | [] => 0
  | r :: rs => r.amount + sumAmounts rs

/-- Mirror of `object_locked_supply_for_token` (`supply_repairs.rs:109-118`):
filter by normalized token type and sum the amounts. -/
def lockedSupplyOf (records : List LockedCoinRecord) (T : TokenType) : Nat :=
  sumAmounts (records.filter fun r => r.tokenType = T)

@[simp] theorem sumAmounts_nil : sumAmounts [] = 0 := rfl
@[simp] theorem sumAmounts_cons (r : LockedCoinRecord) (rs : List LockedCoinRecord) :
    sumAmounts (r :: rs) = r.amount + sumAmounts rs := rfl

theorem sumAmounts_append (as bs : List LockedCoinRecord) :
    sumAmounts (as ++ bs) = sumAmounts as + sumAmounts bs := by
  induction as with
  | nil => simp
  | cons a as ih => simp [ih, Nat.add_assoc]

theorem lockedSupplyOf_nil (T : TokenType) : lockedSupplyOf [] T = 0 := rfl

theorem lockedSupplyOf_cons_of_ne {records : List LockedCoinRecord} {r : LockedCoinRecord}
    {T : TokenType} (hne : r.tokenType ≠ T) :
    lockedSupplyOf (r :: records) T = lockedSupplyOf records T := by
  simp [lockedSupplyOf, hne]

theorem lockedSupplyOf_cons_of_eq {records : List LockedCoinRecord} {r : LockedCoinRecord}
    {T : TokenType} (heq : r.tokenType = T) :
    lockedSupplyOf (r :: records) T = r.amount + lockedSupplyOf records T := by
  simp [lockedSupplyOf, heq]

theorem lockedSupplyOf_append (as bs : List LockedCoinRecord) (T : TokenType) :
    lockedSupplyOf (as ++ bs) T = lockedSupplyOf as T + lockedSupplyOf bs T := by
  simp [lockedSupplyOf, sumAmounts_append]

/-- Supply-relevant Rust state with the finite inputs Rust uses to derive
`tracked_fungible_token_types` kept explicit. -/
structure RustState where
  full : FullState
  lockedRecords : List LockedCoinRecord
  treasuryIndex : List TokenType
  walletKeys : List TokenType

/-- The `locked` function of `FullState` must be the checked record sum. -/
def RustStateWF (r : RustState) : Prop :=
  ∀ T : TokenType, r.full.locked T = lockedSupplyOf r.lockedRecords T

/-- Mirror of Rust's derived tracked set: treasury index, wallet cache keys,
object-locked record token types, and the unconditional gas coin. -/
def derivedTracked (r : RustState) : List TokenType :=
  r.treasuryIndex ++ r.walletKeys ++
    r.lockedRecords.map (fun rec => rec.tokenType) ++ [gasCoin]

/-- Default address used only when reconstructing a `TreasuryCap` from the
simplified `FullState.caps` map. -/
def rustDefaultAddress : Address := ⟨"0x0"⟩

/-- View of `FullState` through the faithful `SupplyStore` shape. The owner and
token-type fields are universal placeholders; the map stores only the cap total,
which is the only field `FullState` retains. -/
def rustSupplyStoreOf (r : RustState) : SupplyStore :=
  { nativeTotal := r.full.total
    supplyKey := fun T =>
      match r.full.caps T with
      | some c => some (PersistedSupply.treasury
          { total_supply := c, owner := rustDefaultAddress, tokenType := T })
      | none => none
    supplyKeyRaw := fun _ => none
    walletCache := r.full.walletCache }

/- The corresponding `Supply.Cover.TrackedStore`, used only to identify the
derived tracked set with `Cover.trackedTokens`. -/
def coverStoreOf (r : RustState) : Supply.Cover.TrackedStore :=
  { store := rustSupplyStoreOf r
    treasuryIndex := r.treasuryIndex
    walletKeys := r.walletKeys
    lockedKeys := r.lockedRecords.map (fun rec => rec.tokenType) }

theorem derivedTracked_eq_coverTracked (r : RustState) :
    derivedTracked r = Supply.Cover.trackedTokens (coverStoreOf r) := by
  rfl

theorem gasCoin_mem_derivedTracked (r : RustState) : gasCoin ∈ derivedTracked r := by
  simp [derivedTracked]

theorem derivedTracked_mono {r r' : RustState}
    (hidx : List.Subset r.treasuryIndex r'.treasuryIndex)
    (hw : List.Subset r.walletKeys r'.walletKeys)
    (hl : List.Subset (r.lockedRecords.map (fun rec => rec.tokenType))
      (r'.lockedRecords.map (fun rec => rec.tokenType))) :
    List.Subset (derivedTracked r) (derivedTracked r') := by
  rw [derivedTracked_eq_coverTracked, derivedTracked_eq_coverTracked]
  exact Supply.Cover.trackedTokens_mono hidx hw hl

/-! ## Mirrored phase functions

Each definition below is an alias of the corresponding already-proved
`FullApplyProof` phase. The anchor in the doc comment records the Rust range;
the alias body records the exact Lean transition used by the proof. -/

/-- @rust_mirror `mod.rs:87-131` checked delta fold and native total write. -/
def rustNativeDeltaFold (s : FullState) (cs : ChangeSetModel) : Option FullState :=
  applyNativePhase s cs.ownerDeltas

/-- @rust_mirror `apply_treasury.rs:9-31`, `mod.rs:258-272`, `mod.rs:408-413`. -/
def rustTreasuryCapWrite (s : FullState) (cs : ChangeSetModel) : FullState :=
  writeCaps s cs.capWrites

/-- @rust_mirror `mod.rs:315-355`, `mod.rs:454-559`. -/
def rustGasObjectPass (s : FullState) (cs : ChangeSetModel) : Option FullState :=
  applyGasOption s cs.gas

/-- @rust_mirror `apply_snapshots.rs:117-183`, `mod.rs:623-625`. -/
def rustLockedReconciliation (before after : FullState) (cs : ChangeSetModel) :
    Option (TokenType → Nat) :=
  reconcileAll before after cs.tracked

/-- @rust_mirror `mod.rs:562-567`, `mod.rs:632-642`. -/
def rustNonSupplyPass (s : FullState) (cs : ChangeSetModel) : FullState :=
  applyNonSupply s cs

/-- @rust_mirror `supply_repairs.rs:14-77`, `mod.rs:77`. -/
def rustLegacyRepair (s : FullState) (cs : ChangeSetModel) : FullState :=
  applyLegacyRepair s cs.nativeIndexVisible

/-- @rust_mirror `supply_validation.rs:54-75`, `supply_validation.rs:129-150`. -/
def rustCachedGate (s : FullState) (cs : ChangeSetModel) : Bool :=
  fullGate s cs.tracked

/-- @rust_mirror `supply_validation.rs:54-75` per-token bound. -/
def rustValidateTokenSupplyBound (total wallet locked : Nat) : Bool :=
  Supply.rustTokenBound total wallet locked

theorem rustNativeDeltaFold_eq (s : FullState) (cs : ChangeSetModel) :
    rustNativeDeltaFold s cs = applyNativePhase s cs.ownerDeltas := rfl

theorem rustTreasuryCapWrite_eq (s : FullState) (cs : ChangeSetModel) :
    rustTreasuryCapWrite s cs = writeCaps s cs.capWrites := rfl

theorem rustGasObjectPass_eq (s : FullState) (cs : ChangeSetModel) :
    rustGasObjectPass s cs = applyGasOption s cs.gas := rfl

theorem rustLockedReconciliation_eq (before after : FullState) (cs : ChangeSetModel) :
    rustLockedReconciliation before after cs = reconcileAll before after cs.tracked := rfl

theorem rustNonSupplyPass_eq (s : FullState) (cs : ChangeSetModel) :
    rustNonSupplyPass s cs = applyNonSupply s cs := rfl

theorem rustLegacyRepair_eq (s : FullState) (cs : ChangeSetModel) :
    rustLegacyRepair s cs = applyLegacyRepair s cs.nativeIndexVisible := rfl

theorem rustCachedGate_eq (s : FullState) (cs : ChangeSetModel) :
    rustCachedGate s cs = fullGate s cs.tracked := rfl

theorem rustValidateTokenSupplyBound_ok_iff {total wallet locked : Nat} :
    rustValidateTokenSupplyBound total wallet locked = true ↔
      Supply.rustTokenBound total wallet locked = true := Iff.rfl

/-- The mirrored treasury phase keeps persisted native-cap agreement. -/
theorem rustTreasuryCapWrite_nativeCapAgrees {s : FullState} {cs : ChangeSetModel}
    (h : nativeCapAgrees s) : nativeCapAgrees (rustTreasuryCapWrite s cs) := by
  simpa [rustTreasuryCapWrite] using (writeCaps_nativeCapAgrees (writes := cs.capWrites) h)

/-- The mirrored locked-reconciliation phase preserves the full bound. -/
theorem rustLockedReconciliation_preserves_bound
    {before after : FullState} {cs : ChangeSetModel} {locked' : TokenType → Nat}
    (hcap : nativeCapAgrees after) (hpre : fullBound before cs.tracked)
    (hwa : ∀ T ∈ cs.tracked, after.walletCache T ≤ issuedOf after T)
    (hrec : rustLockedReconciliation before after cs = some locked') :
    fullBound (withLocked after locked') cs.tracked := by
  rw [rustLockedReconciliation_eq] at hrec
  exact reconcileAll_preserves_bound (tracked := cs.tracked) hcap hpre hwa hrec

/-- The mirrored non-supply tail cannot change the bound. -/
theorem rustNonSupplyPass_preserves_bound {s : FullState} {cs : ChangeSetModel}
    {tracked : List TokenType} (h : fullBound s tracked) :
    fullBound (rustNonSupplyPass s cs) tracked := by
  simpa [rustNonSupplyPass] using (applyNonSupply_preserves_bound (cs := cs) h)

/-- The mirrored fit-branch repair preserves the full bound under the explicit
`nativeIndexVisible` fit hypothesis. -/
theorem rustLegacyRepair_preserves_bound {s : FullState} {cs : ChangeSetModel}
    {tracked : List TokenType}
    (hfit : cs.nativeIndexVisible ≤ s.total - s.locked gasCoin)
    (hgas : gasCoin ∈ tracked) (h : fullBound s tracked) :
    fullBound (rustLegacyRepair s cs) tracked := by
  simpa [rustLegacyRepair] using
    (applyLegacyRepair_preserves_bound (tracked := tracked) hfit hgas h)

/-- The gate over the mirrored derived tracked set gives the full bound for
exactly that set. -/
theorem rustCachedGate_sound_of_derived {r : RustState} {cs : ChangeSetModel}
    (htr : cs.tracked = derivedTracked r)
    (hg : rustCachedGate r.full cs = true) :
    fullBound r.full (derivedTracked r) := by
  rw [rustCachedGate_eq] at hg
  rw [htr] at hg
  exact fullGate_sound hg

/-! ## Mirrored Rust apply order and terminal theorem -/

/-- The Rust phase order, expressed with the mirrored aliases. -/
def rustApplyFullCore (s : FullState) (cs : ChangeSetModel) : Option FullState :=
  match rustNativeDeltaFold s cs with
  | none => none
  | some s1 =>
      let s2 := rustTreasuryCapWrite s1 cs
      match rustGasObjectPass s2 cs with
      | none => none
      | some s3 =>
          match rustLockedReconciliation s s3 cs with
          | none => none
          | some locked' =>
              some (rustNonSupplyPass (withLocked s3 locked') cs)

/-- The validated wrapper: core, candidate repair, gate, and commit. -/
def rustApplyFull (s : FullState) (cs : ChangeSetModel) : Option FullState :=
  match rustApplyFullCore s cs with
  | none => none
  | some s0 =>
      let s' := rustLegacyRepair s0 cs
      if rustCachedGate s' cs then some s' else none

theorem rustApplyFullCore_eq_fullApplyCore (s : FullState) (cs : ChangeSetModel) :
    rustApplyFullCore s cs = applyFullCore s cs := rfl

theorem rustApplyFull_eq_fullApply (s : FullState) (cs : ChangeSetModel) :
    rustApplyFull s cs = applyFull s cs := rfl

/-- Terminal correspondence theorem. This is a reduction from the mirrored
pipeline to the existing machine-checked terminal guarantee; it is not a proof
about Rust execution. -/
theorem rustApplyFull_safe {s : FullState} {cs : ChangeSetModel} {s' : FullState}
    (h : rustApplyFull s cs = some s') : fullBound s' cs.tracked := by
  rw [rustApplyFull_eq_fullApply] at h
  exact applyFull_safe h

/-- The same guarantee stated over Rust's derived tracked set. -/
theorem rustApplyFull_safe_derived {r : RustState} {cs : ChangeSetModel} {s' : FullState}
    (htr : cs.tracked = derivedTracked r)
    (h : rustApplyFull r.full cs = some s') :
    fullBound s' (derivedTracked r) := by
  simpa [htr] using (rustApplyFull_safe h)

end RustApply
end Runtime
end KanariMoveRuntime
