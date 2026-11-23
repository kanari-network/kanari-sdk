module

public import KanariMoveRuntime.Runtime.ApplyProof

/-!
# Supply model of the sibling batch-native-burn fast path

This module mirrors the supply arithmetic of
`StateManager::try_apply_owned_native_burn_batch_without_supply_validation`
(`src/state/apply/mod.rs:667-857`) inside `KanariMoveRuntime`.

The Rust function is an eligibility-gated fast path: if any changed object,
gas reference, owner delta, or metadata field fails a structural predicate it
returns `Ok(false)` and the normal validated apply path must be used. For an
eligible batch it folds the owner deltas with checked `i128`, rejects a positive
net supply delta, burns once with a checked `u64` subtraction from the native
total and from `global_token_supplies[GAS_COIN]`, debits owner balances with
checked `u64`, writes the gas-coin objects, and advances access versions.

Only supply-relevant state is represented here. The theorems are about this
Lean model; they do not prove the Rust byte layout, object versions, indexes,
or the sibling function's eligibility predicates. The `batchBurnAnchors` list in
`Runtime.RustApplyCorrespondence` lets `scripts/CheckRustAnchors.lean` detect
drift in the mirrored source ranges. -/

@[expose] public section

namespace KanariMoveRuntime
namespace Runtime
namespace BatchBurn

open KanariMoveRuntime
open KanariMoveRuntime.Supply

/-- Supply-relevant summary of one candidate batch changeset. Each Boolean is
a mirror of one Rust eligibility gate; `deltas` is the already aggregated owner
delta map. -/
structure BatchBurnChangeSet where
  success : Bool
  forbiddenExtras : Bool
  oneGasObject : Bool
  oneCreatedObject : Bool
  canonicalGasObject : Bool
  nativeCoinObject : Bool
  paymentOwnerMatches : Bool
  noModulesAdded : Bool
  senderIsDebit : Bool
  gasCreditLeDebit : Bool
  objectBalanceCoversDebit : Bool
  objectWriteSucceeds : Bool
  deltas : List Int

/-- Conjunction of all Rust eligibility gates. -/
def batchBurnEligible (cs : BatchBurnChangeSet) : Bool :=
  cs.success && !cs.forbiddenExtras && cs.oneGasObject && cs.oneCreatedObject &&
    cs.canonicalGasObject && cs.nativeCoinObject && cs.paymentOwnerMatches &&
    cs.noModulesAdded && cs.senderIsDebit && cs.gasCreditLeDebit &&
    cs.objectBalanceCoversDebit && cs.objectWriteSucceeds

/-- Rust's `burn_amount = u64::try_from(supply_delta.unsigned_abs())` for a
non-positive net delta. -/
def burnAmount (net : Int) : Nat :=
  if net < 0 then (-net).toNat else 0

theorem burnAmount_of_neg {net : Int} (h : net < 0) : burnAmount net = (-net).toNat := by
  simp [burnAmount, h]

theorem burnAmount_of_nonneg {net : Int} (h : ¬ net < 0) : burnAmount net = 0 := by
  simp [burnAmount, h]

/-- The fast-path supply transition. Ineligible batches and positive net deltas
are rejected; an eligible non-positive batch reuses the already-proved checked
native apply transition. -/
def batchBurnNative (s : NativeState) (cs : BatchBurnChangeSet) : Option NativeState :=
  if !batchBurnEligible cs then none
  else
    match supplyDeltaChecked cs.deltas with
    | none => none
    | some net =>
        if net > 0 then none else applyChangesetNative s cs.deltas

theorem batchBurnNative_none_of_ineligible (s : NativeState) (cs : BatchBurnChangeSet)
    (h : batchBurnEligible cs = false) : batchBurnNative s cs = none := by
  simp [batchBurnNative, h]

theorem batchBurnNative_none_of_credit (s : NativeState) (cs : BatchBurnChangeSet) {net : Int}
    (hnet : supplyDeltaChecked cs.deltas = some net) (hpos : net > 0) :
    batchBurnNative s cs = none := by
  by_cases h : batchBurnEligible cs
  · simp [batchBurnNative, h, hnet, hpos]
  · simp [batchBurnNative, h]

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- A successful fast path implies the ordinary checked native transition
succeeded. -/
theorem batchBurnNative_eq_applyChangesetNative {s : NativeState} {cs : BatchBurnChangeSet}
    {s' : NativeState} (hres : batchBurnNative s cs = some s') :
    applyChangesetNative s cs.deltas = some s' := by
  unfold batchBurnNative at hres
  by_cases helig : batchBurnEligible cs
  · cases hnet : supplyDeltaChecked cs.deltas with
    | none => simp [helig, hnet] at hres
    | some net =>
        simp [helig, hnet] at hres
        exact hres.2
  · simp [helig] at hres

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- A successful batch has a non-positive net supply delta. -/
theorem batchBurnNative_net_nonpositive {s : NativeState} {cs : BatchBurnChangeSet}
    {s' : NativeState} {net : Int}
    (hres : batchBurnNative s cs = some s')
    (hnet : supplyDeltaChecked cs.deltas = some net) : net ≤ 0 := by
  unfold batchBurnNative at hres
  by_cases helig : batchBurnEligible cs
  · simp [helig, hnet] at hres
    exact hres.1
  · simp [helig] at hres

/-- A successful fast path can only burn or leave the native total unchanged;
it can never mint, matching the `if supply_delta > 0 { return Ok(false) }`
guard in Rust. -/
theorem batchBurnNative_total_nonincreasing {s : NativeState} {cs : BatchBurnChangeSet}
    {s' : NativeState} {net : Int}
    (hres : batchBurnNative s cs = some s')
    (hnet : supplyDeltaChecked cs.deltas = some net) : s'.total ≤ s.total := by
  have happ := batchBurnNative_eq_applyChangesetNative hres
  cases ht : nextTotalSupplyRust s.total net with
  | none => simp [applyChangesetNative, hnet, ht] at happ
  | some total' =>
      cases hb : applyBalances s.balances cs.deltas with
      | none => simp [applyChangesetNative, hnet, ht, hb] at happ
      | some balances' =>
          have hsEq : s' = { s with total := total', balances := balances' } := by
            simpa [applyChangesetNative, hnet, ht, hb] using happ.symm
          have htotalEq : s'.total = total' := by rw [hsEq]
          have hspec : (total' : Int) = (s.total : Int) + net := nextTotalSupplyRust_spec ht
          have hnp := batchBurnNative_net_nonpositive hres hnet
          omega

/-- The fast path preserves the full native bound. -/
theorem batchBurnNative_preserves {s : NativeState} {cs : BatchBurnChangeSet}
    {s' : NativeState} (h : batchBurnNative s cs = some s') (hinv : NativeInv s) :
    NativeInv s' := by
  exact applyChangesetNative_preserves (batchBurnNative_eq_applyChangesetNative h) hinv

/-- The fast path keeps all `u64` fields in range. -/
theorem batchBurnNative_valid {s : NativeState} {cs : BatchBurnChangeSet}
    {s' : NativeState} (h : batchBurnNative s cs = some s') (hv : NativeValid s) :
    NativeValid s' := by
  exact applyChangesetNative_valid (batchBurnNative_eq_applyChangesetNative h) hv

/-- Final safety statement for the modelled fast path. -/
theorem batchBurnNative_safe {s : NativeState} {cs : BatchBurnChangeSet}
    {s' : NativeState} (h : batchBurnNative s cs = some s') (hv : NativeValid s) :
    nativeTokenBound s' = true := by
  exact applyChangesetNative_checked (batchBurnNative_eq_applyChangesetNative h) hv

end BatchBurn
end Runtime
end KanariMoveRuntime
