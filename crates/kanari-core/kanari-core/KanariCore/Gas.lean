-- Kanari Core: Lean 4 model of gas accounting.

import KanariCore.Model

namespace KanariCore.Gas

def GasCost (limit price : Nat) : Nat := limit * price

-- Gas cost is monotonic in limit.
theorem gas_cost_monotonic_limit (l1 l2 price : Nat) (h : l1 ≤ l2) :
    GasCost l1 price ≤ GasCost l2 price := by
  unfold GasCost
  exact Nat.mul_le_mul_right price h

-- Gas cost is monotonic in price.
theorem gas_cost_monotonic_price (limit p1 p2 : Nat) (h : p1 ≤ p2) :
    GasCost limit p1 ≤ GasCost limit p2 := by
  unfold GasCost
  exact Nat.mul_le_mul_left limit h

-- Zero limit gives zero cost.
theorem gas_cost_zero_limit (price : Nat) : GasCost 0 price = 0 := by
  unfold GasCost
  simp

-- Zero price gives zero cost.
theorem gas_cost_zero_price (limit : Nat) : GasCost limit 0 = 0 := by
  unfold GasCost
  simp

-- Sufficient balance check.
def SufficientBalance (balance cost : Nat) : Prop := balance ≥ cost

-- If sufficient, remainder is non-negative.
theorem sufficient_remainder (balance cost : Nat) (h : SufficientBalance balance cost) :
    balance - cost + cost = balance := by
  unfold SufficientBalance at h
  exact Nat.sub_add_cancel h

end KanariCore.Gas