import KanariMoveRuntime.Model.Balance

/-!
# Kanari Move Runtime v2 — treasury-cap model

Trusted core: the `0x2::coin::TreasuryCap<T>` object (source:
`crates/kanari-frameworks/packages/kanari-system/sources/coin.move`) and its
role as the source of truth for a non-native token's total supply.

A `TreasuryCap` is a shared object whose `total_supply` field is the
authoritative total supply of token type `T`. Every mint through the shared
cap increments this field; every burn decrements it. The supply invariant
(`wallet_visible_supply(T) ≤ total_supply(T)`) is only sound when *all*
`T`-denominated supply originates through this treasury cap — pre-funding
an address with `T` outside the treasury cap (as `fund_sender_with_coin_type`
did for USD gas in `engine_tests.rs`) breaks the accounting and is exactly
what the invariant catches.

`TreasuryCap.mint` / `TreasuryCap.burn` model the effect on the authoritative
field; the runtime-side cache update is `State.mintCap` / `State.burnCap`.
-/

namespace KanariMoveRuntime

/-- A treasury-cap object for token type `tokenType`.
- `total_supply` is the authoritative issued supply of `tokenType`.
- `owner` is the shared-object owner (the DAO / governor address).
- `tokenType` identifies which fungible token type this cap governs. -/
structure TreasuryCap where
  /-- Total issued supply of this token type. -/
  total_supply : Nat
  /-- Owner of the shared treasury-cap object. -/
  owner : Address
  /-- The token type this treasury cap governs. -/
  tokenType : TokenType
deriving DecidableEq, Repr

/-- Minting `amount` through a treasury cap increments its authoritative
total supply by `amount` (`0x2::coin::mint`). -/
def TreasuryCap.mint (cap : TreasuryCap) (amount : Nat) : TreasuryCap :=
  { cap with total_supply := cap.total_supply + amount }

/-- Burning `amount` through a treasury cap decrements its authoritative
total supply by `amount` (`0x2::coin::burn`). Move rejects underflow, so the
caller must establish `amount ≤ cap.total_supply`. -/
def TreasuryCap.burn (cap : TreasuryCap) (amount : Nat) : TreasuryCap :=
  { cap with total_supply := cap.total_supply - amount }

@[simp] theorem TreasuryCap.mint_total_supply (cap : TreasuryCap) (amount : Nat) :
    (cap.mint amount).total_supply = cap.total_supply + amount := rfl

@[simp] theorem TreasuryCap.burn_total_supply (cap : TreasuryCap) (amount : Nat) :
    (cap.burn amount).total_supply = cap.total_supply - amount := rfl

end KanariMoveRuntime
