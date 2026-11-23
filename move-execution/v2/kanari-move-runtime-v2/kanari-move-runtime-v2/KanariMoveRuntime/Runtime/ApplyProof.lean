module

public import KanariMoveRuntime.Supply.Store
public import Mathlib.Algebra.BigOperators.Group.List.Defs

/-
# Kanari Move Runtime v2 — a real preservation proof for the apply pipeline

This module fixes the two gaps of `Pipeline.lean`:

1. `applyOwnerDelta` there updated `Ledger.total` through `mint`/`burn` while
   `nextTotalSupply` also updated the total — a composed pipeline would double
   count the net delta. Here the owner pass updates **only owner balances**;
   the total is updated **exactly once** by `nextTotalSupplyRust`, matching
   `state/apply/mod.rs` (`supply_deltas` → `next_total_supply` → owner deltas).
2. `Ledger.Inv` only had `visible ≤ total`. The real runtime check
   `validate_token_supply_bound` requires both
   `wallet ≤ total` and `wallet + object_locked ≤ total`, with `u64::checked_add`.
   The invariant here is expressed directly through `rustTokenBound` from
   `Supply/Store.lean`.

Every fallible runtime operation (`i128` checked fold, `u64::try_from`,
`checked_add`, `checked_sub` precheck, owner debit cover) is modeled as an
`Option`. A failed operation leaves no state, so `applyChangesetNative` either
rejects or returns a state satisfying the full bound, proved below.
-/

@[expose] public section

namespace KanariMoveRuntime
namespace Runtime

open KanariMoveRuntime.Supply

/-! ## Fallible arithmetic (the runtime's checked operations) -/

/- `i128` bounds used by `supply_deltas`' `checked_add` (`try_fold(0i128, ...)`). -/
def i128Min : Int := -((2 : Int) ^ 127)
def i128Max : Int := ((2 : Int) ^ 127) - 1

/-- `i128::checked_add`. -/
def checkedAddI128 (a b : Int) : Option Int :=
  if i128Min ≤ a + b ∧ a + b ≤ i128Max then some (a + b) else none

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
theorem checkedAddI128_some {a b c : Int} (h : checkedAddI128 a b = some c) : c = a + b := by
  unfold checkedAddI128 at h
  split at h
  · exact (Option.some.inj h).symm
  · cases h

/-- The runtime's `supply_deltas` fold, including the `i128` overflow check. -/
def supplyDeltaChecked : List Int → Option Int
  | [] => some 0
  | d :: ds =>
      match supplyDeltaChecked ds with
      | none => none
      | some acc => checkedAddI128 acc d

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
theorem supplyDeltaChecked_eq_sum {ds : List Int} {net : Int}
    (h : supplyDeltaChecked ds = some net) : net = ds.sum := by
  induction ds generalizing net with
  | nil =>
      simp [supplyDeltaChecked] at h
      simpa using h.symm
  | cons d ds ih =>
      simp [supplyDeltaChecked] at h
      cases hs : supplyDeltaChecked ds with
      | none => simp [hs] at h
      | some acc =>
          simp [hs] at h
          have hacc : acc = ds.sum := ih hs
          have hnet : net = acc + d := checkedAddI128_some h
          rw [hnet, hacc]
          simp [List.sum_cons]
          omega

/-- `u64::try_from` for a nonnegative `Int`. -/
def u64FromInt (z : Int) : Option Nat :=
  if 0 ≤ z ∧ z ≤ (u64Max : Int) then some z.toNat else none

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
theorem u64FromInt_spec {z : Int} {n : Nat} (h : u64FromInt z = some n) : (n : Int) = z := by
  unfold u64FromInt at h
  by_cases hz : 0 ≤ z ∧ z ≤ (u64Max : Int)
  · simp [hz] at h
    subst n
    exact Int.toNat_of_nonneg hz.1
  · simp [hz] at h

/-- Model of `next_total_supply` in `apply_changeset_with_options`: mint on a
positive delta (with `u64::try_from` and `checked_add`), prechecked burn on a
negative delta (`checked_sub`), no write on zero. The result is the *new total*
on success; `none` means the runtime rejects the change set. -/
def nextTotalSupplyRust (total : Nat) (δ : Int) : Option Nat :=
  if δ > 0 then
    match u64FromInt δ with
    | none => none
    | some amount => if total + amount ≤ u64Max then some (total + amount) else none
  else if δ < 0 then
    match u64FromInt (-δ) with
    | none => none
    | some burn => if burn ≤ total then some (total - burn) else none
  else
    some total

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
theorem nextTotalSupplyRust_spec {total : Nat} {δ : Int} {t' : Nat}
    (h : nextTotalSupplyRust total δ = some t') : (t' : Int) = (total : Int) + δ := by
  unfold nextTotalSupplyRust at h
  by_cases hpos : δ > 0
  · simp [hpos] at h
    cases hu : u64FromInt δ with
    | none => simp [hu] at h
    | some amount =>
        simp [hu] at h
        have hamount : (amount : Int) = δ := u64FromInt_spec hu
        by_cases hle : total + amount ≤ u64Max
        · simp [hle] at h
          cases h
          omega
        · simp [hle] at h
  · by_cases hneg : δ < 0
    · simp [hpos, hneg] at h
      cases hu : u64FromInt (-δ) with
      | none => simp [hu] at h
      | some burn =>
          simp [hu] at h
          have hburn : (burn : Int) = -δ := u64FromInt_spec hu
          by_cases hle : burn ≤ total
          · simp [hle] at h
            cases h
            omega
          · simp [hle] at h
    · simp [hpos, hneg] at h
      cases h
      omega

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
theorem nextTotalSupplyRust_le_u64 {total : Nat} {δ : Int} {t' : Nat}
    (h : nextTotalSupplyRust total δ = some t') (htotal : total ≤ u64Max) : t' ≤ u64Max := by
  unfold nextTotalSupplyRust at h
  by_cases hpos : δ > 0
  · simp [hpos] at h
    cases hu : u64FromInt δ with
    | none => simp [hu] at h
    | some amount =>
        simp [hu] at h
        by_cases hle : total + amount ≤ u64Max
        · simp [hle] at h
          cases h
          exact hle
        · simp [hle] at h
  · by_cases hneg : δ < 0
    · simp [hpos, hneg] at h
      cases hu : u64FromInt (-δ) with
      | none => simp [hu] at h
      | some burn =>
          simp [hu] at h
          by_cases hle : burn ≤ total
          · simp [hle] at h
            cases h
            omega
          · simp [hle] at h
    · simp [hpos, hneg] at h
      cases h
      exact htotal

/-! ## The owner-balance pass (validate_owner_debits + owner deltas) -/

/-- One owner delta: a checked credit or a covered debit. This is the native
balance write in the owner-delta loop plus `validate_owner_debits` in
`apply_phases.rs` (the runtime uses `max(ledger, object)` as cover; here the
owner's balance is that cover). -/
def applyBalanceDelta (b : Nat) (δ : Int) : Option Nat :=
  if 0 ≤ δ then
    if b + δ.toNat ≤ u64Max then some (b + δ.toNat) else none
  else
    if (-δ).toNat ≤ b then some (b - (-δ).toNat) else none

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
theorem applyBalanceDelta_spec {b : Nat} {δ : Int} {b' : Nat}
    (h : applyBalanceDelta b δ = some b') : (b' : Int) = (b : Int) + δ := by
  unfold applyBalanceDelta at h
  split at h
  · split at h
    · cases h
      have hδ : (δ.toNat : Int) = δ := Int.toNat_of_nonneg (by omega)
      omega
    · cases h
  · split at h
    · cases h
      have hcast : ((-δ).toNat : Int) = -δ := Int.toNat_of_nonneg (by omega)
      omega
    · cases h

/-- The owner-delta fold over all owners. `none` means a debit was uncovered
or a credit overflowed — the runtime rejects the change set. -/
def applyBalances : List Nat → List Int → Option (List Nat)
  | [], [] => some []
  | b :: bs, d :: ds =>
      match applyBalanceDelta b d with
      | none => none
      | some b' =>
          match applyBalances bs ds with
          | none => none
          | some rest => some (b' :: rest)
  | _, _ => none

/-- Wallet-visible supply as an `Int` (the cast of the exact `Nat` sum). -/
def balanceSum (l : List Nat) : Int := (l.sum : Int)

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- The owner pass changes the wallet cache by exactly the net delta. This is
the key identity that rules out the double-counting of `Pipeline.lean`. -/
theorem applyBalances_sum {bs : List Nat} {ds : List Int} {bs' : List Nat}
    (h : applyBalances bs ds = some bs') :
    balanceSum bs' = balanceSum bs + ds.sum := by
  induction bs generalizing ds bs' with
  | nil =>
      cases ds with
      | nil =>
          simp [applyBalances] at h
          subst bs'
          simp [balanceSum]
      | cons d ds =>
          simp [applyBalances] at h
  | cons b bs ih =>
      cases ds with
      | nil =>
          simp [applyBalances] at h
      | cons d ds =>
          simp [applyBalances] at h
          cases hb : applyBalanceDelta b d with
          | none => simp [hb] at h
          | some b' =>
              cases hr : applyBalances bs ds with
              | none => simp [hb, hr] at h
              | some rest =>
                  simp [hb, hr] at h
                  subst bs'
                  have hbspec : (b' : Int) = (b : Int) + d := applyBalanceDelta_spec hb
                  have hrspec : (rest.sum : Int) = (bs.sum : Int) + ds.sum := by
                    simpa [balanceSum] using (ih hr)
                  change ((b' + rest.sum : Nat) : Int) =
                    (((b + bs.sum : Nat) : Int) + (d + ds.sum))
                  push_cast
                  omega

/-! ## The composed transition and its invariant -/

/-- The native supply state relevant to `apply_changeset`: authoritative total,
object-locked supply, and the per-owner wallet balances whose sum is the cached
wallet-visible supply (`global_token_supplies[GAS_COIN]`). -/
structure NativeState where
  total : Nat
  locked : Nat
  balances : List Nat

/-- The full runtime bound, expressed through the executable check. -/
def nativeTokenBound (s : NativeState) : Bool :=
  Supply.rustTokenBound s.total s.balances.sum s.locked

/-- Machine-checked form of the bound accepted by `rustTokenBound`. -/
def NativeInv (s : NativeState) : Prop :=
  s.balances.sum ≤ s.total ∧ s.balances.sum + s.locked ≤ s.total

/-- A state with all `u64` fields in range and the bound satisfied. -/
def NativeValid (s : NativeState) : Prop := s.total ≤ u64Max ∧ NativeInv s

/-- The single apply transition: net delta (checked `i128` fold), total update
(once, checked `u64`), owner deltas (checked, covered). -/
def applyChangesetNative (s : NativeState) (deltas : List Int) : Option NativeState :=
  match supplyDeltaChecked deltas with
  | none => none
  | some net =>
      match nextTotalSupplyRust s.total net with
      | none => none
      | some total' =>
          match applyBalances s.balances deltas with
          | none => none
          | some balances' => some { s with total := total', balances := balances' }

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- **Theorem (real preservation).** A successful `applyChangesetNative`
preserves the full bound `wallet ≤ total ∧ wallet + locked ≤ total`. -/
theorem applyChangesetNative_preserves {s : NativeState} {deltas : List Int} {s' : NativeState}
    (h : applyChangesetNative s deltas = some s') (hinv : NativeInv s) : NativeInv s' := by
  simp [applyChangesetNative] at h
  cases hnet : supplyDeltaChecked deltas with
  | none => simp [hnet] at h
  | some net =>
      simp [hnet] at h
      cases ht : nextTotalSupplyRust s.total net with
      | none => simp [ht] at h
      | some total' =>
          simp [ht] at h
          cases hb : applyBalances s.balances deltas with
          | none => simp [hb] at h
          | some balances' =>
              simp [hb] at h
              have hs' : s' = { s with total := total', balances := balances' } := by
                simpa using h.symm
              subst s'
              have hnetsum : net = deltas.sum := supplyDeltaChecked_eq_sum hnet
              have hbalsum : balanceSum balances' = balanceSum s.balances + net := by
                simpa [hnetsum] using applyBalances_sum hb
              have htspec : (total' : Int) = (s.total : Int) + net :=
                nextTotalSupplyRust_spec ht
              have hpre1 : (s.balances.sum : Int) ≤ (s.total : Int) := by
                exact_mod_cast hinv.1
              have hpre2 : (s.balances.sum : Int) + (s.locked : Int) ≤ (s.total : Int) := by
                exact_mod_cast hinv.2
              have hbalsum' : (balances'.sum : Int) = (s.balances.sum : Int) + net := by
                simpa [balanceSum] using hbalsum
              have h1Int : (balances'.sum : Int) ≤ (total' : Int) := by omega
              have h2Int : (balances'.sum : Int) + (s.locked : Int) ≤ (total' : Int) := by
                omega
              have h1 : balances'.sum ≤ total' := by exact_mod_cast h1Int
              have h2 : balances'.sum + s.locked ≤ total' := by exact_mod_cast h2Int
              exact ⟨h1, h2⟩

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- A successful transition keeps the authoritative total within `u64`. -/
theorem applyChangesetNative_total_le_u64 {s : NativeState} {deltas : List Int} {s' : NativeState}
    (h : applyChangesetNative s deltas = some s') (htotal : s.total ≤ u64Max) :
    s'.total ≤ u64Max := by
  simp [applyChangesetNative] at h
  cases hnet : supplyDeltaChecked deltas with
  | none => simp [hnet] at h
  | some net =>
      simp [hnet] at h
      cases ht : nextTotalSupplyRust s.total net with
      | none => simp [ht] at h
      | some total' =>
          simp [ht] at h
          cases hb : applyBalances s.balances deltas with
          | none => simp [hb] at h
          | some balances' =>
              simp [hb] at h
              have hs' : s' = { s with total := total', balances := balances' } := by
                simpa using h.symm
              subst s'
              exact nextTotalSupplyRust_le_u64 ht htotal

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- A successful transition also keeps every `u64` field in range. -/
theorem applyChangesetNative_valid {s : NativeState} {deltas : List Int} {s' : NativeState}
    (h : applyChangesetNative s deltas = some s') (hv : NativeValid s) : NativeValid s' := by
  exact ⟨applyChangesetNative_total_le_u64 h hv.1,
    applyChangesetNative_preserves h hv.2⟩

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- **Theorem (runtime check accepts the result).** If the prestate is valid,
every successful apply yields a state that `validate_token_supply_bound`
accepts. -/
theorem applyChangesetNative_checked {s : NativeState} {deltas : List Int} {s' : NativeState}
    (h : applyChangesetNative s deltas = some s') (hv : NativeValid s) :
    nativeTokenBound s' = true := by
  have hv' := applyChangesetNative_valid h hv
  have hacc := hv'.2.2
  have htot := hv'.1
  have hnoOverflow : s'.balances.sum + s'.locked ≤ u64Max := by omega
  unfold nativeTokenBound
  exact Supply.rustTokenBound_complete hv'.2.1 hv'.2.2 hnoOverflow

/-- The validate-and-reject wrapper: run the apply, then call the runtime check;
reject if either fails. -/
def applyChangesetValidated (s : NativeState) (deltas : List Int) : Option NativeState :=
  match applyChangesetNative s deltas with
  | none => none
  | some s' => if nativeTokenBound s' then some s' else none

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- On a valid prestate the check never rejects a successful apply — the gate
is a real safeguard, not an extra restriction on honest transitions. -/
theorem applyChangesetValidated_eq_of_valid {s : NativeState} {deltas : List Int}
    (hv : NativeValid s) :
    applyChangesetValidated s deltas = applyChangesetNative s deltas := by
  unfold applyChangesetValidated
  cases hn : applyChangesetNative s deltas with
  | none => simp [hn]
  | some s' =>
      have hb := applyChangesetNative_checked hn hv
      simp [hn, hb]

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- **Final safety statement.** A validated apply either rejects, or returns a
state accepted by the runtime bound. -/
theorem applyChangesetValidated_safe {s : NativeState} {deltas : List Int} {s' : NativeState}
    (h : applyChangesetValidated s deltas = some s') : nativeTokenBound s' = true := by
  unfold applyChangesetValidated at h
  cases ha : applyChangesetNative s deltas with
  | none => simp [ha] at h
  | some s'' =>
      simp [ha] at h
      rcases h with ⟨hb, hsEq⟩
      simpa [hsEq] using hb

end Runtime
end KanariMoveRuntime
