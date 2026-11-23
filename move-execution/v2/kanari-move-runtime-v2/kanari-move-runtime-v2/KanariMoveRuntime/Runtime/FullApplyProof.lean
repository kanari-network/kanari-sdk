module

public import KanariMoveRuntime.Runtime.ApplyProof

/-
# Kanari Move Runtime v2 — supply-relevant proof of the full apply pipeline

`ApplyProof.lean` proves the native owner-delta transition with checked
arithmetic. This module extends the proof to the remaining supply-relevant
phases of `StateManager::apply_changeset` (`src/state/apply/mod.rs`):

* treasury write-back from created `TreasuryCap` objects
  (`apply_treasury.rs::persist_treasury_cap_state`);
* explicit gas-object debit and owner-balance recompute
  (`mod.rs` gas-object pass, `supply_tracking.rs::recompute_token_balances_for_owner`);
* object-locked coin reconciliation
  (`apply_snapshots.rs::reconcile_object_locked_coin_records`);
* the candidate-clone legacy wallet repair
  (`supply_repairs.rs::repair_legacy_native_wallet_overcount`);
* the validate-and-reject gate on the candidate clone
  (`mod.rs::apply_changeset_with_options`, `supply_validation.rs`);
* index / Move-write / dynamic-field / access-version writes, which are modeled
  as explicit fields and proved to leave every supply field unchanged.

The theorem `applyFull_safe` is the runtime-facing guarantee: if the full
pipeline returns a state, that state satisfies the native-agreement check and
the full per-token bound `wallet <= total && wallet + locked <= total`.
-/

@[expose] public section

namespace KanariMoveRuntime
namespace Runtime

open KanariMoveRuntime
open KanariMoveRuntime.Supply

/-! ## Supply-relevant state -/

/-- A coin object reduced to the fields that affect supply accounting. -/
structure CoinObject where
  id : Nat
  owner : Nat
  tokenType : TokenType
  amount : Nat
  isLockedHolder : Bool

/-- Supply-relevant projection of `ObjectLockedCoinRecord`. The runtime keeps a
list of records; `locked` is their checked per-token sum, which is exactly what
`validate_token_supply_bound` consumes. -/
structure FullState where
  /-- Native `state.total_supply`. -/
  total : Nat
  /-- Native owner ledger balances (`OwnerState.native_balance`). -/
  ownerBalances : List Nat
  /-- `global_token_supplies`: cached wallet-visible supply. -/
  walletCache : TokenType → Nat
  /-- Persisted treasury caps; `none` means the fallback cache is used. -/
  caps : TokenType → Option Nat
  /-- Coin objects with a balance-bearing type. -/
  coins : List CoinObject
  /-- Checked sum of object-locked coin records. -/
  locked : TokenType → Nat
  /-- Non-supply canonical state. -/
  indexes : List Nat
  moveWrites : List Nat
  accessVersions : Nat

/-- Parsed treasury cap object from `treasury_cap_token_supply`. -/
abbrev CapWrite := TokenType × Nat

/-- Explicit gas-object adjustment, reduced to supply fields. -/
structure GasChange where
  objectId : Nat
  delta : Int
  debit : Nat
  credit : Nat
  objectAfter : Nat
  priorNonObject : Nat
  ownerIndex : Nat

/-- The changeset fields consumed by the supply-relevant phases. Snapshots are
not stored here: `applyFullCore` captures them from the pre-state, exactly as
`capture_supply_snapshots` does before `apply_changeset`'s write phases. -/
structure ChangeSetModel where
  ownerDeltas : List Int
  capWrites : List CapWrite
  gas : Option GasChange
  tracked : List TokenType
  /-- Canonical native wallet index computed on the candidate clone and used
  by `repair_legacy_native_wallet_overcount`. -/
  nativeIndexVisible : Nat
  /-- Non-supply writes carried by the changeset. -/
  indexes : List Nat
  moveWrites : List Nat
  accessVersions : Nat

/-- Replace the value of a function at one key. -/
def updateAt {α β : Type} [DecidableEq α] (f : α → β) (k : α) (v : β) : α → β :=
  fun k' => if k' = k then v else f k'

/-- `List.getD` with an explicit default, used for owner indexes. -/
def ownerAt (l : List Nat) (i : Nat) : Nat := l.getD i 0

/-- Set an owner balance by index. Out-of-range indexes are extended with zero
entries so `ownerAt (setOwner l i v) i = v` always holds. -/
def setOwner : List Nat → Nat → Nat → List Nat
  | l, 0, v => v :: l.drop 1
  | [], n + 1, v => 0 :: setOwner [] n v
  | x :: xs, n + 1, v => x :: setOwner xs n v

/-- `issued_supply_for_token` restricted to the fields modeled here:
native total, persisted cap, then wallet-cache fallback. -/
def issuedOf (s : FullState) (T : TokenType) : Nat :=
  if T = gasCoin then s.total else (s.caps T).getD (s.walletCache T)

/-- Native persisted cap must agree with the cached native total
(`validate_persisted_native_supply`). -/
def nativeCapAgrees (s : FullState) : Prop :=
  ∀ c, s.caps gasCoin = some c → c = s.total

/-- The full machine-checked invariant after a successful apply. -/
def fullBound (s : FullState) (tracked : List TokenType) : Prop :=
  nativeCapAgrees s ∧
  ∀ T ∈ tracked,
    s.walletCache T ≤ issuedOf s T ∧
    s.walletCache T + s.locked T ≤ issuedOf s T

/-- Executable mirror of the validation gate. -/
def fullGate (s : FullState) (tracked : List TokenType) : Bool :=
  (match s.caps gasCoin with
   | none => true
   | some c => decide (c = s.total)) &&
  tracked.all fun T =>
    Supply.rustTokenBound (issuedOf s T) (s.walletCache T) (s.locked T)

/-- `u64` range conditions required by the executable check's `checked_add`. -/
def fullU64 (s : FullState) (tracked : List TokenType) : Prop :=
  s.total ≤ u64Max ∧
  ∀ T ∈ tracked,
    issuedOf s T ≤ u64Max ∧ s.walletCache T ≤ u64Max ∧ s.locked T ≤ u64Max

/-- Acceptance of the gate implies the full bound. -/
theorem fullGate_sound {s : FullState} {tracked : List TokenType}
    (h : fullGate s tracked = true) : fullBound s tracked := by
  unfold fullGate at h
  have hp :
      (match s.caps gasCoin with
       | none => true
       | some c => decide (c = s.total)) = true ∧
      (tracked.all fun T =>
        Supply.rustTokenBound (issuedOf s T) (s.walletCache T) (s.locked T)) = true := by
    simpa [Bool.and_eq_true] using h
  constructor
  · intro c hc
    cases hcap : s.caps gasCoin with
    | none => simp [hcap] at hc
    | some c' =>
        have hc' : decide (c' = s.total) = true := by
          simpa [hcap] using hp.1
        have heq : c' = s.total := of_decide_eq_true hc'
        have hcc : c' = c := by simpa [hcap] using hc
        omega
  · intro T hT
    have ht := List.all_eq_true.mp hp.2 T hT
    exact Supply.rustTokenBound_sound ht

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- A bounded state satisfying the full bound is accepted by the gate. -/
theorem fullGate_complete {s : FullState} {tracked : List TokenType}
    (h : fullBound s tracked) (hu : fullU64 s tracked) :
    fullGate s tracked = true := by
  unfold fullGate
  have hcapCase :
      (match s.caps gasCoin with
       | none => true
       | some c => decide (c = s.total)) = true := by
    cases hcap : s.caps gasCoin with
    | none => simp [hcap]
    | some c =>
        have hc : c = s.total := h.1 c (by simp [hcap])
        simp [hcap, hc]
  have hall :
      (tracked.all fun T =>
        Supply.rustTokenBound (issuedOf s T) (s.walletCache T) (s.locked T)) = true := by
    apply List.all_eq_true.mpr
    intro T hT
    have hTb := h.2 T hT
    have hU := hu.2 T hT
    have hno : s.walletCache T + s.locked T ≤ u64Max := by omega
    exact Supply.rustTokenBound_complete hTb.1 hTb.2 hno
  simp [hcapCase, hall]

/-! ## Treasury write-back from created cap objects -/

/-- One `persist_treasury_cap_state` call. A native cap object also overwrites
the cached `state.total_supply`; the validation gate later requires the two to
agree. -/
def writeCap (s : FullState) (T : TokenType) (cap : Nat) : FullState :=
  { s with caps := updateAt s.caps T (some cap), total := if T = gasCoin then cap else s.total }

/-- Fold of treasury write-backs over `changeset.created_objects`. -/
def writeCaps (s : FullState) : List CapWrite → FullState
  | [] => s
  | (T, cap) :: ws => writeCaps (writeCap s T cap) ws

/-- A single write-back keeps native persisted-cap agreement. -/
theorem writeCap_nativeCapAgrees {s : FullState} {T : TokenType} {cap : Nat}
    (h : nativeCapAgrees s) : nativeCapAgrees (writeCap s T cap) := by
  intro c hc
  unfold writeCap at hc
  unfold nativeCapAgrees at h
  by_cases hT : T = gasCoin
  · subst T
    have hcc : cap = c := by simpa [updateAt] using hc
    subst c
    simp [writeCap]
  · have hT' : gasCoin ≠ T := fun h => hT h.symm
    have hc' : s.caps gasCoin = some c := by
      simpa [writeCap, updateAt, hT, hT'] using hc
    simpa [writeCap, hT, hT'] using h c hc'

/-- The fold keeps native persisted-cap agreement. -/
theorem writeCaps_nativeCapAgrees {s : FullState} {writes : List CapWrite}
    (h : nativeCapAgrees s) : nativeCapAgrees (writeCaps s writes) := by
  induction writes generalizing s with
  | nil => exact h
  | cons w ws ih =>
      cases w with
      | mk T cap => exact ih (writeCap_nativeCapAgrees h)

/-- A non-native cap object becomes the authoritative issued supply. -/
theorem writeCap_issued_of_ne {s : FullState} {T : TokenType} {cap : Nat}
    (hT : T ≠ gasCoin) : issuedOf (writeCap s T cap) T = cap := by
  simp [issuedOf, writeCap, updateAt, hT]

/-- A native cap object is persisted and used as the cached total. -/
theorem writeCap_caps_of_gas {s : FullState} {cap : Nat} :
    (writeCap s gasCoin cap).caps gasCoin = some cap := by
  simp [writeCap, updateAt]

theorem writeCap_total_of_gas {s : FullState} {cap : Nat} :
    (writeCap s gasCoin cap).total = cap := by
  simp [writeCap]

/-- A disagreeing persisted native cap is rejected by the gate. -/
theorem fullGate_rejects_native_cap_mismatch {s : FullState} {c : Nat}
    (hc : s.caps gasCoin = some c) (hne : c ≠ s.total)
    (tracked : List TokenType) : fullGate s tracked = false := by
  unfold fullGate
  simp [hc, hne]

/-! ## Explicit gas-object debit and owner recompute -/

/-- One checked gas-object balance write. The runtime requires the gas coin's
balance to cover the remaining debit. -/
def debitCoin (c : CoinObject) (debit : Nat) : Option CoinObject :=
  if c.tokenType = gasCoin ∧ debit ≤ c.amount then
    some { c with amount := c.amount - debit }
  else
    none

/-- A successful gas debit lowers the object amount by exactly the debit and
leaves identity/owner/token unchanged. -/
theorem debitCoin_spec {c : CoinObject} {debit : Nat} {c' : CoinObject}
    (h : debitCoin c debit = some c') :
    c'.amount = c.amount - debit ∧ c'.id = c.id ∧ c'.owner = c.owner ∧
      c'.tokenType = c.tokenType := by
  unfold debitCoin at h
  split at h
  · cases h
    simp
  · cases h

/-- Insufficient gas-object balance is rejected. -/
theorem debitCoin_none_of_insufficient {c : CoinObject} {debit : Nat}
    (hTok : c.tokenType = gasCoin) (hlt : c.amount < debit) :
    debitCoin c debit = none := by
  unfold debitCoin
  simp [hTok, Nat.not_le.mpr hlt]

/-- Apply a gas debit to the object with the given id. A missing object is a
no-op, matching the runtime's `continue` when the gas ref cannot be loaded. -/
def adjustCoin : List CoinObject → Nat → Nat → Option (List CoinObject)
  | [], _, _ => some []
  | c :: cs, id, debit =>
      if c.id = id then
        match debitCoin c debit with
        | none => none
        | some c' => some (c' :: cs)
      else
        match adjustCoin cs id debit with
        | none => none
        | some cs' => some (c :: cs')

/-- `ownerAt` after `setOwner` reads back the written value. -/
theorem ownerAt_setOwner (l : List Nat) (i v : Nat) :
    ownerAt (setOwner l i v) i = v := by
  induction i generalizing l with
  | zero => simp [ownerAt, setOwner]
  | succ n ih =>
      cases l with
      | nil => simpa [ownerAt, setOwner] using ih (l := [])
      | cons x xs => simpa [ownerAt, setOwner] using ih (l := xs)

/-- The checked object-backed balance from
`recompute_token_balances_for_owner`. -/
def recomputeNative (objectAfter priorNonObject credit ledgerAfter : Nat)
    (delta : Int) : Option Nat :=
  match Supply.checkedAdd objectAfter priorNonObject with
  | none => none
  | some a =>
      match Supply.checkedAdd a credit with
      | none => none
      | some objectBacked =>
          some (if delta > 0 then max objectBacked ledgerAfter
                else if delta < 0 then min objectBacked ledgerAfter
                else objectBacked)

/-- A covered native debit can never raise the recomputed owner balance above
the already-debited ledger value. -/
theorem recomputeNative_neg_le {objectAfter priorNonObject credit ledgerAfter : Nat}
    {delta : Int} {b : Nat} (hdelta : delta < 0)
    (h : recomputeNative objectAfter priorNonObject credit ledgerAfter delta = some b) :
    b ≤ ledgerAfter := by
  cases h1 : Supply.checkedAdd objectAfter priorNonObject with
  | none => simp [recomputeNative, h1] at h
  | some a =>
      cases h2 : Supply.checkedAdd a credit with
      | none => simp [recomputeNative, h1, h2] at h
      | some objectBacked =>
          have hnotpos : ¬ 0 < delta := by omega
          have hb : min objectBacked ledgerAfter = b := by
            simpa [recomputeNative, h1, h2, hnotpos, hdelta] using h
          rw [← hb]
          exact Nat.min_le_right _ _

/-- With no owner delta the recomputed balance is exactly the checked
object-backed amount. -/
theorem recomputeNative_zero_eq {objectAfter priorNonObject credit ledgerAfter : Nat}
    {b : Nat}
    (h : recomputeNative objectAfter priorNonObject credit ledgerAfter 0 = some b) :
    b = objectAfter + priorNonObject + credit := by
  cases h1 : Supply.checkedAdd objectAfter priorNonObject with
  | none => simp [recomputeNative, h1] at h
  | some a =>
      cases h2 : Supply.checkedAdd a credit with
      | none => simp [recomputeNative, h1, h2] at h
      | some objectBacked =>
          have hobEq : objectBacked = b := by
            simpa [recomputeNative, h1, h2] using h
          have ha : a = objectAfter + priorNonObject := Supply.checkedAdd_some h1
          have hob : objectBacked = a + credit := Supply.checkedAdd_some h2
          rw [← hobEq, hob, ha]

/-- One explicit gas-object pass followed by the native owner recompute. -/
def applyGas (s : FullState) (g : GasChange) : Option FullState :=
  match adjustCoin s.coins g.objectId g.debit with
  | none => none
  | some coins' =>
      let ledgerAfter := ownerAt s.ownerBalances g.ownerIndex
      match recomputeNative g.objectAfter g.priorNonObject g.credit ledgerAfter g.delta with
      | none => none
      | some bal =>
          let owners' := setOwner s.ownerBalances g.ownerIndex bal
          let wallet' := updateAt s.walletCache gasCoin owners'.sum
          some { s with coins := coins', ownerBalances := owners', walletCache := wallet' }

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- The gas phase keeps the native wallet cache equal to the owner-balance sum. -/
theorem applyGas_walletCache_eq_sum {s : FullState} {g : GasChange} {s' : FullState}
    (h : applyGas s g = some s') : s'.walletCache gasCoin = s'.ownerBalances.sum := by
  unfold applyGas at h
  cases hc : adjustCoin s.coins g.objectId g.debit with
  | none => simp [hc] at h
  | some coins' =>
      simp [hc] at h
      cases hr : recomputeNative g.objectAfter g.priorNonObject g.credit
          (ownerAt s.ownerBalances g.ownerIndex) g.delta with
      | none => simp [hr] at h
      | some bal =>
          simp [hr] at h
          cases h
          simp [updateAt]

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- A negative native delta cannot increase the recomputed owner balance. -/
theorem applyGas_neg_le {s : FullState} {g : GasChange} {s' : FullState}
    (h : applyGas s g = some s') (hg : g.delta < 0) :
    ownerAt s'.ownerBalances g.ownerIndex ≤ ownerAt s.ownerBalances g.ownerIndex := by
  unfold applyGas at h
  cases hc : adjustCoin s.coins g.objectId g.debit with
  | none => simp [hc] at h
  | some coins' =>
      simp [hc] at h
      cases hr : recomputeNative g.objectAfter g.priorNonObject g.credit
          (ownerAt s.ownerBalances g.ownerIndex) g.delta with
      | none => simp [hr] at h
      | some bal =>
          simp [hr] at h
          cases h
          have hle := recomputeNative_neg_le hg hr
          simpa [ownerAt_setOwner] using hle

/-! ## Object-locked coin reconciliation -/

/-- Signed locked-supply delta computed by
`reconcile_object_locked_coin_records` from the captured snapshots:
`(issued_after - issued_before) - (visible_after - visible_before)`. -/
def lockedDeltaOf (before after : FullState) (T : TokenType) : Int :=
  ((issuedOf after T : Int) - (issuedOf before T : Int)) -
    ((after.walletCache T : Int) - (before.walletCache T : Int))

/-- One token's record update. A positive delta is a checked `checked_add` of
the record amount; a negative delta is a release bounded by the existing
record amount; a zero delta is a no-op. `none` means the runtime rejects. -/
def reconcileLocked (before after : FullState) (T : TokenType) : Option Nat :=
  let δ := lockedDeltaOf before after T
  if δ > 0 then
    Supply.checkedAdd (before.locked T) δ.toNat
  else if δ < 0 then
    if (-δ).toNat ≤ before.locked T then some (before.locked T - (-δ).toNat) else none
  else
    some (before.locked T)

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- Reconciliation shifts exactly the difference between the issued-supply and
wallet-visible deltas, so a prestate satisfying the bound yields a poststate
satisfying the same full bound. -/
theorem reconcileLocked_bound {before after : FullState} {T : TokenType} {lockedAfter : Nat}
    (hpre : before.walletCache T ≤ issuedOf before T ∧
      before.walletCache T + before.locked T ≤ issuedOf before T)
    (hwa : after.walletCache T ≤ issuedOf after T)
    (hrec : reconcileLocked before after T = some lockedAfter) :
    after.walletCache T ≤ issuedOf after T ∧
      after.walletCache T + lockedAfter ≤ issuedOf after T := by
  let δ : Int := lockedDeltaOf before after T
  constructor
  · exact hwa
  · by_cases hpos : δ > 0
    · have hrec' : Supply.checkedAdd (before.locked T) δ.toNat = some lockedAfter := by
        simpa [reconcileLocked, δ, hpos] using hrec
      have hla : lockedAfter = before.locked T + δ.toNat := Supply.checkedAdd_some hrec'
      have hδto : (δ.toNat : Int) = δ := Int.toNat_of_nonneg (by omega)
      dsimp [δ, lockedDeltaOf] at hla hδto
      omega
    · by_cases hneg : δ < 0
      · have hrec' :
            (if (-δ).toNat ≤ before.locked T
             then some (before.locked T - (-δ).toNat) else none) = some lockedAfter := by
          simpa [reconcileLocked, δ, hpos, hneg] using hrec
        by_cases hle : (-δ).toNat ≤ before.locked T
        · have hla : before.locked T - (-δ).toNat = lockedAfter := by
            simpa [hle] using hrec'
          have hδto : ((-δ).toNat : Int) = -δ := Int.toNat_of_nonneg (by omega)
          dsimp [δ, lockedDeltaOf] at hla hδto
          omega
        · simp [hle] at hrec'
      · have hrec' : (some (before.locked T) : Option Nat) = some lockedAfter := by
          simpa [reconcileLocked, δ, hpos, hneg] using hrec
        have hla : before.locked T = lockedAfter := Option.some.inj hrec'
        have hδzero : δ = 0 := by omega
        dsimp [δ, lockedDeltaOf] at hδzero
        omega

/-- Fold of the per-token reconciliation over every tracked token. -/
def reconcileAll (before after : FullState) : List TokenType → Option (TokenType → Nat)
  | [] => some after.locked
  | T :: Ts =>
      match reconcileLocked before after T with
      | none => none
      | some l =>
          match reconcileAll before after Ts with
          | none => none
          | some f => some (updateAt f T l)

/-- Replace only the object-locked record map. -/
def withLocked (s : FullState) (locked : TokenType → Nat) : FullState := { s with locked := locked }

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- The whole reconciliation pass preserves the full bound on all tracked
tokens, provided the earlier phases have already kept `wallet <= issued`. -/
theorem reconcileAll_preserves_bound {before after : FullState} {tracked : List TokenType}
    {locked' : TokenType → Nat}
    (hcap : nativeCapAgrees after)
    (hpre : fullBound before tracked)
    (hwa : ∀ T ∈ tracked, after.walletCache T ≤ issuedOf after T)
    (hrec : reconcileAll before after tracked = some locked') :
    fullBound (withLocked after locked') tracked := by
  induction tracked generalizing locked' with
  | nil =>
      simp [reconcileAll] at hrec
      subst locked'
      simpa [withLocked, fullBound] using hcap
  | cons T Ts ih =>
      simp only [reconcileAll] at hrec
      cases h1 : reconcileLocked before after T with
      | none => simp [h1] at hrec
      | some l =>
          cases h2 : reconcileAll before after Ts with
          | none => simp [h1, h2] at hrec
          | some f =>
              simp [h1, h2] at hrec
              subst locked'
              have hpreT : before.walletCache T ≤ issuedOf before T ∧
                  before.walletCache T + before.locked T ≤ issuedOf before T :=
                hpre.2 T (by simp)
              have hTb := reconcileLocked_bound hpreT (hwa T (by simp)) h1
              have hpreTail : fullBound before Ts := by
                rcases hpre with ⟨hcpre, hbpre⟩
                exact ⟨hcpre, fun U hU => hbpre U (by simp [hU])⟩
              have hwaTail : ∀ U ∈ Ts, after.walletCache U ≤ issuedOf after U := by
                intro U hU
                exact hwa U (by simp [hU])
              have ihRes := ih hpreTail hwaTail h2
              constructor
              · exact hcap
              · intro U hU
                have hUcases : U = T ∨ U ∈ Ts := by simpa using hU
                cases hUcases with
                | inl hUT =>
                    subst U
                    simpa [withLocked, issuedOf, updateAt] using hTb
                | inr hUTs =>
                    by_cases hEq : U = T
                    · subst U
                      simpa [withLocked, issuedOf, updateAt] using hTb
                    · have hTU := ihRes.2 U hUTs
                      simpa [withLocked, issuedOf, updateAt, hEq] using hTU

/-! ## Non-supply writes (indexes, Move writes, access versions) -/

/-- Model of the final index / Move-write / dynamic-field / access-version
passes. Every supply field is untouched. -/
def applyNonSupply (s : FullState) (cs : ChangeSetModel) : FullState :=
  { s with indexes := cs.indexes, moveWrites := cs.moveWrites, accessVersions := cs.accessVersions }

/-- Non-supply writes cannot change the bound. -/
theorem applyNonSupply_preserves_bound {s : FullState} {cs : ChangeSetModel}
    {tracked : List TokenType} (h : fullBound s tracked) :
    fullBound (applyNonSupply s cs) tracked := by
  simpa [applyNonSupply, fullBound, nativeCapAgrees, issuedOf] using h

/-! ## Native owner-delta / total phase (bridge to `ApplyProof.lean`) -/

/-- Projection from the full state to the native state consumed by
`ApplyProof.applyChangesetNative`. -/
def toNative (s : FullState) : NativeState :=
  { total := s.total, locked := s.locked gasCoin, balances := s.ownerBalances }

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- `save_native_total_supply`: update the cached total and, when a persisted
native cap record exists, keep that record equal to the new total. -/
def writeNativeTotal (s : FullState) (total' : Nat) : FullState :=
  let caps' := if (s.caps gasCoin).isSome then updateAt s.caps gasCoin (some total') else s.caps
  { s with total := total', caps := caps' }

/-- The native total write-back preserves persisted-cap agreement. -/
theorem writeNativeTotal_nativeCapAgrees {s : FullState} {total' : Nat} :
    nativeCapAgrees (writeNativeTotal s total') := by
  intro c hc
  unfold writeNativeTotal at hc
  cases hcap : s.caps gasCoin with
  | none => simp [hcap] at hc
  | some c0 =>
      have hcEq : total' = c := by simpa [writeNativeTotal, updateAt, hcap] using hc
      exact hcEq.symm

/-- The owner-delta phase plus the single checked `u64` total update. -/
def applyNativePhase (s : FullState) (deltas : List Int) : Option FullState :=
  match applyChangesetNative (toNative s) deltas with
  | none => none
  | some n =>
      let s1 := writeNativeTotal s n.total
      let wallet' := updateAt s1.walletCache gasCoin n.balances.sum
      some { s1 with ownerBalances := n.balances, walletCache := wallet' }

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- The native phase leaves the native wallet cache equal to the owner sum. -/
theorem applyNativePhase_walletCache_eq_sum {s : FullState} {deltas : List Int} {s' : FullState}
    (h : applyNativePhase s deltas = some s') :
    s'.walletCache gasCoin = s'.ownerBalances.sum := by
  unfold applyNativePhase at h
  cases hn : applyChangesetNative (toNative s) deltas with
  | none => simp [hn] at h
  | some n =>
      simp [hn] at h
      cases h
      simp [writeNativeTotal, updateAt]

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- The native phase preserves persisted native-cap agreement. -/
theorem applyNativePhase_nativeCapAgrees {s : FullState} {deltas : List Int} {s' : FullState}
    (h : applyNativePhase s deltas = some s') : nativeCapAgrees s' := by
  unfold applyNativePhase at h
  cases hn : applyChangesetNative (toNative s) deltas with
  | none => simp [hn] at h
  | some n =>
      simp [hn] at h
      cases h
      have hw := writeNativeTotal_nativeCapAgrees (s := s) (total' := n.total)
      simpa [nativeCapAgrees] using hw

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- The native phase preserves the full bound at `GAS_COIN` by the already
machine-checked `ApplyProof.applyChangesetNative_preserves`. -/
theorem applyNativePhase_gas_bound {s : FullState} {deltas : List Int} {s' : FullState}
    (h : applyNativePhase s deltas = some s') (hinv : NativeInv (toNative s)) :
    s'.walletCache gasCoin ≤ issuedOf s' gasCoin ∧
      s'.walletCache gasCoin + s'.locked gasCoin ≤ issuedOf s' gasCoin := by
  unfold applyNativePhase at h
  cases hn : applyChangesetNative (toNative s) deltas with
  | none => simp [hn] at h
  | some n =>
      simp [hn] at h
      cases h
      have hpres : NativeInv n := applyChangesetNative_preserves hn hinv
      have hlocked : n.locked = s.locked gasCoin := by
        unfold applyChangesetNative at hn
        cases hnet : supplyDeltaChecked deltas with
        | none => simp [hnet] at hn
        | some net =>
            simp [hnet] at hn
            cases ht : nextTotalSupplyRust (toNative s).total net with
            | none => simp [ht] at hn
            | some total' =>
                simp [ht] at hn
                cases hb : applyBalances (toNative s).balances deltas with
                | none => simp [hb] at hn
                | some balances' =>
                    simp [hb] at hn
                    cases hn
                    simp [toNative]
      have hpres' : n.balances.sum ≤ n.total ∧ n.balances.sum + s.locked gasCoin ≤ n.total := by
        constructor
        · exact hpres.1
        · simpa [hlocked] using hpres.2
      simpa [issuedOf, writeNativeTotal, updateAt] using hpres'

/-- An absent explicit gas-object pass is a no-op. -/
def applyGasOption (s : FullState) (g : Option GasChange) : Option FullState :=
  match g with
  | none => some s
  | some gc => applyGas s gc

/-! ## Candidate-clone legacy repair -/

/-- The supply effect of `repair_legacy_native_wallet_overcount` on the
candidate clone: the native wallet cache is set to the canonical index value. -/
def applyLegacyRepair (s : FullState) (indexedVisible : Nat) : FullState :=
  { s with walletCache := updateAt s.walletCache gasCoin indexedVisible }

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- The legacy repair only writes a cache value known to fit under
`total - locked`, so it preserves the full bound. -/
theorem applyLegacyRepair_preserves_bound {s : FullState} {tracked : List TokenType}
    {indexedVisible : Nat}
    (hfit : indexedVisible ≤ s.total - s.locked gasCoin)
    (hgas : gasCoin ∈ tracked)
    (h : fullBound s tracked) :
    fullBound (applyLegacyRepair s indexedVisible) tracked := by
  have hgasBound := h.2 gasCoin hgas
  have hissued : issuedOf s gasCoin = s.total := by simp [issuedOf]
  have hlockedLe : s.locked gasCoin ≤ s.total := by omega
  have hsum : indexedVisible + s.locked gasCoin ≤ s.total := by omega
  constructor
  · exact h.1
  · intro T hT
    by_cases hTgas : T = gasCoin
    · subst T
      simp [issuedOf, applyLegacyRepair, updateAt]
      exact ⟨by omega, hsum⟩
    · have hTbound := h.2 T hT
      simpa [issuedOf, applyLegacyRepair, updateAt, hTgas] using hTbound

/-! ## Composed full pipeline -/

/-- Supply-relevant core of `apply_changeset_with_options(..., false)`:
native owner deltas/total, treasury write-back, explicit gas object,
locked-coin reconciliation, then non-supply writes. -/
def applyFullCore (s : FullState) (cs : ChangeSetModel) : Option FullState :=
  match applyNativePhase s cs.ownerDeltas with
  | none => none
  | some s1 =>
      let s2 := writeCaps s1 cs.capWrites
      match applyGasOption s2 cs.gas with
      | none => none
      | some s3 =>
          match reconcileAll s s3 cs.tracked with
          | none => none
          | some locked' =>
              some (applyNonSupply (withLocked s3 locked') cs)

/-- The validated pipeline: clone/apply core, run the candidate-clone legacy
repair, then reject unless the runtime supply gate accepts the candidate
(matching `apply_changeset`'s candidate clone, repair, and
`validate_cached_supply_invariants`). -/
def applyFull (s : FullState) (cs : ChangeSetModel) : Option FullState :=
  match applyFullCore s cs with
  | none => none
  | some s0 =>
      let s' := applyLegacyRepair s0 cs.nativeIndexVisible
      if fullGate s' cs.tracked then some s' else none

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- The reconciliation + non-supply tail preserves the bound when the earlier
phases have preserved cap agreement and `wallet <= issued`. -/
theorem applyFullTail_preserves {before mid : FullState} {cs : ChangeSetModel}
    {locked' : TokenType → Nat}
    (hpre : fullBound before cs.tracked)
    (hcap : nativeCapAgrees mid)
    (hwa : ∀ T ∈ cs.tracked, mid.walletCache T ≤ issuedOf mid T)
    (hrec : reconcileAll before mid cs.tracked = some locked') :
    fullBound (applyNonSupply (withLocked mid locked') cs) cs.tracked := by
  exact applyNonSupply_preserves_bound (reconcileAll_preserves_bound hcap hpre hwa hrec)

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- **Compositional preservation.** If every supply-relevant phase succeeds and
the native/cap/gas phases preserve cap agreement and the wallet bound, then the
whole core preserves the full bound. -/
theorem applyFullCore_preserves {s s1 s2 s3 s' : FullState} {cs : ChangeSetModel}
    (hpre : fullBound s cs.tracked)
    (h1 : applyNativePhase s cs.ownerDeltas = some s1)
    (h2 : writeCaps s1 cs.capWrites = s2)
    (h3 : applyGasOption s2 cs.gas = some s3)
    (hcap3 : nativeCapAgrees s3)
    (hwa3 : ∀ T ∈ cs.tracked, s3.walletCache T ≤ issuedOf s3 T)
    (hcore : applyFullCore s cs = some s') :
    fullBound s' cs.tracked := by
  unfold applyFullCore at hcore
  simp [h1, h2, h3] at hcore
  cases hrec : reconcileAll s s3 cs.tracked with
  | none => simp [hrec] at hcore
  | some locked' =>
      simp [hrec] at hcore
      cases hcore
      exact applyFullTail_preserves hpre hcap3 hwa3 hrec

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- A preserved, `u64`-bounded repaired candidate is accepted by the gate, so
the gate is a safeguard rather than an extra restriction on honest transitions. -/
theorem applyFull_eq_repaired_of_bound {s s0 : FullState} {cs : ChangeSetModel}
    (hcore : applyFullCore s cs = some s0)
    (hfit : cs.nativeIndexVisible ≤ s0.total - s0.locked gasCoin)
    (hgas : gasCoin ∈ cs.tracked)
    (hb : fullBound s0 cs.tracked)
    (hu : fullU64 (applyLegacyRepair s0 cs.nativeIndexVisible) cs.tracked) :
    applyFull s cs = some (applyLegacyRepair s0 cs.nativeIndexVisible) := by
  have hb' := applyLegacyRepair_preserves_bound hfit hgas hb
  unfold applyFull
  simp [hcore, fullGate_complete hb' hu]

set_option linter.flexible false in
set_option linter.unusedSimpArgs false in
/-- **Final runtime-facing guarantee.** If the validated full pipeline returns
a state, that state is accepted by the runtime supply check. -/
theorem applyFull_safe {s : FullState} {cs : ChangeSetModel} {s' : FullState}
    (h : applyFull s cs = some s') : fullBound s' cs.tracked := by
  unfold applyFull at h
  cases hc : applyFullCore s cs with
  | none => simp [hc] at h
  | some s0 =>
      simp [hc] at h
      rcases h with ⟨hg, hsEq⟩
      subst s'
      exact fullGate_sound hg

end Runtime
end KanariMoveRuntime
