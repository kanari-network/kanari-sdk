import KanariMoveRuntime.Supply.Statement
import KanariMoveRuntime.Supply.Proof
open KanariMoveRuntime

/-
# Kanari Move Runtime v2 — supply invariant along an execution trace

`Proof.lean` proves the one-step mint lemma and the soundness of the
executable check. This file lifts those results to a whole execution trace,
which is the form the runtime cares about: `apply_changeset` validates after
every change set, so the invariant is inductive over the trace of
mint / burn / transfer steps.

The trace model is deliberately small:

* `Ledger.visible T` is the wallet-visible supply of `T`;
* `Ledger.total T` is the treasury-cap total supply of `T`;
* `Ledger.Inv` is `∀ T, visible T ≤ total T`;
* `mint` adds the same amount to both sides;
* `burn` subtracts the same amount from both sides (Move rejects underflow,
  so burn steps carry `amount ≤ visible T`);
* `transfer` changes neither side;
* `fundOutside` adds to the wallet-visible side only — the bug shape. Its
  theorem `fundOutside_breaks` is the formal statement that funding a wallet
  outside the treasury cap destroys the invariant, which is why the runtime
  must reject such states and why `usd_shared_cap_mint_pays_out_twice_to_any_recipient`
  was a real bug.

`ledgerOf` and `supplyInvariant_of_ledgerInv` connect this abstract model back
to the concrete `State`/`Coin` model of `Statement.lean`.
-/

namespace KanariMoveRuntime

namespace Supply

/-- Abstract per-token supply ledger: wallet-visible supply and treasury-cap
total supply, for every token type. -/
structure Ledger where
  /-- Wallet-visible supply per token type. -/
  visible : TokenType → Nat
  /-- Treasury-cap total supply per token type. -/
  total : TokenType → Nat

namespace Ledger

/-- The supply invariant on an abstract ledger. -/
def Inv (l : Ledger) : Prop :=
  ∀ T : TokenType, l.visible T ≤ l.total T

/-- Mint `amount` of `T`: both the wallet-visible and the cap total grow by
`amount`, exactly as `0x2::coin::mint` does through a `TreasuryCap<T>`. -/
def mint (l : Ledger) (T : TokenType) (amount : Nat) : Ledger where
  visible := fun T' => if T' = T then l.visible T' + amount else l.visible T'
  total := fun T' => if T' = T then l.total T' + amount else l.total T'

/-- Burn `amount` of `T`: both sides shrink by `amount`. Move rejects
underflow, so a burn step must carry `amount ≤ l.visible T`. -/
def burn (l : Ledger) (T : TokenType) (amount : Nat) : Ledger where
  visible := fun T' => if T' = T then l.visible T' - amount else l.visible T'
  total := fun T' => if T' = T then l.total T' - amount else l.total T'

/-- Transfer `amount` of `T` between wallets: neither side changes. -/
def transfer (l : Ledger) (_T : TokenType) (_amount : Nat) : Ledger := l

/-- The bug shape: credit the wallet with `amount` of `T` without minting
through the treasury cap, so the cap total is unchanged. This mirrors
`fund_sender_with_coin_type` in `engine_tests.rs`. -/
def fundOutside (l : Ledger) (T : TokenType) (amount : Nat) : Ledger where
  visible := fun T' => if T' = T then l.visible T' + amount else l.visible T'
  total := l.total

/-- Mint preserves the invariant. -/
theorem mint_preserves {l : Ledger} (h : l.Inv) (T : TokenType) (amount : Nat) :
    (l.mint T amount).Inv := by
  intro T'
  by_cases hT : T' = T
  · subst T'
    simp only [mint]
    have hb := h T
    split <;> omega
  · simp only [mint, hT]
    exact h T'

/-- Burn preserves the invariant, provided the burned amount is actually
held (Move's underflow check). -/
theorem burn_preserves {l : Ledger} (h : l.Inv) (T : TokenType) (amount : Nat)
    (hamount : amount ≤ l.visible T) :
    (l.burn T amount).Inv := by
  intro T'
  by_cases hT : T' = T
  · subst T'
    simp only [burn]
    have hb := h T
    split <;> omega
  · simp only [burn, hT]
    exact h T'

/-- Transfers preserve the invariant trivially: no supply is created or
destroyed by moving a coin between wallets. -/
theorem transfer_preserves {l : Ledger} (h : l.Inv) (T : TokenType) (amount : Nat) :
    (l.transfer T amount).Inv := h

/-- **Negative result / bug detector.** If the ledger is fully accounted
(`total T ≤ visible T`, e.g. every token was minted through its cap) then
funding the wallet with `amount > 0` of `T` outside the treasury cap breaks
the invariant. This is the formal counterpart of
`usd_shared_cap_mint_pays_out_twice_to_any_recipient`. -/
theorem fundOutside_breaks {l : Ledger} {T : TokenType} {amount : Nat}
    (hexhausted : l.total T ≤ l.visible T) (hpos : 0 < amount) :
    ¬ (l.fundOutside T amount).Inv := by
  intro hinv
  have hb := hinv T
  have hvis : (l.fundOutside T amount).visible T = l.visible T + amount := by
    simp [fundOutside]
  have htot : (l.fundOutside T amount).total T = l.total T := rfl
  rw [hvis, htot] at hb
  omega

end Ledger

/-- Reachability by well-formed supply operations: the initial ledger, then
any sequence of mint / burn (with a held amount) / transfer steps. -/
inductive Reachable (init : Ledger) : Ledger → Prop where
  | refl : Reachable init init
  | mint {l : Ledger} (h : Reachable init l) (T : TokenType) (amount : Nat) :
      Reachable init (l.mint T amount)
  | burn {l : Ledger} (h : Reachable init l) (T : TokenType) (amount : Nat)
      (hamount : amount ≤ l.visible T) :
      Reachable init (l.burn T amount)
  | transfer {l : Ledger} (h : Reachable init l) (T : TokenType) (amount : Nat) :
      Reachable init (l.transfer T amount)

/-- **Induction over the execution trace.** Every state reachable from an
invariant ledger by supply-accounted operations satisfies the invariant. -/
theorem reachable_invariant {init : Ledger} (hinit : init.Inv) {l : Ledger}
    (h : Reachable init l) : l.Inv := by
  induction h with
  | refl => exact hinit
  | mint h T amount ih => exact Ledger.mint_preserves ih T amount
  | burn h T amount hamount ih => exact Ledger.burn_preserves ih T amount hamount
  | transfer h T amount ih => exact Ledger.transfer_preserves ih T amount

/-- View a concrete runtime state as an abstract ledger. Uncapped token types
are given total supply `0`; for those the ledger invariant additionally
requires zero wallet-visible supply, which is what
`supplyInvariant_of_ledgerInv` uses. -/
def ledgerOf (st : State) (coins : List Coin)
    (capOf : TokenType → Option TreasuryCap) : Ledger where
  visible := fun T => walletVisibleSupply coins T
  total := fun T =>
    match capOf T with
    | some cap => totalSupply st cap
    | none => 0

/-- A concrete state whose ledger satisfies the abstract invariant satisfies
`SupplyInvariant`. -/
theorem supplyInvariant_of_ledgerInv {st : State} {coins : List Coin}
    {capOf : TokenType → Option TreasuryCap}
    (h : (ledgerOf st coins capOf).Inv) :
    SupplyInvariant st coins capOf := by
  intro T c hc
  have hb := h T
  simpa [ledgerOf, hc] using hb

/-- Conversely, if the concrete invariant holds and there are no wallet coins
of uncapped token types, the abstract ledger invariant holds. -/
theorem ledgerInv_of_supplyInvariant {st : State} {coins : List Coin}
    {capOf : TokenType → Option TreasuryCap}
    (hinv : SupplyInvariant st coins capOf)
    (hzero : ∀ T : TokenType, capOf T = none → walletVisibleSupply coins T = 0) :
    (ledgerOf st coins capOf).Inv := by
  intro T
  cases h : capOf T with
  | none => simp [ledgerOf, h, hzero T h]
  | some c => simpa [ledgerOf, h] using hinv T c h

end Supply

end KanariMoveRuntime
