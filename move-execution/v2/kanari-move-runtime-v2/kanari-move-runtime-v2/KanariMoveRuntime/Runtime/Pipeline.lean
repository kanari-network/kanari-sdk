module

public import KanariMoveRuntime.Supply.Trace
public import Mathlib.Algebra.BigOperators.Group.List.Defs
public import Mathlib.Data.Multiset.MapFold

set_option linter.style.haveILetI false

open KanariMoveRuntime
open KanariMoveRuntime.Supply

/-!
# Kanari Move Runtime v2 — apply-pipeline model (supply control)

`Trace.lean` models supply operations as abstract ledger steps. The real
runtime, `StateManager::apply_changeset` (`state/apply/mod.rs`), additionally
moves the native supply through per-owner deltas: it computes the net supply
delta from `owner_deltas` (`supply_deltas`), updates the native total via
`next_total_supply` (mint on a positive delta, burn with the
`total_supply >= burn_amount` precheck on a negative one), applies each owner
delta with debit validation (`validate_owner_debits`), and relies on the
post-apply check (`validate_cached_supply_invariants`) to reject effects that
break the invariant — the
`usd_shared_cap_mint_pays_out_twice_to_any_recipient` shape.

This module keeps the **component-level** model and the Mathlib demonstrations
for the invariant `Ledger.Inv` (`visible T <= total T` per token type):

* `applyOwnerDelta` — one per-owner native delta as a standalone cap-aware
  mint/burn (`Ledger.mint` / `Ledger.burn`), with its preservation theorems;
* `applyOwnerDeltas` — the fold over such standalone steps;
* `applyOwnerDeltas_allCredits_preserves` — the fold preserves the invariant
  when every delta is a credit;
* `supplyDelta` — the runtime's `supply_deltas` fold;
* `nextTotalSupply` — the runtime's `next_total_supply` decision, with the
  mint/burn cases.

**Important:** `applyOwnerDelta` already changes the cap total via
`Ledger.mint`/`Ledger.burn`. It must **not** be composed with `nextTotalSupply`
as if it only changed the wallet cache — that would count the net delta twice.
The compositional, checked-arithmetic proof of the actual native apply flow is
in `KanariMoveRuntime.Runtime.ApplyProof`: there the owner pass changes only
owner balances, the total is updated exactly once, and the full runtime bound
(`wallet <= total && wallet + locked <= total`) is proved after validation.

Mathlib is used for the algebraic big operator `List.sum`
(`supplyDelta_eq_sum`) and for `Multiset.foldl_induction`
(`supplyDelta_nonneg_of_all_nonneg`), which gives the net-delta nonnegativity
of an all-credit change set as a machine-checked consequence of the same fold
the runtime performs.

Machine-checked Lean 4 with Mathlib.
-/

@[expose] public section

namespace KanariMoveRuntime
namespace Runtime

/-! ## Net native delta and next total -/

/-- Sum of the per-owner native balance deltas: the runtime's
`supply_deltas` fold in `apply_changeset_with_options`
(`owner_deltas.values().try_fold(0i128, ...)`). -/
def supplyDelta (deltas : List Int) : Int :=
  deltas.foldl (fun acc δ => acc + δ) 0

/-- Core fold lemma used to identify the runtime's `foldl` with Mathlib's
`List.sum` (an additive big operator over an `AddCommMonoid`). -/
private lemma foldl_add (l : List Int) (a b : Int) :
    l.foldl (fun acc x => acc + x) (a + b) = a + l.foldl (fun acc x => acc + x) b := by
  induction l generalizing b with
  | nil => simp
  | cons x xs ih =>
      change xs.foldl (fun acc x => acc + x) ((a + b) + x) =
        a + xs.foldl (fun acc x => acc + x) (b + x)
      rw [show (a + b) + x = a + (b + x) by omega]
      exact ih (b + x)

/-- The runtime's `supply_deltas` fold equals Mathlib's `List.sum`. -/
theorem supplyDelta_eq_sum (deltas : List Int) : supplyDelta deltas = deltas.sum := by
  induction deltas with
  | nil => simp [supplyDelta]
  | cons δ ds ih =>
      change ds.foldl (fun acc x : Int => acc + x) (0 + δ) = δ + ds.sum
      have h0 : (0 : Int) + δ = δ := by omega
      rw [h0]
      calc
        ds.foldl (fun acc x : Int => acc + x) δ
            = ds.foldl (fun acc x : Int => acc + x) (δ + 0) := by
                have hz : δ + (0 : Int) = δ := by omega
                rw [hz]
        _ = δ + ds.foldl (fun acc x : Int => acc + x) 0 := foldl_add ds δ 0
        _ = δ + ds.sum := by
              simpa [supplyDelta] using congrArg (fun x => δ + x) ih

/-- Net nonnegativity of an all-credit change set, proved with Mathlib's
`Multiset.foldl_induction`. Addition on `Int` is right-commutative, so the
multiset fold agrees with the runtime's list fold (`Multiset.coe_foldl`). -/
theorem supplyDelta_nonneg_of_all_nonneg (deltas : List Int)
    (h : ∀ δ ∈ deltas, 0 ≤ δ) : 0 ≤ supplyDelta deltas := by
  letI : RightCommutative (fun acc δ : Int => acc + δ) :=
    ⟨by intro b a₁ a₂; omega⟩
  rw [show supplyDelta deltas =
      Multiset.foldl (fun acc δ : Int => acc + δ) 0 (deltas : Multiset Int) by
        simp [supplyDelta, Multiset.coe_foldl]]
  exact Multiset.foldl_induction (fun acc δ : Int => acc + δ) 0
    (fun x => 0 ≤ x) (deltas : Multiset Int)
    (by intro a b ha hb; omega)
    (by omega)
    (by
      intro a ha
      exact h a (by simpa using ha))

/-- Model of `apply_changeset`'s `next_total_supply`: the native total after a
nonzero supply delta — minted when the delta is positive; when negative, the
runtime burns the total and prechecks `total_supply ≥ burn_amount` (modeled as
`total + supply_delta ≥ 0`; `none` when the burn would take the total
negative); the zero case keeps the total unchanged (the runtime passes
`None`). -/
def nextTotalSupply (total : Int) (supply_delta : Int) : Option Int :=
  if supply_delta > 0 then
    some (total + supply_delta)
  else if supply_delta < 0 then
    if total + supply_delta ≥ 0 then some (total + supply_delta) else none
  else
    some total

/-- A positive supply delta sets the native total to `total + supply_delta`
(the runtime's mint path). -/
theorem nextTotalSupply_pos_eq {total : Int} (δ : Int) (hpos : 0 < δ) :
    nextTotalSupply total δ = some (total + δ) := by
  simp only [nextTotalSupply]
  split
  · rfl
  · exfalso; omega

/-- A negative supply delta with the runtime's underflow precheck
(`total + supply_delta ≥ 0`) sets the native total to `total + supply_delta`
(the runtime's burn path). -/
theorem nextTotalSupply_burn_eq {total : Int} (δ : Int) (hlt : δ < 0)
    (hok : total + δ ≥ 0) :
    nextTotalSupply total δ = some (total + δ) := by
  simp only [nextTotalSupply]
  split
  · exfalso; omega
  · rfl

/-! ## Per-owner native deltas -/

/-- Model of one step of the owner-delta pass of `apply_changeset`: a credit
(`δ ≥ 0`) mints to the wallet (`Ledger.mint`); a debit (`δ < 0`) burns from
the wallet — the runtime's `validate_owner_debits` guarantees the debit is
covered by the owner's balance (see `applyOwnerDelta_burn_preserves`). -/
def applyOwnerDelta (l : Ledger) (T : TokenType) (δ : Int) : Ledger :=
  if 0 ≤ δ then l.mint T δ.toNat else l.burn T (-δ).toNat

/-- Minting one owner's native credit preserves the invariant
(`Ledger.mint_preserves`). -/
theorem applyOwnerDelta_mint_preserves (l : Ledger) (T : TokenType) (δ : Int)
    (hge : 0 ≤ δ) (hinv : l.Inv) :
    (applyOwnerDelta l T δ).Inv := by
  dsimp only [applyOwnerDelta]
  split
  · exact Ledger.mint_preserves hinv T δ.toNat
  · exfalso; omega

/-- Burning one owner's native debit preserves the invariant when the debit is
covered by the wallet (`Ledger.burn_preserves`). -/
theorem applyOwnerDelta_burn_preserves (l : Ledger) (T : TokenType) (δ : Int)
    (hlt : δ < 0) (hinv : l.Inv)
    (hcover : (-δ).toNat ≤ l.visible T) :
    (applyOwnerDelta l T δ).Inv := by
  dsimp only [applyOwnerDelta]
  split
  · exfalso; omega
  · exact Ledger.burn_preserves hinv T (-δ).toNat hcover

/-- The pipeline fold of per-owner native deltas: `applyOwnerDeltas l deltas`
applies `applyOwnerDelta` to each `(T, δ)` of the list. -/
def applyOwnerDeltas (l : Ledger) (deltas : List (TokenType × Int)) : Ledger :=
  match deltas with
  | [] => l
  | p :: ps => applyOwnerDeltas (applyOwnerDelta l p.1 p.2) ps

/-- The pipeline of per-owner deltas preserves the invariant when every delta is
a credit — the shape of a runtime change set that mints through the treasury
caps (the debits in a valid change set are covered by the owner balances
validated by `validate_owner_debits`). -/
theorem applyOwnerDeltas_allCredits_preserves :
    ∀ (deltas : List (TokenType × Int)) (l : Ledger), l.Inv →
      (∀ p ∈ deltas, p.2 ≥ 0) → (applyOwnerDeltas l deltas).Inv := by
  intro deltas
  induction deltas with
  | nil =>
      intro l hinv _hcov
      simpa [applyOwnerDeltas] using hinv
  | cons p ps ih =>
      intro l hinv hcov
      have hge : p.2 ≥ 0 := hcov p (List.mem_cons.mpr (Or.inl rfl))
      have hinv' : (applyOwnerDelta l p.1 p.2).Inv :=
        applyOwnerDelta_mint_preserves l p.1 p.2 hge hinv
      exact ih (applyOwnerDelta l p.1 p.2) hinv'
        (fun q hq => hcov q (List.mem_cons.mpr (Or.inr hq)))

end Runtime
end KanariMoveRuntime
