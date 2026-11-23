module

/-!
Machine-checked model and proof of `src/scheduler.rs`
(`TransactionScheduler::schedule`).

The Rust scheduler greedily groups transactions into speculative waves of at
most `MAX_SPECULATIVE_WAVE_SIZE` (64) transactions, flushing the current wave
when it is full or when the next transaction conflicts with a key already
reserved by that wave. This module proves:

* the flattening of the produced waves is the input list (no loss/duplication);
* every wave is non-empty;
* with `0 < max`, every wave has length at most `max`;
* transactions in one wave have pairwise disjoint conflict keys.

Nothing outside this Lean model is claimed. -/

noncomputable section

@[expose] public section

namespace KanariMoveRuntime
namespace SchedulerProof

/-- Minimal transaction model: an identity and its conflict keys. -/
structure Tx (α : Type) where
  id : Nat
  keys : List α

/-- Accumulator state corresponding to the three Rust locals:
`scheduled`, `current_wave`, `current_reserved_keys`. -/
structure SchedState (α : Type) where
  done : List (List (Tx α))
  wave : List (Tx α)
  reserved : List α

/-- Insert a key into a reserved-key list, preserving deduplication. -/
def reserveKeys [DecidableEq α] : List α → List α → List α
  | [], acc => acc
  | k :: ks, acc => reserveKeys ks (if k ∈ acc then acc else k :: acc)

/-- Existing reserved keys survive further insertions. -/
theorem mem_reserveKeys_of_acc [DecidableEq α] {keys acc : List α} {k : α}
    (hk : k ∈ acc) : k ∈ reserveKeys keys acc := by
  induction keys generalizing acc with
  | nil => simpa [reserveKeys] using hk
  | cons k0 ks ih =>
      change k ∈ reserveKeys ks (if k0 ∈ acc then acc else k0 :: acc)
      by_cases h : k0 ∈ acc
      · simpa [h] using ih (acc := acc) hk
      · simpa [h] using ih (acc := k0 :: acc) (by simp [hk])

/-- Every inserted key is present in the resulting reserved set. -/
theorem mem_reserveKeys_of_mem [DecidableEq α] {keys acc : List α} {k : α}
    (hk : k ∈ keys) : k ∈ reserveKeys keys acc := by
  induction keys generalizing acc with
  | nil => simp at hk
  | cons k0 ks ih =>
      simp only [List.mem_cons] at hk
      change k ∈ reserveKeys ks (if k0 ∈ acc then acc else k0 :: acc)
      rcases hk with hk0 | hk
      · subst k0
        by_cases h : k ∈ acc
        · simpa [h] using mem_reserveKeys_of_acc (keys := ks) (acc := acc) h
        · simpa [h] using
            (mem_reserveKeys_of_acc (keys := ks) (acc := k :: acc) (by simp))
      · by_cases h : k0 ∈ acc
        · simpa [h] using ih (acc := acc) hk
        · simpa [h] using ih (acc := k0 :: acc) (by simp [hk])

/-- Every key in the result came from the inserted keys or the old set. -/
theorem mem_of_mem_reserveKeys [DecidableEq α] {keys acc : List α} {k : α}
    (hk : k ∈ reserveKeys keys acc) : k ∈ keys ∨ k ∈ acc := by
  induction keys generalizing acc with
  | nil =>
      exact Or.inr (by simpa [reserveKeys] using hk)
  | cons k' ks ih =>
      by_cases h : k' ∈ acc
      · have hres := ih (acc := acc) (by simpa [reserveKeys, h] using hk)
        rcases hres with hres | hres
        · exact Or.inl (by simp [hres])
        · exact Or.inr hres
      · have hres := ih (acc := k' :: acc) (by simpa [reserveKeys, h] using hk)
        rcases hres with hres | hres
        · exact Or.inl (by simp [hres])
        · simp only [List.mem_cons] at hres
          rcases hres with rfl | hres
          · exact Or.inl (by simp)
          · exact Or.inr hres

/-- `conflict_keys` of the next transaction intersects the current wave's
reserved keys. -/
def conflicts [DecidableEq α] (tx : Tx α) (reserved : List α) : Prop :=
  ∃ k ∈ tx.keys, k ∈ reserved

/-- The flush condition used by `schedule`: the wave is non-empty and either
full or conflicting with the next transaction. -/
def shouldFlush [DecidableEq α] (max : Nat) (s : SchedState α) (tx : Tx α) : Prop :=
  s.wave ≠ [] ∧ (max ≤ s.wave.length ∨ conflicts tx s.reserved)

/-- One scheduling step. -/
def step [DecidableEq α] (max : Nat) (s : SchedState α) (tx : Tx α) : SchedState α := by
  classical
  exact if shouldFlush max s tx then
    let s1 : SchedState α := { done := s.done ++ [s.wave], wave := [], reserved := [] }
    { s1 with wave := [tx], reserved := reserveKeys tx.keys [] }
  else
    { s with wave := s.wave ++ [tx], reserved := reserveKeys tx.keys s.reserved }

/-- Flush the last non-empty wave after the loop, as Rust does. -/
def finalize [DecidableEq α] (s : SchedState α) : SchedState α :=
  if s.wave = [] then s
  else { done := s.done ++ [s.wave], wave := [], reserved := [] }

/-- Scheduler state after folding all transactions and finalizing. -/
def scheduleState [DecidableEq α] (max : Nat) (txs : List (Tx α)) : SchedState α :=
  finalize (txs.foldl (step max) { done := [], wave := [], reserved := [] })

/-- The scheduler: fold over transactions, then flush the final wave. -/
def schedule [DecidableEq α] (max : Nat) (txs : List (Tx α)) : List (List (Tx α)) :=
  (scheduleState max txs).done

/-- Two transactions in one wave must not share a conflict key. -/
def DisjointWave (wave : List (Tx α)) : Prop :=
  ∀ t₁ ∈ wave, ∀ t₂ ∈ wave, t₁ ≠ t₂ → ∀ k ∈ t₁.keys, k ∉ t₂.keys

/-- State invariant kept by every fold step. -/
def Inv [DecidableEq α] (max : Nat) (s : SchedState α) : Prop :=
  (∀ w ∈ s.done, w ≠ [] ∧ w.length ≤ max ∧ DisjointWave w) ∧
  (s.wave ≠ [] → s.wave.length ≤ max ∧ DisjointWave s.wave) ∧
  (∀ t ∈ s.wave, ∀ k ∈ t.keys, k ∈ s.reserved) ∧
  (∀ k ∈ s.reserved, ∃ t ∈ s.wave, k ∈ t.keys)

/-- A step preserves the flattening of done waves plus the current wave. -/
theorem step_flatten [DecidableEq α] (max : Nat) (s : SchedState α) (tx : Tx α) :
    (step max s tx).done.flatten ++ (step max s tx).wave =
      s.done.flatten ++ s.wave ++ [tx] := by
  classical
  unfold step
  by_cases h : shouldFlush max s tx
  · simp [h]
  · simp [h, List.append_assoc]

/-- Fold invariant for flattening. -/
theorem foldl_flatten_inv [DecidableEq α] (max : Nat) :
    ∀ (txs : List (Tx α)) (s : SchedState α),
      (txs.foldl (step max) s).done.flatten ++ (txs.foldl (step max) s).wave =
        s.done.flatten ++ s.wave ++ txs := by
  intro txs
  induction txs with
  | nil => intro s; simp
  | cons tx txs ih =>
      intro s
      simp only [List.foldl_cons]
      rw [ih (step max s tx)]
      rw [step_flatten]
      simp [List.append_assoc]

/-- **No loss, no duplication, no reordering:** flattening the waves returns
exactly the input transaction list. -/
theorem schedule_flatten [DecidableEq α] (max : Nat) (txs : List (Tx α)) :
    (schedule max txs).flatten = txs := by
  unfold schedule scheduleState finalize
  have h := foldl_flatten_inv max txs ({ done := [], wave := [], reserved := [] } : SchedState α)
  simp at h
  by_cases hw : (txs.foldl (step max)
      ({ done := [], wave := [], reserved := [] } : SchedState α)).wave = []
  · simpa [hw] using h
  · simpa [hw] using h

/-- A step never commits an empty wave. -/
theorem step_done_nonempty [DecidableEq α] (max : Nat) {s : SchedState α}
    (hne : ∀ w ∈ s.done, w ≠ []) (tx : Tx α) :
    ∀ w ∈ (step max s tx).done, w ≠ [] := by
  classical
  unfold step
  split
  · rename_i h
    intro w hw
    simp only [List.mem_append, List.mem_singleton] at hw
    rcases hw with hw | rfl
    · exact hne w hw
    · exact h.1
  · exact hne

/-- Fold invariant for non-empty committed waves. -/
theorem foldl_done_nonempty [DecidableEq α] (max : Nat) :
    ∀ (txs : List (Tx α)) (s : SchedState α),
      (∀ w ∈ s.done, w ≠ []) →
        ∀ w ∈ (txs.foldl (step max) s).done, w ≠ [] := by
  intro txs
  induction txs with
  | nil => intro s hs; simpa
  | cons tx txs ih =>
      intro s hs
      exact ih (step max s tx) (step_done_nonempty max hs tx)

/-- The state invariant is preserved by one step. -/
theorem step_inv [DecidableEq α] {max : Nat} (hmax : 0 < max) {s : SchedState α}
    (hs : Inv max s) (tx : Tx α) : Inv max (step max s tx) := by
  classical
  rcases hs with ⟨hdone, hwaveSize, hwaveKeys, hresKeys⟩
  unfold step
  split
  · rename_i hflush
    dsimp [Inv]
    constructor
    · intro w hw
      simp only [List.mem_append, List.mem_singleton] at hw
      rcases hw with hw | rfl
      · exact hdone w hw
      · exact ⟨hflush.1, (hwaveSize hflush.1).1, (hwaveSize hflush.1).2⟩
    constructor
    · intro _
      constructor
      · omega
      · intro t₁ ht₁ t₂ ht₂ hne k hk
        simp only [List.mem_singleton] at ht₁ ht₂
        subst t₁
        subst t₂
        exact (hne rfl).elim
    constructor
    · intro t ht k hk
      have ht' : t = tx := by simpa using ht
      subst t
      exact mem_reserveKeys_of_mem hk
    · intro k hk
      rcases mem_of_mem_reserveKeys hk with hknew | hkold
      · exact ⟨tx, by simp, hknew⟩
      · simp at hkold
  · rename_i hflush
    dsimp [Inv]
    constructor
    · exact hdone
    constructor
    · intro _
      by_cases hsEmpty : s.wave = []
      · constructor
        · simp [hsEmpty]
          omega
        · intro t₁ ht₁ t₂ ht₂ hne k hk
          have ht₁' : t₁ = tx := by simpa [hsEmpty] using ht₁
          have ht₂' : t₂ = tx := by simpa [hsEmpty] using ht₂
          subst t₁
          subst t₂
          exact (hne rfl).elim
      · constructor
        · have hsize := (hwaveSize hsEmpty).1
          have hnotfull : ¬ max ≤ s.wave.length := by
            intro hfull
            exact hflush ⟨hsEmpty, Or.inl hfull⟩
          have hlt : s.wave.length < max := Nat.lt_of_not_ge hnotfull
          simp
          omega
        · intro t₁ ht₁ t₂ ht₂ hne k hk₁
          simp only [List.mem_append, List.mem_singleton] at ht₁ ht₂
          rcases ht₁ with ht₁ | rfl
          · rcases ht₂ with ht₂ | rfl
            · exact (hwaveSize hsEmpty).2 t₁ ht₁ t₂ ht₂ hne k hk₁
            · intro hk₂
              have hkres : k ∈ s.reserved := hwaveKeys t₁ ht₁ k hk₁
              exact hflush ⟨hsEmpty, Or.inr ⟨k, hk₂, hkres⟩⟩
          · rcases ht₂ with ht₂ | rfl
            · intro hk₂
              have hkres : k ∈ s.reserved := hwaveKeys t₂ ht₂ k hk₂
              exact hflush ⟨hsEmpty, Or.inr ⟨k, hk₁, hkres⟩⟩
            · exact (hne rfl).elim
    constructor
    · intro t ht k hk
      simp only [List.mem_append, List.mem_singleton] at ht
      rcases ht with ht | rfl
      · exact mem_reserveKeys_of_acc (keys := tx.keys) (hwaveKeys t ht k hk)
      · exact mem_reserveKeys_of_mem hk
    · intro k hk
      rcases mem_of_mem_reserveKeys hk with hknew | hkold
      · exact ⟨tx, by simp, hknew⟩
      · rcases hresKeys k hkold with ⟨t, ht, hkt⟩
        exact ⟨t, by simp [ht], hkt⟩

/-- Finalizing a non-empty wave preserves the invariant. -/
theorem finalize_inv [DecidableEq α] {max : Nat} {s : SchedState α}
    (hs : Inv max s) : Inv max (finalize s) := by
  classical
  unfold finalize
  split
  · exact hs
  · rename_i hw
    dsimp [Inv]
    rcases hs with ⟨hdone, hwaveSize, hwaveKeys, hresKeys⟩
    have hwprop := hwaveSize hw
    constructor
    · intro w hmem
      simp only [List.mem_append, List.mem_singleton] at hmem
      rcases hmem with hmem | rfl
      · exact hdone w hmem
      · exact ⟨hw, hwprop.1, hwprop.2⟩
    · simp

/-- The state invariant is preserved by the whole fold. -/
theorem foldl_inv [DecidableEq α] (max : Nat) (hmax : 0 < max) :
    ∀ (txs : List (Tx α)) (s : SchedState α), Inv max s →
      Inv max (txs.foldl (step max) s) := by
  intro txs
  induction txs with
  | nil => intro s hs; simpa
  | cons tx txs ih =>
      intro s hs
      exact ih (step max s tx) (step_inv hmax hs tx)

/-- The scheduler state satisfies the invariant. -/
theorem schedule_inv [DecidableEq α] (max : Nat) (hmax : 0 < max) (txs : List (Tx α)) :
    Inv max (scheduleState max txs) := by
  unfold scheduleState
  apply finalize_inv
  apply foldl_inv max hmax txs
  simp [Inv]

/-- Every committed wave is non-empty. -/
theorem schedule_waves_nonempty [DecidableEq α] (max : Nat) (hmax : 0 < max)
    (txs : List (Tx α)) : ∀ w ∈ schedule max txs, w ≠ [] := by
  intro w hw
  have hw' : w ∈ (scheduleState max txs).done := by simpa [schedule] using hw
  exact ((schedule_inv max hmax txs).1 w hw').1

/-- **Size bound:** with a positive maximum, no wave exceeds `max`
transactions (`MAX_SPECULATIVE_WAVE_SIZE = 64` in Rust). -/
theorem schedule_wave_size [DecidableEq α] {max : Nat} (hmax : 0 < max)
    {txs : List (Tx α)} {w : List (Tx α)} (hw : w ∈ schedule max txs) :
    w.length ≤ max := by
  have hw' : w ∈ (scheduleState max txs).done := by simpa [schedule] using hw
  exact ((schedule_inv max hmax txs).1 w hw').2.1

/-- **Conflict freedom:** no two transactions in the same wave share a
conflict key. -/
theorem schedule_wave_disjoint [DecidableEq α] {max : Nat} (hmax : 0 < max)
    {txs : List (Tx α)} {w : List (Tx α)} (hw : w ∈ schedule max txs) :
    DisjointWave w := by
  have hw' : w ∈ (scheduleState max txs).done := by simpa [schedule] using hw
  exact ((schedule_inv max hmax txs).1 w hw').2.2

end SchedulerProof
end KanariMoveRuntime
