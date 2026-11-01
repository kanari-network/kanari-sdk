import KanariMoveRuntime.Model.State
import KanariMoveRuntime.Model.Balance
import KanariMoveRuntime.Model.Treasury
open KanariMoveRuntime

/-
Kanari Move Runtime v2 — supply invariant statement

Statement: the cached supply invariant enforced by validate_cached_supply_invariants
in crates/kanari-move-runtime-v2/src/state/apply/mod.rs:

for all T, wallet visible supply of T is at most total supply of T

where wallet visible supply of T is the sum of balances of all Coin objects whose
token type equals T, and total supply of T is the treasury cap total supply for T.

This is the invariant validate_token_supply_bound checks, and it is exactly
what caught usd_shared_cap_mint_pays_out_twice_to_any_recipient in
crates/kanari-core/tests/unit/engine_tests.rs: funding the gas object with
USD outside the treasury cap made wallet visible supply of USD exceed
total supply of USD.

Statements and definitions only; proofs live in Proof.lean, Trace.lean and
Counterexample.lean.
-/

namespace KanariMoveRuntime

namespace Supply

/-- The wallet-visible supply contributed by a list of coin objects: the sum
of balances of the coins whose token type is `T`. This is
`compute_wallet_supply_index` in `state/supply/supply_holders.rs`, specialized
to one token type. -/
def coinSupplyOf (coins : List Coin) (T : TokenType) : Nat :=
  coins.foldr (fun c acc => if c.tokenType = T then c.balance + acc else acc) 0

/-- The wallet-visible supply of token type `T`: the sum of balances of all
`Coin` objects whose `tokenType = T`. Only wallet-held coins are counted;
object-locked supply is passed separately to the checker (`locked`). -/
def walletVisibleSupply (coins : List Coin) (T : TokenType) : Nat :=
  coinSupplyOf coins T

/-- The treasury-cap total supply of token type `T`, taken from the treasury
cap `cap`'s `total_supply` field. For non-native tokens this is the
authoritative issued supply (`issued_supply_for_token`); for the native gas
coin it is the persisted `TreasuryCap` mirror of `state.total_supply`
(`validate_persisted_native_supply`). -/
def totalSupply (st : State) (cap : TreasuryCap) : Nat :=
  st.capTotal cap

/-- Supply invariant: wallet-visible supply never exceeds the treasury-cap
total supply, for every token type `T` that has a treasury cap. This is
validate_token_supply_bound in state/apply/mod.rs; it is what failed with
fungible token supply overcount in
usd_shared_cap_mint_pays_out_twice_to_any_recipient. -/
def SupplyInvariant (st : State)
    (coins : List Coin)
    (capOf : TokenType → Option TreasuryCap)
    : Prop :=
  ∀ (T : TokenType) (c : TreasuryCap), capOf T = some c →
    walletVisibleSupply coins T ≤ totalSupply st c

/-- The stronger bound `validate_token_supply_bound` actually checks:
wallet-visible supply plus object-locked supply must fit inside the
treasury-cap total. `locked` is the object-locked supply of the token
(`object_locked_supply_for_token`). -/
def AccountedInvariant (st : State)
    (coins : List Coin)
    (capOf : TokenType → Option TreasuryCap)
    (locked : TokenType → Nat)
    : Prop :=
  ∀ (T : TokenType) (c : TreasuryCap), capOf T = some c →
    walletVisibleSupply coins T ≤ totalSupply st c ∧
    walletVisibleSupply coins T + locked T ≤ totalSupply st c

/-- Full statement, kept for compatibility with the audit surface: the supply
invariant holds in the state. Note that this is a property of a *validated*
state, not of an arbitrary `State` value — `Counterexample.lean` proves an
arbitrary state can violate it, which is precisely why the runtime calls
`validate_cached_supply_invariants` after every change set. -/
def Statement (st : State)
    (coins : List Coin)
    (capOf : TokenType → Option TreasuryCap)
    : Prop :=
  SupplyInvariant st coins capOf

/-- Executable mirror of the `ensure!` in `validate_token_supply_bound`:
`wallet_visible_supply <= total_supply && accounted_supply <= total_supply`,
where `accounted_supply = wallet_visible_supply + object_locked_supply`
(with checked addition in Rust; `Nat` addition is unbounded here). -/
def tokenBoundHolds (total wallet locked : Nat) : Bool :=
  decide (wallet ≤ total) && decide (wallet + locked ≤ total)

/-- Executable mirror of `validate_cached_supply_invariants`: for every
tracked token type, look up its treasury cap and check the per-token bound.
A token type with no treasury cap is skipped (the invariant quantifies only
over token types that have a cap). -/
def cachedSupplyCheck (st : State)
    (coins : List Coin)
    (capOf : TokenType → Option TreasuryCap)
    (locked : TokenType → Nat)
    (tracked : List TokenType)
    : Bool :=
  tracked.all fun T =>
    match capOf T with
    | none => true
    | some cap => tokenBoundHolds (totalSupply st cap) (walletVisibleSupply coins T) (locked T)

end Supply

end KanariMoveRuntime
