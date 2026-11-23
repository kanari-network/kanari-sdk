module

public import KanariMoveRuntime.Runtime.FullApplyProof
public import KanariMoveRuntime.Supply.Cover

/-!
# Guards for the remaining supply-relevant residual risks

This module does not change the Rust runtime. It adds Lean-side, machine-checked
guards and theorems for two residual paths called out in `VERIFICATION.md`:

* generic Move writes that overwrite supply-critical keys
  (`apply_phases.rs:241-264`);
* the sibling batch-native-burn fast path
  (`state/apply/mod.rs:667-857`), whose supply arithmetic is modelled in
  `Runtime/BatchBurnProof.lean`.

The guard over Move writes is deliberately abstract: a caller supplies a
`classify` function from opaque store keys to `SupplyWriteClass`. The theorem is
conditional on that classifier being faithful to Rust key construction, which
is stated explicitly rather than assumed silently. -/

@[expose] public section

namespace KanariMoveRuntime
namespace Runtime
namespace ResidualGuards

open KanariMoveRuntime
open KanariMoveRuntime.Supply

/-! ## Supply-critical Move writes -/

/-- Classification of a canonical overlay write key. -/
inductive SupplyWriteClass where
  | totalSupply
  | globalSupplies
  | supplyKey
  | treasuryKey
  | other
  deriving DecidableEq

/-- Only these classes can change a supply total or a persisted cap. -/
def supplyCritical : SupplyWriteClass → Bool
  | .totalSupply => true
  | .globalSupplies => true
  | .supplyKey => true
  | .treasuryKey => true
  | .other => false

theorem any_of_mem_of_pred {α : Type} (p : α → Bool) {xs : List α} {x : α}
    (hx : x ∈ xs) (hp : p x = true) : xs.any p = true := by
  induction xs with
  | nil => simp at hx
  | cons y ys ih =>
      simp only [List.mem_cons] at hx
      rcases hx with rfl | hx
      · simp [hp]
      · simp [ih hx]

/-- Reject a changeset tail containing a supply-critical Move write. Otherwise
apply the writes only to the inert `moveWrites` field. -/
def guardedApplyMoveWrites (s : FullState) (classify : Nat → SupplyWriteClass)
    (writes : List Nat) : Option FullState :=
  if writes.any (fun k => supplyCritical (classify k)) then none
  else some { s with moveWrites := writes }

theorem guardedApplyMoveWrites_rejects_critical (s : FullState)
    (classify : Nat → SupplyWriteClass) {writes : List Nat}
    (hany : writes.any (fun k => supplyCritical (classify k)) = true) :
    guardedApplyMoveWrites s classify writes = none := by
  simp [guardedApplyMoveWrites, hany]

/-- A successful guard never admits a supply-critical key. -/
theorem guardedApplyMoveWrites_no_critical {s : FullState}
    {classify : Nat → SupplyWriteClass} {writes : List Nat} {s' : FullState}
    (hres : guardedApplyMoveWrites s classify writes = some s') :
    ∀ k ∈ writes, supplyCritical (classify k) = false := by
  intro k hk
  cases hval : supplyCritical (classify k) with
  | false => rfl
  | true =>
      have hany : writes.any (fun k => supplyCritical (classify k)) = true :=
        any_of_mem_of_pred (fun k => supplyCritical (classify k)) hk hval
      simp [guardedApplyMoveWrites, hany] at hres

/-- The guard cannot weaken the full bound: it either rejects, or touches only
an inert field. -/
theorem guardedApplyMoveWrites_preserves_bound {s : FullState}
    {classify : Nat → SupplyWriteClass} {writes : List Nat} {tracked : List TokenType}
    (h : fullBound s tracked) :
    ∀ {s'}, guardedApplyMoveWrites s classify writes = some s' → fullBound s' tracked := by
  intro s' hres
  by_cases hany : writes.any (fun k => supplyCritical (classify k))
  · simp [guardedApplyMoveWrites, hany] at hres
  · simp [guardedApplyMoveWrites, hany] at hres
    subst s'
    simpa [fullBound, nativeCapAgrees, issuedOf] using h

end ResidualGuards
end Runtime
end KanariMoveRuntime
