import KanariMoveRuntime.Model.Balance
import KanariMoveRuntime.Model.Treasury

/-!
# Kanari Move Runtime v2 — state model

Trusted core: the runtime's cached supply bookkeeping
(`crates/kanari-move-runtime-v2/src/state/supply/`, `state/apply/mod.rs`).

`State` captures what `validate_cached_supply_invariants` reads:

- `tokenSupplies : TokenType → Nat` — wallet-visible supply cache,
  maintained by `capture_supply_changed` / owner reconciliation.
- `capTotal : TreasuryCap → Nat` — treasury-cap total supply per cap.
- `supplyRecord : TreasuryCap → Nat` — the persisted `supply:...` record mirror
  used by `issued_supply_for_token`.

`capTotal` is authoritative for the check (`totalSupply` in
`Supply/Statement.lean`); `tokenSupplies` and `supplyRecord` are caches that
the runtime reconciles before validating. The model keeps them in the state
so cached and persisted totals are not conflated, but the proven invariant
only depends on `capTotal` and the coin list.

The supply invariant (in `Supply/Statement.lean`) is:

    ∀ T, wallet_visible_supply(T) ≤ total_supply(T)

where `wallet_visible_supply(T)` is the sum of `Coin` balances and
`total_supply(T)` is the treasury-cap total. This is what
`validate_cached_supply_invariants` checks in `validate_token_supply_bound`.
-/

namespace KanariMoveRuntime

/-- The runtime's supply bookkeeping state.
- `tokenSupplies` maps each token type to its wallet-visible supply cache
  (`global_token_supplies`).
- `capTotal` maps each treasury cap to its `total_supply` field.
- `supplyRecord` maps each treasury cap to the persisted `supply:...`
  record used by `issued_supply_for_token`.
-/
structure State where
  /-- Wallet-visible supply cache per token type. -/
  tokenSupplies : TokenType → Nat
  /-- Treasury-cap total supply per cap. -/
  capTotal : TreasuryCap → Nat
  /-- Persisted `supply:...` record per treasury cap. -/
  supplyRecord : TreasuryCap → Nat

/-- Overwrite the cached total supply of `cap`. This mirrors replacing the
`total_supply` field of the shared treasury-cap object. -/
def State.setCapTotal (st : State) (cap : TreasuryCap) (total : Nat) : State :=
  { st with capTotal := fun c => if c = cap then total else st.capTotal c }

/-- Cache the effect of `0x2::coin::mint<T>(cap, amount)` on `state.capTotal`
(the authoritative `TreasuryCap<T>.total_supply` is incremented by `amount`). -/
def State.mintCap (st : State) (cap : TreasuryCap) (amount : Nat) : State :=
  st.setCapTotal cap (st.capTotal cap + amount)

/-- Cache the effect of `0x2::coin::burn<T>(cap, amount)` on `state.capTotal`
(the authoritative `TreasuryCap<T>.total_supply` is decremented by `amount`). -/
def State.burnCap (st : State) (cap : TreasuryCap) (amount : Nat) : State :=
  st.setCapTotal cap (st.capTotal cap - amount)

@[simp] theorem State.mintCap_self (st : State) (cap : TreasuryCap) (amount : Nat) :
    (st.mintCap cap amount).capTotal cap = st.capTotal cap + amount := by
  simp [State.mintCap, State.setCapTotal]

@[simp] theorem State.mintCap_other (st : State) (cap other : TreasuryCap)
    (amount : Nat) (h : other ≠ cap) :
    (st.mintCap cap amount).capTotal other = st.capTotal other := by
  simp [State.mintCap, State.setCapTotal, h]

@[simp] theorem State.burnCap_self (st : State) (cap : TreasuryCap) (amount : Nat) :
    (st.burnCap cap amount).capTotal cap = st.capTotal cap - amount := by
  simp [State.burnCap, State.setCapTotal]

@[simp] theorem State.burnCap_other (st : State) (cap other : TreasuryCap)
    (amount : Nat) (h : other ≠ cap) :
    (st.burnCap cap amount).capTotal other = st.capTotal other := by
  simp [State.burnCap, State.setCapTotal, h]

end KanariMoveRuntime
