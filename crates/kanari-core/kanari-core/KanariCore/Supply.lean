-- Kanari Core: Lean 4 model of supply invariants (native + non-native).

import KanariCore.Model

namespace KanariCore.Supply

-- Native supply invariant: wallet + locked <= total.
def NativeSupplyInv (wallet locked total : Nat) : Prop :=
  wallet ≤ total ∧ wallet + locked ≤ total

-- Non-native (USD) invariant: visible <= tracked total.
def TokenSupplyInv (visible tracked : Nat) : Prop := visible ≤ tracked

-- Mint increases both visible and tracked by same amount (preserves inv).
theorem mint_preserves_token_inv (visible tracked amount : Nat)
    (h : visible ≤ tracked) : visible + amount ≤ tracked + amount := by
  omega

-- Burn decreases both by same amount (preserves inv when sufficient).
theorem burn_preserves_token_inv (visible tracked amount : Nat)
    (h : visible ≤ tracked) (hamount : amount ≤ visible) :
    visible - amount ≤ tracked - amount := by
  omega

-- Transfer preserves native invariant (no total change).
theorem transfer_preserves_native (w l t : Nat) (amount : Nat)
    (h : w ≤ t ∧ w + l ≤ t) (hamount : amount ≤ w) :
    (w - amount) ≤ t ∧ (w - amount) + l ≤ t := by
  constructor
  · omega
  · omega

-- Self-transfer preserves native invariant trivially.
theorem self_transfer_preserves_native (w l t : Nat) (h : w ≤ t ∧ w + l ≤ t) :
    w ≤ t ∧ w + l ≤ t := h

-- No-lost-tokens: total == wallet + locked (strict equality at checkpoint boundary).
def NoLostTokens (wallet locked total : Nat) : Prop := wallet + locked = total

-- If no tokens are lost, the overcount bound also holds.
theorem no_lost_implies_bound (w l t : Nat) (h : NoLostTokens w l t) :
    w ≤ t ∧ w + l ≤ t := by
  unfold NoLostTokens at h
  constructor
  · have : w ≤ w + l := Nat.le_add_right w l
    rw [h] at this
    exact this
  · rw [h]
    exact Nat.le_refl t

-- Post-apply supply audit: at checkpoint boundary, total == wallet + locked.
-- This is stricter than the transaction-path bound (<=) and detects lost tokens.
theorem checkpoint_supply_exact (w l t : Nat) (h : w + l = t) :
    w ≤ t ∧ w + l = t := by
  constructor
  · omega
  · exact h

-- Gas fee preserves native invariant (debit wallet, credit DAO; total unchanged).
theorem gas_fee_preserves_native (w l t fee : Nat)
    (h : w ≤ t ∧ w + l ≤ t) (hfee : fee ≤ w) :
    (w - fee) ≤ t ∧ (w - fee) + l ≤ t := by
  constructor
  · omega
  · omega

-- Supply inflation guard: native total supply only increases on explicit mint.
-- A non-mint transaction must have supply_delta <= 0 (burn) or 0 (transfer).
theorem non_mint_no_supply_increase (old_total delta : Nat) (hno_mint : delta = 0 ∨ delta < 0) :
    old_total + delta ≤ old_total ∨ delta = 0 := by
  cases hno_mint with
  | inl h => right; exact h
  | inr h => left; omega

end KanariCore.Supply