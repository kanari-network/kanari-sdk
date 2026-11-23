module

public import KanariMoveRuntime.Supply.Store

/-!
# From `hcover` assumption to a concrete, write-path-preserved invariant

`Supply/Store.lean` states the residual precondition as an opaque function

```text
hcover : ∀ T c, capOf T = some c → T ∈ tracked
```

This module replaces it with a **concrete database invariant** over the finite
inputs Rust uses to build `tracked_fungible_token_types`
(`src/state/supply/supply_validation.rs:32-52`):

* `treasuryIndex` — the `b"treasury_index"` internal list;
* `walletKeys` — the keys of `global_token_supplies`;
* `lockedKeys` — the token types of object-locked records;
* `gasCoin` — inserted unconditionally by Rust.

`CoverInv` says every token with a persisted cap at `supply:<T>` appears in
that derived set. The two Rust paths that write a `TreasuryCap`
(`apply_treasury.rs:9-24` and `apply_phases.rs:97-110`) both insert
`treasury:<normalized T>` into the index, and the check strips that prefix and
normalizes again. The native-supply path writes `supply:<GAS_COIN>` without an
index entry, but gas is in the derived set unconditionally. These facts are
proved below as preservation lemmas:

* `writeTreasuryCap_preserves_CoverInv`
* `writeNativeSupply_preserves_CoverInv`
* `addWalletKey_preserves_CoverInv`, `addLockedKey_preserves_CoverInv`

Consequently `CoverInv` needs to be checked **once on the initial database**;
it is an inductive invariant of every normal write. The remaining residual
risk is therefore not an untestable function property but a startup condition:
legacy databases with an unindexed `supply:<T>` record, or changesets that
write `supply:` keys through the generic `move_writes` escape hatch, violate
`CoverInv` and must be repaired.

No claim is made about Rust code that has not been modelled here. -/

@[expose] public section

namespace KanariMoveRuntime
namespace Supply
namespace Cover

/-- The finite inputs from which Rust derives `tracked_fungible_token_types`. -/
structure TrackedStore where
  store : SupplyStore
  treasuryIndex : List TokenType
  walletKeys : List TokenType
  lockedKeys : List TokenType

/-- Rust's derived tracked set: treasury index, wallet cache keys, locked
records, and the unconditionally inserted gas coin. `List.all` is insensitive
to order and duplicates, so a flat append is faithful for membership. -/
def trackedTokens (v : TrackedStore) : List TokenType :=
  v.treasuryIndex ++ v.walletKeys ++ v.lockedKeys ++ [gasCoin]

/-- Every token with a persisted `TreasuryCap` at the canonical key is tracked.
This is the concrete replacement for the opaque `hcover` hypothesis. -/
def CoverInv (v : TrackedStore) : Prop :=
  ∀ (T : TokenType) (c : TreasuryCap),
    v.store.supplyKey T = some (PersistedSupply.treasury c) →
      T ∈ trackedTokens v

/-- The checker over the derived tracked set, with the locked map supplied
separately (Rust reads `object_locked_supply_for_token` per token). -/
def coveredCheck (v : TrackedStore) (locked : TokenType → Nat) : Bool :=
  rustCachedSupplyCheck v.store locked (trackedTokens v)

/-- **No opaque cover assumption needed here.** Acceptance by the checker over
the derived set bounds every token that has a persisted cap. -/
theorem coveredCheck_sound {v : TrackedStore} {locked : TokenType → Nat}
    (hv : CoverInv v) (hcheck : coveredCheck v locked = true)
    (T : TokenType) (c : TreasuryCap)
    (hc : v.store.supplyKey T = some (PersistedSupply.treasury c)) :
    v.store.walletCache T ≤ v.store.issuedSupply T ∧
      v.store.walletCache T + locked T ≤ v.store.issuedSupply T := by
  exact rustCachedSupplyCheck_sound hcheck (hv T c hc)

/-- If each finite input only grows, the derived tracked set only grows. -/
theorem trackedTokens_mono {v w : TrackedStore}
    (hi : List.Subset v.treasuryIndex w.treasuryIndex)
    (hw : List.Subset v.walletKeys w.walletKeys)
    (hl : List.Subset v.lockedKeys w.lockedKeys) :
    List.Subset (trackedTokens v) (trackedTokens w) := by
  intro U hU
  simp only [trackedTokens, List.mem_append, List.mem_singleton] at hU ⊢
  rcases hU with ((hU | hU) | hU) | hU
  · exact Or.inl (Or.inl (Or.inl (hi hU)))
  · exact Or.inl (Or.inl (Or.inr (hw hU)))
  · exact Or.inl (Or.inr (hl hU))
  · exact Or.inr hU

/-- A treasury-cap write through the normal Rust path: persist the cap at
`supply:<T>` **and** insert `treasury:<T>` into the index. -/
def writeTreasuryCap (v : TrackedStore) (T : TokenType) (cap : TreasuryCap) : TrackedStore where
  store :=
    { v.store with
      supplyKey := fun U => if U = T then some (PersistedSupply.treasury cap) else v.store.supplyKey U }
  treasuryIndex := T :: v.treasuryIndex
  walletKeys := v.walletKeys
  lockedKeys := v.lockedKeys

/-- Writing a cap through the normal path preserves `CoverInv`. -/
theorem writeTreasuryCap_preserves_CoverInv {v : TrackedStore} {T : TokenType}
    {cap : TreasuryCap} (hv : CoverInv v) : CoverInv (writeTreasuryCap v T cap) := by
  classical
  intro U cap' hU
  by_cases hUT : U = T
  · subst U
    simp [trackedTokens, writeTreasuryCap]
  · dsimp [writeTreasuryCap] at hU
    simp [hUT] at hU
    have hmem := hv U cap' hU
    exact trackedTokens_mono
      (v := v) (w := writeTreasuryCap v T cap)
      (fun U' hU' => List.mem_cons_of_mem T hU')
      (List.Subset.refl _) (List.Subset.refl _) hmem

/-- The native path writes `supply:<GAS_COIN>` without an index insert; gas is
nevertheless in the derived set because Rust inserts it unconditionally. -/
def writeNativeSupply (v : TrackedStore) (cap : TreasuryCap) : TrackedStore where
  store :=
    { v.store with
      nativeTotal := cap.total_supply
      supplyKey := fun U => if U = gasCoin then some (PersistedSupply.treasury cap) else v.store.supplyKey U }
  treasuryIndex := v.treasuryIndex
  walletKeys := v.walletKeys
  lockedKeys := v.lockedKeys

/-- The native-supply write preserves `CoverInv`. -/
theorem writeNativeSupply_preserves_CoverInv {v : TrackedStore} {cap : TreasuryCap}
    (hv : CoverInv v) : CoverInv (writeNativeSupply v cap) := by
  classical
  intro U cap' hU
  by_cases hUgas : U = gasCoin
  · subst U
    simp [trackedTokens, writeNativeSupply]
  · dsimp [writeNativeSupply] at hU
    simp [hUgas] at hU
    have hmem := hv U cap' hU
    exact trackedTokens_mono
      (v := v) (w := writeNativeSupply v cap)
      (List.Subset.refl _) (List.Subset.refl _) (List.Subset.refl _) hmem

/-- Adding a wallet-cache key only enlarges the derived tracked set. -/
def addWalletKey (v : TrackedStore) (T : TokenType) : TrackedStore :=
  { v with walletKeys := T :: v.walletKeys }

theorem addWalletKey_preserves_CoverInv {v : TrackedStore} {T : TokenType}
    (hv : CoverInv v) : CoverInv (addWalletKey v T) := by
  intro U cap' hU
  have hmem := hv U cap' hU
  exact trackedTokens_mono
    (v := v) (w := addWalletKey v T)
    (List.Subset.refl _) (fun U' hU' => List.mem_cons_of_mem T hU')
    (List.Subset.refl _) hmem

/-- Adding a locked-record token only enlarges the derived tracked set. -/
def addLockedKey (v : TrackedStore) (T : TokenType) : TrackedStore :=
  { v with lockedKeys := T :: v.lockedKeys }

theorem addLockedKey_preserves_CoverInv {v : TrackedStore} {T : TokenType}
    (hv : CoverInv v) : CoverInv (addLockedKey v T) := by
  intro U cap' hU
  have hmem := hv U cap' hU
  exact trackedTokens_mono
    (v := v) (w := addLockedKey v T)
    (List.Subset.refl _) (List.Subset.refl _)
    (fun U' hU' => List.mem_cons_of_mem T hU') hmem

end Cover
end Supply
end KanariMoveRuntime
