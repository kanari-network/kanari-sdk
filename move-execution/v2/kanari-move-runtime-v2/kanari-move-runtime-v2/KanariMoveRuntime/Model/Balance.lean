/-!
# Kanari Move Runtime v2 — balance model

Trusted core: the fungible-token balance model of `0x2::coin` (source:
`crates/kanari-frameworks/packages/kanari-system/sources/coin.move`) and the
runtime supply bookkeeping in `kanari-move-runtime-v2`.

A `Coin` is an object owned by an address carrying a `balance : u64`.
Balances are modeled as `Nat` (the Move VM rejects underflow, so the runtime
never observes a negative balance). The native gas coin is distinguished
because its total supply is persisted in the same `TreasuryCap` mechanism
(see `state/supply/supply_validation.rs`).

# Supply invariant

For every fungible token type `T`, the wallet-visible supply
(the sum of balances of all `Coin` objects whose `tokenType = T`) must never
exceed the treasury-capped total supply:

    wallet_visible_supply(T) ≤ total_supply(T)

This is the invariant enforced by `validate_cached_supply_invariants` in
`state/apply/mod.rs` and the one that caught
`usd_shared_cap_mint_pays_out_twice_to_any_recipient` when the shared
treasury cap's total was not updated to reflect pre-funded gas supply.
-/

namespace KanariMoveRuntime

/-- Addresses are modeled abstractly; only equality is needed for owner
grouping. -/
structure Address where
  /-- Hex-literal form, e.g. `"0x1"`. -/
  value : String
deriving DecidableEq, Repr

/-- A fungible-token type. The native gas coin is `gasCoin`; every other
token type (e.g. `0x2::usd::USD`) is tracked through a shared
`TreasuryCap`. -/
structure TokenType where
  /-- Human-readable fully-qualified name, e.g. `"0x2::usd::USD"`. -/
  name : String
  /-- Whether this is the native gas coin. -/
  isGas : Bool
deriving DecidableEq, Repr

/-- The native gas coin type. Its total supply is the native
`state.total_supply` field, mirrored by a persisted `TreasuryCap`. -/
def gasCoin : TokenType :=
  ⟨"0x2::kanari::KANARI", true⟩

/-- A coin object: its token type, owner address, and balance. -/
structure Coin where
  /-- The token type of this coin object. -/
  tokenType : TokenType
  /-- The address that owns this coin. -/
  owner : Address
  /-- The balance held by this coin object. -/
  balance : Nat
deriving DecidableEq, Repr

end KanariMoveRuntime
