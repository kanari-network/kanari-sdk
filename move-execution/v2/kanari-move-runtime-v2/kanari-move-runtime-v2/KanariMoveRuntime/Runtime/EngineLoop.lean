module

/-!
# Conditional model of the wave engine loop

`Runtime/SchedulerProof.lean` proves properties of `schedule`. This module
models its consumer loop (`crates/kanari-core/src/engine.rs::
execute_tx_waves_parallel_inner`): every wave is executed speculatively
against one snapshot, committed only when its access footprint is independent
and the post-checks pass, and otherwise replayed serially in input order.

`engineFold_eq_seqFold` is **conditional**: it assumes `EngineOK`, i.e.

* `exec` is stable under agreement on its declared read keys — the
  extensional form of "the access manifest covers every key the execution
  reads";
* every key actually written by a changeset is in the declared write set —
  the extensional form of "the manifest covers every write".

Under those two locality hypotheses no further semantic assumption is used:
independent waves commute on their read keys, so speculative results equal
serial results, and the retry branch is a serial fold by construction. This
is a theorem about the model, not a proof of the Rust engine, the epoch
fence, or the VM. -/

noncomputable section

@[expose] public section

namespace KanariMoveRuntime
namespace EngineLoop

universe u v w

/-- A key/value store; abstracts RocksDB plus overlays. -/
abbrev Store (K : Type u) (V : Type v) := K → V

/-- A changeset is a key-indexed set of optional writes (`none` = no write). -/
abbrev CS (K : Type u) (V : Type v) := K → Option V

/-- Apply a changeset to a store. -/
def applyCS (s : Store K V) (cs : CS K V) : Store K V :=
  fun k => match cs k with
  | some v => v
  | none => s k

/-- Deterministic transaction engine plus the fail-closed checks. -/
structure Engine (K : Type u) (V : Type v) (T : Type w) where
  reads : T → List K
  writes : T → List K
  exec : Store K V → T → CS K V
  failed : CS K V → Bool
  commit : Store K V → List (CS K V) → Bool

/-- Locality hypotheses: read-set and write-set completeness. -/
def EngineOK (eng : Engine K V T) : Prop :=
  (∀ (s s' : Store K V) (t : T),
      (∀ k ∈ eng.reads t, s k = s' k) → eng.exec s t = eng.exec s' t) ∧
    (∀ (s : Store K V) (t : T) (k : K),
      eng.exec s t k ≠ none → k ∈ eng.writes t)

/-- Two transactions are independent when neither writes a key the other
reads or writes. Read/read overlap is allowed. -/
def Independent (eng : Engine K V T) (wave : List T) : Prop :=
  ∀ t₁ ∈ wave, ∀ t₂ ∈ wave, t₁ ≠ t₂ →
    (∀ k ∈ eng.writes t₁, k ∉ eng.writes t₂) ∧
    (∀ k ∈ eng.writes t₁, k ∉ eng.reads t₂) ∧
    (∀ k ∈ eng.writes t₂, k ∉ eng.reads t₁)

/-- Decidable independence test used by the model. `overlap a b` is true when
the two key lists share a key; a wave is independent when no later pair
overlaps an earlier pair's writes. -/
def overlap [DecidableEq K] (a b : List K) : Bool :=
  a.any fun k => k ∈ b

def independentSteps [DecidableEq K] (pairs : List (List K × List K)) : Bool :=
  match pairs with
  | [] => true
  | p :: rest =>
      (!rest.any fun q =>
        overlap p.1 q.1 || overlap p.1 q.2 || overlap q.1 p.2) &&
        independentSteps rest

theorem noOverlap_of_overlap_false [DecidableEq K] {a b : List K}
    (h : overlap a b = false) : ∀ k ∈ a, k ∉ b := by
  intro k hk hkb
  have ht : overlap a b = true :=
    List.any_eq_true.mpr ⟨k, hk, by simp [hkb]⟩
  simp [h] at ht

theorem independentSteps_of_append [DecidableEq K] {p : List K × List K}
    {pre rest : List (List K × List K)}
    (h : independentSteps (pre ++ p :: rest) = true) :
    independentSteps (p :: rest) = true := by
  induction pre with
  | nil => simpa using h
  | cons q qs ih =>
      change independentSteps (q :: (qs ++ p :: rest)) = true at h
      simp only [independentSteps, Bool.and_eq_true] at h
      exact ih h.2

theorem find_pair_either {f : T → List K × List K} {wave : List T}
    {t₁ t₂ : T} (ht₁ : t₁ ∈ wave) (ht₂ : t₂ ∈ wave) (hne : t₁ ≠ t₂) :
    (∃ pre rest : List (List K × List K),
      wave.map f = pre ++ f t₁ :: rest ∧ f t₂ ∈ rest) ∨
    (∃ pre rest : List (List K × List K),
      wave.map f = pre ++ f t₂ :: rest ∧ f t₁ ∈ rest) := by
  induction wave generalizing t₁ t₂ with
  | nil => simp at ht₁
  | cons t ts ih =>
      simp only [List.map_cons]
      rcases List.mem_cons.mp ht₁ with ht₁head | ht₁tail
      · subst t₁
        rcases List.mem_cons.mp ht₂ with ht₂head | ht₂tail
        · subst t₂
          exact (hne rfl).elim
        · left
          exact ⟨[], ts.map f, by simp, List.mem_map.mpr ⟨t₂, ht₂tail, rfl⟩⟩
      · rcases List.mem_cons.mp ht₂ with ht₂head | ht₂tail
        · subst t₂
          right
          exact ⟨[], ts.map f, by simp, List.mem_map.mpr ⟨t₁, ht₁tail, rfl⟩⟩
        · rcases ih ht₁tail ht₂tail hne with hres | hres
          · rcases hres with ⟨pre, rest, happ, hmemrest⟩
            left
            exact ⟨f t :: pre, rest, by simp [happ], hmemrest⟩
          · rcases hres with ⟨pre, rest, happ, hmemrest⟩
            right
            exact ⟨f t :: pre, rest, by simp [happ], hmemrest⟩

theorem overlap_false_of_mem [DecidableEq K] {p q : List K × List K}
    {rest : List (List K × List K)} (hq : q ∈ rest)
    (hanyfalse : (rest.any fun r =>
        overlap p.1 r.1 || overlap p.1 r.2 || overlap r.1 p.2) = false) :
    overlap p.1 q.1 = false ∧ overlap p.1 q.2 = false ∧ overlap q.1 p.2 = false := by
  have hnot : ¬ (overlap p.1 q.1 || overlap p.1 q.2 || overlap q.1 p.2 = true) := by
    intro ht
    have hanytrue : (rest.any fun r =>
        overlap p.1 r.1 || overlap p.1 r.2 || overlap r.1 p.2) = true :=
      List.any_eq_true.mpr ⟨q, hq, ht⟩
    simp [hanyfalse] at hanytrue
  refine ⟨?_, ?_, ?_⟩
  · cases h1 : overlap p.1 q.1 <;> cases h2 : overlap p.1 q.2 <;>
      cases h3 : overlap q.1 p.2 <;> simp_all
  · cases h1 : overlap p.1 q.1 <;> cases h2 : overlap p.1 q.2 <;>
      cases h3 : overlap q.1 p.2 <;> simp_all
  · cases h1 : overlap p.1 q.1 <;> cases h2 : overlap p.1 q.2 <;>
      cases h3 : overlap q.1 p.2 <;> simp_all

theorem independentSteps_sound {eng : Engine K V T} [DecidableEq K]
    {wave : List T} {t₁ : T}
    (ht₁ : t₁ ∈ wave)
    (h : independentSteps (wave.map fun t => (eng.writes t, eng.reads t)) = true)
    {t₂ : T} (ht₂ : t₂ ∈ wave) (hne : t₁ ≠ t₂) :
    (∀ k ∈ eng.writes t₁, k ∉ eng.writes t₂) ∧
    (∀ k ∈ eng.writes t₁, k ∉ eng.reads t₂) ∧
    (∀ k ∈ eng.writes t₂, k ∉ eng.reads t₁) := by
  classical
  rcases find_pair_either (f := fun t => (eng.writes t, eng.reads t))
      ht₁ ht₂ hne with hbefore | hafter
  · rcases hbefore with ⟨pre, rest, happ, hrest⟩
    have htail := independentSteps_of_append (p := (eng.writes t₁, eng.reads t₁))
      (pre := pre) (rest := rest) (by simpa [happ] using h)
    have htail' := Bool.and_eq_true_iff.mp htail
    have hanyfalse : (rest.any fun q =>
        overlap (eng.writes t₁) q.1 || overlap (eng.writes t₁) q.2 ||
          overlap q.1 (eng.reads t₁)) = false := by
      simpa using htail'.1
    have hqfalse := overlap_false_of_mem (p := (eng.writes t₁, eng.reads t₁))
      (q := (eng.writes t₂, eng.reads t₂)) hrest hanyfalse
    exact ⟨noOverlap_of_overlap_false hqfalse.1,
      noOverlap_of_overlap_false hqfalse.2.1,
      noOverlap_of_overlap_false hqfalse.2.2⟩
  · rcases hafter with ⟨pre, rest, happ, hrest⟩
    have htail := independentSteps_of_append (p := (eng.writes t₂, eng.reads t₂))
      (pre := pre) (rest := rest) (by simpa [happ] using h)
    have htail' := Bool.and_eq_true_iff.mp htail
    have hanyfalse : (rest.any fun q =>
        overlap (eng.writes t₂) q.1 || overlap (eng.writes t₂) q.2 ||
          overlap q.1 (eng.reads t₂)) = false := by
      simpa using htail'.1
    have hqfalse := overlap_false_of_mem (p := (eng.writes t₂, eng.reads t₂))
      (q := (eng.writes t₁, eng.reads t₁)) hrest hanyfalse
    constructor
    · intro k hk₁ hk₂
      exact noOverlap_of_overlap_false hqfalse.1 k hk₂ hk₁
    · exact ⟨noOverlap_of_overlap_false hqfalse.2.2,
        noOverlap_of_overlap_false hqfalse.2.1⟩

/-- Speculative execution of one wave. -/
def engineWave [DecidableEq K] [DecidableEq T] (eng : Engine K V T)
    (s : Store K V) (wave : List T) : Option (List (CS K V)) :=
  let cs := wave.map (eng.exec s)
  if cs.any eng.failed then none
  else if independentSteps (wave.map fun t => (eng.writes t, eng.reads t)) &&
      eng.commit s cs then some cs else none

/-- Strict sequential reference semantics. -/
def seqFold (eng : Engine K V T) (s : Store K V) (txs : List T) : Store K V :=
  txs.foldl (fun st t => applyCS st (eng.exec st t)) s

/-- The wave engine: commit or serially replay each wave. -/
def engineFold [DecidableEq K] [DecidableEq T] (eng : Engine K V T)
    (waves : List (List T)) (s : Store K V) : Store K V :=
  waves.foldl (fun st wave =>
    match engineWave eng st wave with
    | some cs => cs.foldl applyCS st
    | none => seqFold eng st wave) s

/-- Applying a changeset cannot change a key it does not write. -/
theorem applyCS_eq_of_not_mem {s : Store K V} {cs : CS K V}
    {k : K} {writes : List K}
    (hnot : k ∉ writes) (hwr : ∀ k', cs k' ≠ none → k' ∈ writes) :
    applyCS s cs k = s k := by
  unfold applyCS
  by_cases hc : cs k = none
  · simp [hc]
  · exact False.elim (hnot (hwr k hc))

set_option linter.unusedVariables false in

/-- Folding a prefix of snapshot changesets preserves snapshot values of keys
that no changeset in the prefix writes. -/
theorem fold_preserves_keys {eng : Engine K V T}
    (heng : EngineOK eng) {snap : Store K V} :
    ∀ (pre : List T) (precs : List (CS K V)) (rest : List T),
      precs = pre.map (eng.exec snap) →
      (∀ t ∈ pre, ∀ t' ∈ rest, ∀ k ∈ eng.writes t, k ∉ eng.reads t') →
      ∀ (s₀ : Store K V) (t' : T) (ht' : t' ∈ rest) (k : K)
        (hk : k ∈ eng.reads t'),
        (precs.foldl applyCS s₀) k = s₀ k := by
  intro pre
  induction pre with
  | nil =>
      intro precs rest hcs hdisj s₀ t' ht' k hk
      simp at hcs
      subst precs
      simp
  | cons t pre ih =>
      intro precs rest hcs hdisj s₀ t' ht' k hk
      cases precs with
      | nil => simp at hcs
      | cons c precs =>
          injection hcs with hc_eq hprecs
          have hwr_t : ∀ k', c k' ≠ none → k' ∈ eng.writes t := by
            intro k' hk'
            rw [hc_eq] at hk'
            exact heng.2 snap t k' hk'
          have hdisj_t : ∀ k' ∈ eng.writes t, k' ∉ eng.reads t' :=
            hdisj t (by simp) t' ht'
          have hstep : applyCS s₀ c k = s₀ k :=
            applyCS_eq_of_not_mem (fun hkmem => hdisj_t k hkmem hk) hwr_t
          have hdisj_tail : ∀ u ∈ pre, ∀ v ∈ rest, ∀ k ∈ eng.writes u, k ∉ eng.reads v := by
            intro u hu v hv k hk
            exact hdisj u (by simp [hu]) v hv k hk
          have htail := ih precs rest hprecs hdisj_tail (applyCS s₀ c) t' ht' k hk
          exact htail.trans hstep

/-- A value cannot occur on both sides of an append of a nodup list. -/
theorem not_mem_right_of_mem_left_append {xs ys : List T} (hnd : (xs ++ ys).Nodup)
    {x : T} (hx : x ∈ xs) : x ∉ ys := by
  induction xs with
  | nil => simp at hx
  | cons y ys' ih =>
      simp only [List.mem_cons] at hx
      rcases List.nodup_cons.mp hnd with ⟨hy, hnd_tail⟩
      rcases hx with rfl | hx
      · intro hmem
        exact hy (List.mem_append.mpr (Or.inr hmem))
      · exact ih hnd_tail hx

/-- Tail of an independent wave is independent. -/
theorem independent_tail {eng : Engine K V T} {t : T} {ts : List T}
    (h : Independent eng (t :: ts)) : Independent eng ts := by
  intro t₁ ht₁ t₂ ht₂ hne
  exact h t₁ (by simp [ht₁]) t₂ (by simp [ht₂]) hne

/-- Once a wave is independent with distinct entries, each transaction
executes identically against the snapshot and against its serial prefix. -/
theorem exec_prefix_eq_snapshot {eng : Engine K V T}
    (heng : EngineOK eng) {snap : Store K V} {wave : List T}
    (hnd : wave.Nodup) (hind : Independent eng wave) :
    ∀ (pre : List T) (precs : List (CS K V)) (rest : List T),
      wave = pre ++ rest →
      precs = pre.map (eng.exec snap) →
      ∀ t' ∈ rest, eng.exec (precs.foldl applyCS snap) t' = eng.exec snap t' := by
  intro pre precs rest hwave hprecs t' ht'
  apply heng.1
  intro k hk
  have hdisj : ∀ t ∈ pre, ∀ u ∈ rest, ∀ k ∈ eng.writes t, k ∉ eng.reads u := by
    intro t ht u hu k hk
    have htw : t ∈ wave := by simpa [hwave] using List.mem_append.mpr (Or.inl ht)
    have huw : u ∈ wave := by simpa [hwave] using List.mem_append.mpr (Or.inr hu)
    have htne : t ≠ u := by
      intro htu
      subst u
      exact not_mem_right_of_mem_left_append (by simpa [hwave] using hnd) ht hu
    exact (hind t htw u huw htne).2.1 k hk
  exact fold_preserves_keys (eng := eng) heng (snap := snap)
    pre precs rest hprecs hdisj snap t' ht' k hk

/-- Pointwise equality of executions lifts to a mapped list. -/
theorem map_exec_eq_of_pointwise {eng : Engine K V T} {s s' : Store K V}
    {ts : List T} (h : ∀ t ∈ ts, eng.exec s t = eng.exec s' t) :
    ts.map (eng.exec s) = ts.map (eng.exec s') := by
  induction ts with
  | nil => simp
  | cons t ts ih =>
      simp [h t (by simp), ih (fun t' ht' => h t' (by simp [ht']))]

/-- A committed independent wave has the same final store as a serial fold. -/
theorem wave_commit_eq_seq {eng : Engine K V T} (heng : EngineOK eng)
    {snap : Store K V} {wave : List T}
    (hnd : wave.Nodup) (hind : Independent eng wave)
    {cs : List (CS K V)} (hcs : cs = wave.map (eng.exec snap)) :
    cs.foldl applyCS snap = seqFold eng snap wave := by
  induction wave generalizing cs snap with
  | nil =>
      simp [hcs, seqFold]
  | cons t ts ih =>
      simp only [List.map_cons] at hcs
      rcases hcs with ⟨rfl, rfl⟩
      have hnd_tail : ts.Nodup := by
        simpa [List.Nodup] using hnd.tail
      have hind_tail : Independent eng ts := independent_tail hind
      have hpre : ∀ t' ∈ ts,
          eng.exec (applyCS snap (eng.exec snap t)) t' = eng.exec snap t' := by
        intro t' ht'
        exact exec_prefix_eq_snapshot (eng := eng) heng hnd hind
          [t] [eng.exec snap t] ts rfl rfl t' ht'
      have hmap : ts.map (eng.exec snap) =
          ts.map (eng.exec (applyCS snap (eng.exec snap t))) :=
        (map_exec_eq_of_pointwise (eng := eng) (fun t' ht' => (hpre t' ht').symm))
      rw [hmap]
      simpa [seqFold] using
        (ih (snap := applyCS snap (eng.exec snap t))
          (cs := ts.map (eng.exec (applyCS snap (eng.exec snap t))))
          hnd_tail hind_tail rfl)

/-- Extraction of the evidence carried by a successful speculative commit. -/
theorem engineWave_eq_some [DecidableEq K] [DecidableEq T] {eng : Engine K V T}
    {s : Store K V} {wave : List T} {cs : List (CS K V)}
    (h : engineWave eng s wave = some cs) :
    cs = wave.map (eng.exec s) ∧
      independentSteps (wave.map fun t => (eng.writes t, eng.reads t)) = true ∧
      eng.commit s cs = true := by
  classical
  let cs0 := wave.map (eng.exec s)
  let cond := independentSteps (wave.map fun t => (eng.writes t, eng.reads t)) &&
    eng.commit s cs0
  by_cases hf : cs0.any eng.failed
  · simp [engineWave, hf, cs0] at h
  · by_cases hcond : cond = true
    · have hced : cs0 = cs := by
        simpa [engineWave, hf, cs0, cond, hcond] using h
      have hcond' : independentSteps (wave.map fun t => (eng.writes t, eng.reads t)) = true ∧
          eng.commit s cs0 = true := by
        simpa [cond] using Bool.and_eq_true_iff.mp hcond
      exact ⟨by simpa [cs0] using hced.symm, hcond'.1,
        by rw [← hced]; exact hcond'.2⟩
    · simp [engineWave, hf, cs0, cond, hcond] at h

/-- Sequential fold over an append. -/
theorem seqFold_append (eng : Engine K V T) (s : Store K V) (l₁ l₂ : List T) :
    seqFold eng s (l₁ ++ l₂) = seqFold eng (seqFold eng s l₁) l₂ := by
  unfold seqFold
  rw [List.foldl_append]

/-- **Main theorem.** Under `EngineOK`, the modelled wave engine equals the
serial fold over the same transaction order for any wave partition with
distinct entries. With `SchedulerProof.schedule_flatten`, the transaction
order can be taken to be the scheduler's input. -/
theorem engineFold_eq_seqFold [DecidableEq K] [DecidableEq T] {eng : Engine K V T}
    (heng : EngineOK eng) (waves : List (List T)) (s : Store K V)
    (hnd : ∀ w ∈ waves, w.Nodup) (txs : List T)
    (hflat : waves.flatten = txs) :
    engineFold eng waves s = seqFold eng s txs := by
  induction waves generalizing txs s with
  | nil =>
      simp at hflat
      subst txs
      simp [engineFold, seqFold]
  | cons wave waves ih =>
      simp at hflat
      rw [← hflat]
      rw [seqFold_append]
      unfold engineFold
      simp only [List.foldl_cons]
      let cs0 := wave.map (eng.exec s)
      let cond := independentSteps (wave.map fun t => (eng.writes t, eng.reads t)) &&
        eng.commit s cs0
      have hnd_tail : ∀ w ∈ waves, w.Nodup := by
        intro w hw
        exact hnd w (List.mem_cons_of_mem wave hw)
      by_cases hf : cs0.any eng.failed
      · have hew : engineWave eng s wave = none := by
          simp [engineWave, hf, cs0]
        simp [hew]
        exact ih (seqFold eng s wave) hnd_tail waves.flatten rfl
      · by_cases hcond : cond = true
        · have hew : engineWave eng s wave = some cs0 := by
            simp [engineWave, hf, hcond, cs0, cond]
          simp [hew]
          have hinfo := engineWave_eq_some (eng := eng) hew
          have hind : Independent eng wave := by
            classical
            intro t₁ ht₁ t₂ ht₂ hne
            exact independentSteps_sound (eng := eng) ht₁ hinfo.2.1 ht₂ hne
          have hwave : cs0.foldl applyCS s = seqFold eng s wave := by
            simpa [cs0] using
              (wave_commit_eq_seq (eng := eng) heng (hnd wave (by simp)) hind
                (by simp))
          rw [hwave]
          exact ih (seqFold eng s wave) hnd_tail waves.flatten rfl
        · have hew : engineWave eng s wave = none := by
            simp [engineWave, hf, hcond, cs0, cond]
          simp [hew]
          exact ih (seqFold eng s wave) hnd_tail waves.flatten rfl

end EngineLoop
end KanariMoveRuntime
