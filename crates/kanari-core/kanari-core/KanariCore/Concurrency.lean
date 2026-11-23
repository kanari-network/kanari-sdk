-- Kanari Core: Lean 4 model of mempool access counts / concurrency.

import KanariCore.Model

namespace KanariCore.Concurrency

abbrev AccessKey := Nat

structure Counts where
  counts : AccessKey → Nat

def emptyCounts : Counts := { counts := fun _ => 0 }

def increment (c : Counts) (k : AccessKey) : Counts :=
  { counts := fun x => if x = k then c.counts x + 1 else c.counts x }

def decrement (c : Counts) (k : AccessKey) : Counts :=
  { counts := fun x => if x = k then c.counts x - 1 else c.counts x }

-- Increment then decrement returns to original for that key (when count was 0).
theorem increment_decrement (c : Counts) (k : AccessKey) (h : c.counts k = 0) :
    (decrement (increment c k) k).counts k = c.counts k := by
  unfold increment decrement
  simp [h]

-- Increment increases count by 1.
theorem increment_count (c : Counts) (k : AccessKey) :
    (increment c k).counts k = c.counts k + 1 := by
  unfold increment
  simp

-- Decrement of zero saturates (modelled as Nat sub; use saturating in Rust).
theorem decrement_saturates (c : Counts) (k : AccessKey) (h : c.counts k = 0) :
    (decrement c k).counts k = 0 := by
  unfold decrement
  simp [h]

-- Empty counts are zero.
theorem empty_counts_zero (k : AccessKey) : emptyCounts.counts k = 0 := rfl

-- Removing all pending txs resets counts to zero (modelled as fold).
def removeAll (c : Counts) : Counts := { counts := fun _ => 0 }

theorem removeAll_zero (c : Counts) (k : AccessKey) : (removeAll c).counts k = 0 := rfl

end KanariCore.Concurrency