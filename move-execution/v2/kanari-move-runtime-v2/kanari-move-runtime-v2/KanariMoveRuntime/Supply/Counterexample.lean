import KanariMoveRuntime.Supply.Statement
import KanariMoveRuntime.Supply.Proof
open KanariMoveRuntime

/-
# Kanari Move Runtime v2 — the funding bug is rejected

Concrete instance of `usd_shared_cap_mint_pays_out_twice_to_any_recipient`
(`crates/kanari-core/tests/unit/engine_tests.rs`): a wallet is funded with
`5` units of `0x2::usd::USD` *outside* the shared treasury cap, so the wallet
holds USD while `TreasuryCap<USD>.total_supply` is still `0`.

This file proves:

* the wallet-visible supply of USD is `5`;
* the treasury-cap total supply of USD is `0`;
* the supply invariant therefore *fails* on that state (`invariant_fails`);
* `validate_cached_supply_invariants` rejects it (`check_rejects`).

Together with `Proof.lean` this pins down the exact contract: the invariant is
not true of arbitrary states, it is true of states the runtime admits, and the
checker rejects the bug state.
-/

namespace KanariMoveRuntime

namespace Counterexample

open Supply

/-- `0x2::usd::USD`, the token involved in the regression test. -/
def usd : TokenType :=
  ⟨"0x2::usd::USD", false⟩

/-- The pre-funded wallet. -/
def alice : Address :=
  ⟨"0xa11ce"⟩

/-- The shared-object owner of the USD treasury cap. -/
def treasuryOwner : Address :=
  ⟨"0x2"⟩

/-- The USD treasury cap. Its total supply is `0` because no USD was minted
through it. -/
def usdCap : TreasuryCap where
  total_supply := 0
  owner := treasuryOwner
  tokenType := usd

/-- An otherwise empty runtime state. -/
def emptyState : State where
  tokenSupplies := fun _ => 0
  capTotal := fun _ => 0
  supplyRecord := fun _ => 0

/-- `capOf` maps USD to its shared treasury cap and nothing else. -/
def capOfUSD (T : TokenType) : Option TreasuryCap :=
  if T = usd then some usdCap else none

/-- The gas object after `fund_sender_with_coin_type(USD, 5)`: one USD coin
of balance `5` owned by `alice`, with no matching mint. -/
def fundedGasObject : List Coin :=
  [⟨usd, alice, 5⟩]

theorem capOfUSD_usd : capOfUSD usd = some usdCap := by
  simp [capOfUSD]

theorem walletVisibleSupply_funded :
    walletVisibleSupply fundedGasObject usd = 5 := by
  simp [walletVisibleSupply, coinSupplyOf, fundedGasObject, usd, List.foldr]

theorem totalSupply_usdCap :
    totalSupply emptyState usdCap = 0 := rfl

/-- The concrete invariant does not hold on the bug state. -/
theorem invariant_fails :
    ¬ SupplyInvariant emptyState fundedGasObject capOfUSD := by
  intro hinv
  have hb := hinv usd usdCap capOfUSD_usd
  rw [walletVisibleSupply_funded, totalSupply_usdCap] at hb
  omega

/-- `validate_cached_supply_invariants` returns an error on the bug state:
the checker rejects `wallet_visible_supply (5) <= total_supply (0)`. -/
theorem check_rejects :
    cachedSupplyCheck emptyState fundedGasObject capOfUSD (fun _ => 0) [usd] = false := by
  simp [cachedSupplyCheck, capOfUSD_usd, tokenBoundHolds,
    walletVisibleSupply_funded, totalSupply_usdCap]

end Counterexample

end KanariMoveRuntime
