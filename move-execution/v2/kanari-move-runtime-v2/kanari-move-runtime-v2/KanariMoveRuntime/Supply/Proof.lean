import KanariMoveRuntime.Model.State
import KanariMoveRuntime.Model.Balance
import KanariMoveRuntime.Model.Treasury
import KanariMoveRuntime.Supply.Statement
open KanariMoveRuntime

/-
# Kanari Move Runtime v2 — supply invariant proof

Proofs for `Supply.Statement`: the cached supply invariant enforced by
`validate_cached_supply_invariants` in
`crates/kanari-move-runtime-v2/src/state/apply/mod.rs`.

The original version of this file proved `Statement` by `sorry`. That
statement quantified over *arbitrary* states and is in fact false: any state
whose wallet coins hold more of a token than its treasury cap records violates
it, which is exactly the `usd_shared_cap_mint_pays_out_twice_to_any_recipient`
bug. `Counterexample.lean` proves the negation. What is actually true — and
what the runtime relies on — is proved here:

1. `coinSupplyOf_cons*` — how the wallet-visible sum changes when a coin is
   added to / inspected in the wallet;
2. `mint_preserves` — minting through the treasury cap registered for `T`
   increments the wallet-visible supply and the cap total by the same amount,
   so the invariant is preserved;
3. `transfer_pair_preserves` — transferring between two coins of the same
   token type leaves the wallet-visible sum unchanged;
4. `tokenBoundHolds_sound` / `cachedSupplyCheck_sound` — if
   `validate_cached_supply_invariants` accepts a state, the invariant holds
   (the checker never accepts a violating state);
5. `cachedSupplyCheck_complete` — if every tracked token is within its bound,
   the checker accepts.

`Trace.lean` lifts (2)–(3) to an arbitrary execution trace (mint/burn/transfer
steps) and proves the invariant for every reachable state, together with the
negative result that funding a wallet outside the treasury cap breaks it.

This file is machine-checked, sorry-free Lean 4 core (no mathlib).
-/

namespace KanariMoveRuntime

namespace Supply

/-! ## Wallet-visible supply lemmas -/

/-- A single coin's contribution to the wallet-visible supply of `T`. -/
def coinContribution (c : Coin) (T : TokenType) : Nat :=
  if c.tokenType = T then c.balance else 0

/-- Summing the wallet is the same as summing the head contribution and the
tail. -/
theorem coinSupplyOf_cons (c : Coin) (coins : List Coin) (T : TokenType) :
    coinSupplyOf (c :: coins) T =
      coinContribution c T + coinSupplyOf coins T := by
  simp [coinSupplyOf, coinContribution, List.foldr]
  split <;> omega

/-- A coin of type `T` adds exactly its balance to the supply of `T`. -/
theorem coinSupplyOf_cons_of_type (c : Coin) (coins : List Coin) (T : TokenType)
    (h : c.tokenType = T) :
    coinSupplyOf (c :: coins) T = c.balance + coinSupplyOf coins T := by
  rw [coinSupplyOf_cons, coinContribution, ite_eq_left h]

/-- A coin of a different type does not change the supply of `T`. -/
theorem coinSupplyOf_cons_of_ne (c : Coin) (coins : List Coin) (T : TokenType)
    (h : c.tokenType ≠ T) :
    coinSupplyOf (c :: coins) T = coinSupplyOf coins T := by
  rw [coinSupplyOf_cons, coinContribution, ite_eq_right h, Nat.zero_add]

/-- Specialized form used by the mint proof: a freshly minted coin of type
`T` adds its amount to the supply of `T`. -/
theorem coinSupplyOf_cons_self (T : TokenType) (owner : Address) (amount : Nat)
    (coins : List Coin) :
    coinSupplyOf (⟨T, owner, amount⟩ :: coins) T =
      amount + coinSupplyOf coins T := by
  simp [coinSupplyOf, List.foldr]

/-- A freshly minted coin of a different type does not change the supply
of `T`. -/
theorem coinSupplyOf_cons_other {T T' : TokenType} (h : T ≠ T') (owner : Address)
    (amount : Nat) (coins : List Coin) :
    coinSupplyOf (⟨T, owner, amount⟩ :: coins) T' = coinSupplyOf coins T' := by
  simp [coinSupplyOf, List.foldr, h]

/-- Transferring `amount` between two wallets of the same token type preserves
the wallet-visible supply of that type (Move transfers move an amount between
two `Coin<T>` objects; the multiset of balances changes but the sum does not). -/
theorem coinSupplyOf_transfer_pair (T : TokenType) (a b amount : Nat)
    (sender receiver : Address) (hamount : amount ≤ a) :
    coinSupplyOf [⟨T, sender, a - amount⟩, ⟨T, receiver, b + amount⟩] T =
      coinSupplyOf [⟨T, sender, a⟩, ⟨T, receiver, b⟩] T := by
  simp [coinSupplyOf, List.foldr]
  omega

/-! ## Mint preserves the invariant -/

/-- Minting `amount` of `T` through the treasury cap registered for `T`
preserves the supply invariant.

The minted coin is added to the wallet and `state.capTotal cap` is incremented
by the same `amount` (the authoritative `TreasuryCap<T>.total_supply`). For
the minted token type both sides grow by `amount`. For a different token type
whose cap is the same object, only the cap total grows (so the bound still
holds); for every other cap nothing changes. -/
theorem mint_preserves
    (st : State) (coins : List Coin)
    (capOf : TokenType → Option TreasuryCap)
    (T : TokenType) (cap : TreasuryCap) (hc : capOf T = some cap)
    (owner : Address) (amount : Nat)
    (hinv : SupplyInvariant st coins capOf) :
    SupplyInvariant (st.mintCap cap amount) (⟨T, owner, amount⟩ :: coins) capOf := by
  intro T' c' hc'
  have hbound : walletVisibleSupply coins T' ≤ st.capTotal c' := hinv T' c' hc'
  by_cases hcap : c' = cap
  · -- The cap governing `T'` is the cap that was minted through.
    subst c'
    by_cases hT : T' = T
    · -- Same token type: both wallet-visible supply and cap total grow.
      subst T'
      have hvis : walletVisibleSupply (⟨T, owner, amount⟩ :: coins) T =
          amount + walletVisibleSupply coins T := by
        simp only [walletVisibleSupply, coinSupplyOf_cons_self]
      rw [hvis]
      change amount + walletVisibleSupply coins T
        ≤ (st.mintCap cap amount).capTotal cap
      rw [State.mintCap_self]
      omega
    · -- Different token type: only the (unrelated) cap total grows.
      have hvis : walletVisibleSupply (⟨T, owner, amount⟩ :: coins) T' =
          walletVisibleSupply coins T' := by
        simp only [walletVisibleSupply,
          coinSupplyOf_cons_other (Ne.symm hT) owner amount coins]
      rw [hvis]
      change walletVisibleSupply coins T' ≤ (st.mintCap cap amount).capTotal cap
      rw [State.mintCap_self]
      omega
  · -- A different cap is untouched. If `T' = T` then `c'` would have to equal
    -- the cap registered for `T`, contradiction; so the wallet sum is fixed.
    have hT : T' ≠ T := by
      intro hEq
      subst T'
      have hsome : some c' = some cap := hc'.symm.trans hc
      exact hcap (Option.some.inj hsome)
    have hvis : walletVisibleSupply (⟨T, owner, amount⟩ :: coins) T' =
        walletVisibleSupply coins T' := by
      simp only [walletVisibleSupply,
        coinSupplyOf_cons_other (Ne.symm hT) owner amount coins]
    rw [hvis]
    change walletVisibleSupply coins T' ≤ (st.mintCap cap amount).capTotal c'
    rw [State.mintCap_other st cap c' amount hcap]
    exact hbound

/-! ## The executable check is sound and complete -/

/-- If the per-token bound check passes, then both the wallet-visible bound
and the accounted (wallet + object-locked) bound hold. -/
theorem tokenBoundHolds_sound {total wallet locked : Nat}
    (h : tokenBoundHolds total wallet locked = true) :
    wallet ≤ total ∧ wallet + locked ≤ total := by
  simpa [tokenBoundHolds, Bool.and_eq_true] using h

/-- If both bounds hold, the executable check passes. -/
theorem tokenBoundHolds_complete {total wallet locked : Nat}
    (hwallet : wallet ≤ total) (haccounted : wallet + locked ≤ total) :
    tokenBoundHolds total wallet locked = true := by
  simp [tokenBoundHolds, hwallet, haccounted]

/-- **Soundness of `validate_cached_supply_invariants`.** If the runtime's
cached check accepts a state (for a token-type list covering every capped
token), then the supply invariant holds. An accepted state therefore cannot
exhibit the `usd_shared_cap_mint_pays_out_twice_to_any_recipient` overcount. -/
theorem cachedSupplyCheck_sound
    {st : State} {coins : List Coin}
    {capOf : TokenType → Option TreasuryCap}
    {locked : TokenType → Nat} {tracked : List TokenType}
    (hcheck : cachedSupplyCheck st coins capOf locked tracked = true)
    (hcover : ∀ (T : TokenType) (c : TreasuryCap), capOf T = some c → T ∈ tracked) :
    SupplyInvariant st coins capOf := by
  intro T c hc
  have hall := List.all_eq_true.mp hcheck T (hcover T c hc)
  change (match capOf T with
    | none => true
    | some cap => tokenBoundHolds (totalSupply st cap)
        (walletVisibleSupply coins T) (locked T)) = true at hall
  simp only [hc] at hall
  exact (tokenBoundHolds_sound hall).1

/-- Runtime-facing form of the previous `theorem holds`: if the cached check
accepts the state (and `tracked` covers every capped token), then the supply
statement holds. This replaces the unprovable `Statement st ... := by sorry`
from the previous draft with the guarantee the runtime actually establishes. -/
theorem holds
    {st : State} {coins : List Coin}
    {capOf : TokenType → Option TreasuryCap}
    {locked : TokenType → Nat} {tracked : List TokenType}
    (hcheck : cachedSupplyCheck st coins capOf locked tracked = true)
    (hcover : ∀ (T : TokenType) (c : TreasuryCap), capOf T = some c → T ∈ tracked) :
    Statement st coins capOf :=
  cachedSupplyCheck_sound hcheck hcover

/-- **Completeness of `validate_cached_supply_invariants`.** If every tracked
token satisfies the accounted bound, the checker accepts the state. -/
theorem cachedSupplyCheck_complete    {st : State} {coins : List Coin}
    {capOf : TokenType → Option TreasuryCap}
    {locked : TokenType → Nat} {tracked : List TokenType}
    (hinv : AccountedInvariant st coins capOf locked) :
    cachedSupplyCheck st coins capOf locked tracked = true := by
  apply List.all_eq_true.mpr
  intro T _
  change (match capOf T with
    | none => true
    | some cap => tokenBoundHolds (totalSupply st cap)
        (walletVisibleSupply coins T) (locked T)) = true
  cases h : capOf T with
  | none => rfl
  | some c =>
      exact tokenBoundHolds_complete (hinv T c h).1 (hinv T c h).2

end Supply

end KanariMoveRuntime
