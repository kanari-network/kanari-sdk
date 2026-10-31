import KanariMoveRuntime.Supply.Statement
import KanariMoveRuntime.Supply.Proof
open KanariMoveRuntime

/-
# Kanari Move Runtime v2 — faithful model of `issued_supply_for_token`

`Statement.lean` models the check with an abstract `capOf`. The real Rust
lookup is more involved:

```text
issued_supply_for_token(T):
  if T == GAS_COIN:            native total_supply
  else if supply_key(T) is a TreasuryCap:      cap.total_supply
  else if supply_key(T) is a u64 record:       record
  else if supply_key_raw(T) is a TreasuryCap:  cap.total_supply
  else if supply_key_raw(T) is a u64 record:   record
  else:                        global_token_supplies[T]   -- fallback
```

(`state/supply/supply_store.rs:77`.) `validate_token_supply_bound` also uses
`u64::checked_add` for the accounted supply, and
`object_locked_supply_for_token` sums object-locked records with the same
checked addition (`state/supply/supply_repairs.rs:119`).

This module mirrors that exactly, so the proofs below apply to the real
decision procedure rather than to a simplified one:

* `rustTokenBound` — `checked_add` plus the two `<=` conjuncts;
* `SupplyStore.issuedSupply` — the lookup precedence above;
* `rustCachedSupplyCheck` — the loop of `validate_cached_supply_invariants`;
* `rustCachedSupplyCheck_sound` — acceptance implies the bound for the
  authoritative total;
* `reconciled_bound` — if the wallet cache is reconciled with the canonical
  coin scan (which `validate_supply_invariants` enforces), acceptance implies
  the canonical wallet supply fits;
* `fallback_check_accepts` — documents that the final fallback
  (`total = wallet cache`) is *not* a supply cap: it accepts any size;
* `ofState_check_sound` — the bridge back to `SupplyInvariant` for states
  whose cap totals agree with the persisted caps.

The remaining modelling assumption is `hcover`: every token type that can be
looked up through `supply:...` must appear in `tracked` (in Rust that is the
`treasury_index`; `tracked_fungible_token_types` unions it with the cache,
the gas coin and all coin records).
-/

namespace KanariMoveRuntime

namespace Supply

/-- The largest `u64`, as a `Nat`. -/
def u64Max : Nat := 2 ^ 64 - 1

/-- `u64::checked_add`, modeled on `Nat`: `none` exactly on overflow. -/
def checkedAdd (a b : Nat) : Option Nat :=
  if a + b ≤ u64Max then some (a + b) else none

theorem checkedAdd_some {a b c : Nat} (h : checkedAdd a b = some c) :
    c = a + b := by
  unfold checkedAdd at h
  split at h
  · exact (Option.some.inj h).symm
  · cases h

theorem checkedAdd_none {a b : Nat} (h : u64Max < a + b) :
    checkedAdd a b = none := by
  unfold checkedAdd
  simp [Nat.not_le.mpr h]

/-- Executable mirror of `validate_token_supply_bound`: the accounted supply
is computed with checked addition, then both bounds are compared. -/
def rustTokenBound (total wallet locked : Nat) : Bool :=
  match checkedAdd wallet locked with
  | none => false
  | some accounted => decide (wallet ≤ total) && decide (accounted ≤ total)

/-- Acceptance of the per-token bound implies both inequalities, with the
accounted supply equal to `wallet + locked` (the addition did not overflow). -/
theorem rustTokenBound_sound {total wallet locked : Nat}
    (h : rustTokenBound total wallet locked = true) :
    wallet ≤ total ∧ wallet + locked ≤ total := by
  unfold rustTokenBound at h
  cases hc : checkedAdd wallet locked with
  | none => rw [hc] at h; simp at h
  | some accounted =>
      rw [hc] at h
      have hacc : accounted = wallet + locked := checkedAdd_some hc
      subst accounted
      simpa [Bool.and_eq_true] using h

/-- Completeness of the per-token bound when the addition fits in a `u64`. -/
theorem rustTokenBound_complete {total wallet locked : Nat}
    (hwallet : wallet ≤ total) (haccounted : wallet + locked ≤ total)
    (hnoOverflow : wallet + locked ≤ u64Max) :
    rustTokenBound total wallet locked = true := by
  unfold rustTokenBound
  have hc : checkedAdd wallet locked = some (wallet + locked) := by
    unfold checkedAdd
    simp [hnoOverflow]
  rw [hc]
  simp [hwallet, haccounted]

/-- A value loaded from the `supply:<token>` key: either a `TreasuryCap` object
or a legacy `u64` record. -/
inductive PersistedSupply where
  | treasury (cap : TreasuryCap)
  | legacy (amount : Nat)
deriving Repr

/-- The storage view used by `issued_supply_for_token`.

`supplyKeyRaw` is the raw-spelling lookup; it is only consulted when the raw
key differs from the canonical key, so callers model it as `none` for equal
spellings. `walletCache` is `global_token_supplies`. -/
structure SupplyStore where
  /-- Native `state.total_supply`. -/
  nativeTotal : Nat
  /-- Canonical `supply:<normalized>` record. -/
  supplyKey : TokenType → Option PersistedSupply
  /-- Legacy raw-spelling `supply:<raw>` record. -/
  supplyKeyRaw : TokenType → Option PersistedSupply
  /-- `global_token_supplies` cache (wallet-visible supply). -/
  walletCache : TokenType → Nat

/-- The persisted total, following the exact precedence of
`issued_supply_for_token`: canonical `TreasuryCap`, canonical `u64`, raw
`TreasuryCap`, raw `u64`, otherwise `none`. -/
def SupplyStore.lookupPersisted (s : SupplyStore) (T : TokenType) : Option Nat :=
  match s.supplyKey T with
  | some (.treasury cap) => some cap.total_supply
  | some (.legacy amount) => some amount
  | none =>
    match s.supplyKeyRaw T with
    | some (.treasury cap) => some cap.total_supply
    | some (.legacy amount) => some amount
    | none => none

/-- `issued_supply_for_token`: the native total for the gas coin, otherwise
the first persisted record found, otherwise the wallet cache. -/
def SupplyStore.issuedSupply (s : SupplyStore) (T : TokenType) : Nat :=
  if T = gasCoin then s.nativeTotal
  else (s.lookupPersisted T).getD (s.walletCache T)

@[simp] theorem SupplyStore.issuedSupply_gas (s : SupplyStore) :
    s.issuedSupply gasCoin = s.nativeTotal := by
  simp [SupplyStore.issuedSupply]

theorem SupplyStore.issuedSupply_of_lookup {s : SupplyStore} {T : TokenType}
    {v : Nat} (hT : T ≠ gasCoin) (h : s.lookupPersisted T = some v) :
    s.issuedSupply T = v := by
  simp [SupplyStore.issuedSupply, hT, h]

/-- The final fallback: no persisted record, so `issuedSupply` returns the
wallet cache itself. -/
theorem SupplyStore.issuedSupply_fallback {s : SupplyStore} {T : TokenType}
    (hT : T ≠ gasCoin) (h : s.lookupPersisted T = none) :
    s.issuedSupply T = s.walletCache T := by
  simp [SupplyStore.issuedSupply, hT, h]

/-- Mirror of `validate_cached_supply_invariants`: for every tracked token,
compute the accounted supply with checked addition and compare both bounds
against `issued_supply_for_token`. -/
def rustCachedSupplyCheck (s : SupplyStore) (locked : TokenType → Nat)
    (tracked : List TokenType) : Bool :=
  tracked.all fun T =>
    match checkedAdd (s.walletCache T) (locked T) with
    | none => false
    | some accounted =>
      decide (s.walletCache T ≤ s.issuedSupply T) &&
        decide (accounted ≤ s.issuedSupply T)

/-- **Soundness of the faithful checker.** If `rustCachedSupplyCheck` accepts
and `T` is tracked, the cached wallet supply and the accounted supply both fit
inside `issuedSupply T`. -/
theorem rustCachedSupplyCheck_sound {s : SupplyStore} {locked : TokenType → Nat}
    {tracked : List TokenType} {T : TokenType}
    (h : rustCachedSupplyCheck s locked tracked = true) (hT : T ∈ tracked) :
    s.walletCache T ≤ s.issuedSupply T ∧
      s.walletCache T + locked T ≤ s.issuedSupply T := by
  have hall := List.all_eq_true.mp h T hT
  change (match checkedAdd (s.walletCache T) (locked T) with
    | none => false
    | some accounted =>
      decide (s.walletCache T ≤ s.issuedSupply T) &&
        decide (accounted ≤ s.issuedSupply T)) = true at hall
  cases hc : checkedAdd (s.walletCache T) (locked T) with
  | none => rw [hc] at hall; simp at hall
  | some accounted =>
      rw [hc] at hall
      have hacc : accounted = s.walletCache T + locked T := checkedAdd_some hc
      subst accounted
      simpa [Bool.and_eq_true] using hall

/-- Acceptance also rules out an accounted-supply overflow, because the
accounted supply is bounded by a `u64` total. -/
theorem rustCachedSupplyCheck_no_overflow {s : SupplyStore} {locked : TokenType → Nat}
    {tracked : List TokenType} {T : TokenType}
    (h : rustCachedSupplyCheck s locked tracked = true) (hT : T ∈ tracked)
    (hTotal : s.issuedSupply T ≤ u64Max) :
    s.walletCache T + locked T ≤ u64Max := by
  have hs := rustCachedSupplyCheck_sound h hT
  omega

/-- **Bridge to canonical balances.** `validate_supply_invariants` reconciles
the cached wallet supply with the canonical scan of owned coin objects
(`compute_wallet_supply_index`). Under that reconciliation, acceptance of the
fast check bounds the *canonical* wallet supply, not just the cache. -/
theorem reconciled_bound {s : SupplyStore} {locked : TokenType → Nat}
    {tracked : List TokenType} {coins : List Coin} {T : TokenType}
    (h : rustCachedSupplyCheck s locked tracked = true) (hT : T ∈ tracked)
    (hrecon : s.walletCache T = coinSupplyOf coins T) :
    coinSupplyOf coins T ≤ s.issuedSupply T ∧
      coinSupplyOf coins T + locked T ≤ s.issuedSupply T := by
  have hs := rustCachedSupplyCheck_sound h hT
  rw [← hrecon]
  exact hs

/-- **The fallback is not a supply cap.** With no persisted total, the
authoritative value is the wallet cache, so the check accepts any wallet size
that fits in a `u64`. A token without a treasury cap therefore has no fixed
maximum supply; the invariant is only an accounting identity in that case. -/
theorem fallback_check_accepts {s : SupplyStore} {T : TokenType}
    (hT : T ≠ gasCoin) (h : s.lookupPersisted T = none)
    (hcache : s.walletCache T ≤ u64Max) :
    rustTokenBound (s.issuedSupply T) (s.walletCache T) 0 = true := by
  rw [SupplyStore.issuedSupply_fallback hT h]
  unfold rustTokenBound
  have hc : checkedAdd (s.walletCache T) 0 = some (s.walletCache T) := by
    unfold checkedAdd
    simp [hcache]
  rw [hc]
  simp

/-! ## Bridge back to the concrete `SupplyInvariant` -/

/-- The concrete state viewed through the faithful store model, where the
persisted `supply:...` record for `T` is the cap that `capOf` selects and the
wallet cache is the canonical coin scan. -/
def SupplyStore.ofState (st : State) (capOf : TokenType → Option TreasuryCap)
    (coins : List Coin) : SupplyStore where
  nativeTotal :=
    match capOf gasCoin with
    | some cap => st.capTotal cap
    | none => 0
  supplyKey := fun T => (capOf T).map PersistedSupply.treasury
  supplyKeyRaw := fun _ => none
  walletCache := fun T => coinSupplyOf coins T

theorem SupplyStore.ofState_issuedSupply_gas {st : State}
    {capOf : TokenType → Option TreasuryCap} {coins : List Coin}
    {cap : TreasuryCap} (hc : capOf gasCoin = some cap) :
    (SupplyStore.ofState st capOf coins).issuedSupply gasCoin = totalSupply st cap := by
  simp [SupplyStore.issuedSupply, SupplyStore.ofState, hc, totalSupply]

theorem SupplyStore.ofState_issuedSupply {st : State}
    {capOf : TokenType → Option TreasuryCap} {coins : List Coin}
    {T : TokenType} {cap : TreasuryCap}
    (hc : capOf T = some cap) (hT : T ≠ gasCoin) :
    (SupplyStore.ofState st capOf coins).issuedSupply T = cap.total_supply := by
  simp [SupplyStore.issuedSupply, SupplyStore.lookupPersisted,
    SupplyStore.ofState, hT, hc]

/-- **Faithful checker soundness for concrete states.** If the real lookup
returns the persisted treasury cap (and the cap's `total_supply` field agrees
with the state cache `capTotal`), and `tracked` covers every capped token plus
the gas coin, then acceptance by the faithful checker implies
`SupplyInvariant`. -/
theorem ofState_check_sound {st : State} {capOf : TokenType → Option TreasuryCap}
    {coins : List Coin} {locked : TokenType → Nat} {tracked : List TokenType}
    (hcheck : rustCachedSupplyCheck (SupplyStore.ofState st capOf coins) locked tracked = true)
    (hcover : ∀ (T : TokenType) (c : TreasuryCap), capOf T = some c → T ∈ tracked)
    (hgasMem : gasCoin ∈ tracked)
    (hconsistent : ∀ (T : TokenType) (c : TreasuryCap), capOf T = some c →
      st.capTotal c = c.total_supply) :
    SupplyInvariant st coins capOf := by
  intro T c hc
  have hmem : T ∈ tracked := by
    by_cases hT : T = gasCoin
    · simpa [hT] using hgasMem
    · exact hcover T c hc
  have hsound := rustCachedSupplyCheck_sound hcheck hmem
  have hi : (SupplyStore.ofState st capOf coins).issuedSupply T = totalSupply st c := by
    by_cases hT : T = gasCoin
    · subst T
      exact SupplyStore.ofState_issuedSupply_gas hc
    · rw [SupplyStore.ofState_issuedSupply hc hT, totalSupply, hconsistent T c hc]
  rw [hi] at hsound
  have hw : (SupplyStore.ofState st capOf coins).walletCache T = coinSupplyOf coins T := rfl
  rw [hw] at hsound
  simpa [walletVisibleSupply] using hsound.1

end Supply

end KanariMoveRuntime
