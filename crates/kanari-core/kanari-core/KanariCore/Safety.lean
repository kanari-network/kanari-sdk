-- Machine-checked safety facts for the Kanari Core model.
-- No axioms and no `sorry`. Theorems here justify the facts that used to be
-- axioms in `Model.lean` and add conservation/object-version results.

import KanariCore.Model

namespace KanariCore

/-! ## Checkpoint sequence -/

theorem makeCheckpoint_seq (st : State) (txs : List Transaction) :
    (makeCheckpoint st txs).checkpointSeq = st.checkpointSeq + 1 := rfl

theorem makeCheckpoint_increases (st : State) (txs : List Transaction) :
    (makeCheckpoint st txs).checkpointSeq > st.checkpointSeq := by
  simp [makeCheckpoint]

theorem makeCheckpoint_preserves_accounts (st : State) (txs : List Transaction) (a : Address) :
    (makeCheckpoint st txs).accounts a = st.accounts a := rfl

theorem makeCheckpoint_preserves_total (st : State) (txs : List Transaction) :
    (makeCheckpoint st txs).totalSupply = st.totalSupply := rfl

theorem makeCheckpoint_preserves_objects (st : State) (txs : List Transaction) (oid : Nat) :
    (makeCheckpoint st txs).objectVersions oid = st.objectVersions oid := rfl

/-! ## Transfer -/

theorem execTransfer_self (st : State) (a : Address) (amount : Balance) :
    execTransfer st a a amount = st := by
  simp [execTransfer]

theorem execTransfer_total (st : State) (f t : Address) (amount : Balance) :
    (execTransfer st f t amount).totalSupply = st.totalSupply := by
  by_cases hft : f = t
  · simp [execTransfer, hft]
  · by_cases hbal : (st.accounts f).balance >= amount <;> simp [execTransfer, hft, hbal]

theorem execTransfer_noop_of_insufficient (st : State) (f t : Address) (amount : Balance)
    (h : (st.accounts f).balance < amount) : execTransfer st f t amount = st := by
  have hbal : ¬ (st.accounts f).balance >= amount := Nat.not_le_of_gt h
  by_cases hft : f = t
  · simp [execTransfer, hft]
  · simp [execTransfer, hft, hbal]

theorem execTransfer_from_balance (st : State) (f t : Address) (amount : Balance)
    (hft : f ≠ t) (hbal : (st.accounts f).balance >= amount) :
    (execTransfer st f t amount).accounts f = { balance := (st.accounts f).balance - amount } := by
  simp [execTransfer, hft, hbal]

theorem execTransfer_to_balance (st : State) (f t : Address) (amount : Balance)
    (hft : f ≠ t) (hbal : (st.accounts f).balance >= amount) :
    (execTransfer st f t amount).accounts t = { balance := (st.accounts t).balance + amount } := by
  have htf : t ≠ f := Ne.symm hft
  simp [execTransfer, hft, hbal, htf]

theorem execTransfer_other_balance (st : State) (f t a : Address) (amount : Balance)
    (haf : a ≠ f) (hat : a ≠ t) :
    (execTransfer st f t amount).accounts a = st.accounts a := by
  by_cases hft : f = t
  · simp [execTransfer, hft]
  · by_cases hbal : (st.accounts f).balance >= amount <;> simp [execTransfer, hft, hbal, haf, hat]

theorem execTransfer_preserves_checkpoint (st : State) (f t : Address) (amount : Balance) :
    (execTransfer st f t amount).checkpointSeq = st.checkpointSeq := by
  by_cases hft : f = t
  · simp [execTransfer, hft]
  · by_cases hbal : (st.accounts f).balance >= amount <;> simp [execTransfer, hft, hbal]

theorem execTransfer_preserves_objects (st : State) (f t : Address) (amount : Balance) (oid : Nat) :
    (execTransfer st f t amount).objectVersions oid = st.objectVersions oid := by
  by_cases hft : f = t
  · simp [execTransfer, hft]
  · by_cases hbal : (st.accounts f).balance >= amount <;> simp [execTransfer, hft, hbal]

/-! ## Mint and burn -/

theorem execMint_total (st : State) (to : Address) (amount : Balance) :
    (execMint st to amount).totalSupply = st.totalSupply + amount := rfl

theorem execMint_to_balance (st : State) (to : Address) (amount : Balance) :
    (execMint st to amount).accounts to = { balance := (st.accounts to).balance + amount } := by
  simp [execMint]

theorem execMint_other_balance (st : State) (to a : Address) (amount : Balance) (hat : a ≠ to) :
    (execMint st to amount).accounts a = st.accounts a := by
  simp [execMint, hat]

theorem execMint_preserves_checkpoint (st : State) (to : Address) (amount : Balance) :
    (execMint st to amount).checkpointSeq = st.checkpointSeq := rfl

theorem execMint_preserves_objects (st : State) (to : Address) (amount : Balance) (oid : Nat) :
    (execMint st to amount).objectVersions oid = st.objectVersions oid := rfl

theorem execBurn_total (st : State) (f : Address) (amount : Balance)
    (hbal : (st.accounts f).balance >= amount) :
    (execBurn st f amount).totalSupply = st.totalSupply - amount := by
  simp [execBurn, hbal]

theorem execBurn_noop_of_insufficient (st : State) (f : Address) (amount : Balance)
    (h : (st.accounts f).balance < amount) : execBurn st f amount = st := by
  have hbal : ¬ (st.accounts f).balance >= amount := Nat.not_le_of_gt h
  simp [execBurn, hbal]

theorem execBurn_from_balance (st : State) (f : Address) (amount : Balance)
    (hbal : (st.accounts f).balance >= amount) :
    (execBurn st f amount).accounts f = { balance := (st.accounts f).balance - amount } := by
  simp [execBurn, hbal]

theorem execBurn_other_balance (st : State) (f a : Address) (amount : Balance) (haf : a ≠ f) :
    (execBurn st f amount).accounts a = st.accounts a := by
  by_cases hbal : (st.accounts f).balance >= amount
  · simp [execBurn, hbal, haf]
  · simp [execBurn, hbal]

theorem execBurn_preserves_checkpoint (st : State) (f : Address) (amount : Balance) :
    (execBurn st f amount).checkpointSeq = st.checkpointSeq := by
  by_cases hbal : (st.accounts f).balance >= amount <;> simp [execBurn, hbal]

theorem execBurn_preserves_objects (st : State) (f : Address) (amount : Balance) (oid : Nat) :
    (execBurn st f amount).objectVersions oid = st.objectVersions oid := by
  by_cases hbal : (st.accounts f).balance >= amount <;> simp [execBurn, hbal]

/-! ## Transaction execution -/

theorem execTx_preserves_checkpoint (st : State) (tx : Transaction) :
    (execTx st tx).checkpointSeq = st.checkpointSeq := by
  cases tx with
  | mk h k =>
      cases k <;> simp [execTx, execTransfer_preserves_checkpoint, execMint_preserves_checkpoint,
        execBurn_preserves_checkpoint]

theorem execTx_preserves_objects (st : State) (tx : Transaction) (oid : Nat) :
    (execTx st tx).objectVersions oid = st.objectVersions oid := by
  cases tx with
  | mk h k =>
      cases k <;> simp [execTx, execTransfer_preserves_objects, execMint_preserves_objects,
        execBurn_preserves_objects]

theorem execTx_transfer_self (st : State) (h : TxHash) (a : Address) (amount : Balance) :
    execTx st ⟨h, .Transfer a a amount⟩ = st := by
  simp [execTx, execTransfer_self]

/-! ## Object versions -/

theorem setObjectVersion_eq (st : State) (oid v : Nat) :
    (setObjectVersion st oid v).objectVersions oid = v := by
  simp [setObjectVersion]

theorem setObjectVersion_of_ne (st : State) (oid o v : Nat) (h : o ≠ oid) :
    (setObjectVersion st oid v).objectVersions o = st.objectVersions o := by
  simp [setObjectVersion, h]

theorem object_version_positive (st : State) (oid newVer : Nat)
    (h : newVer > st.objectVersions oid) : newVer > 0 :=
  Nat.lt_of_le_of_lt (Nat.zero_le (st.objectVersions oid)) h

theorem setObjectVersion_strictly_increases (st : State) (oid v : Nat)
    (h : v > st.objectVersions oid) :
    (setObjectVersion st oid v).objectVersions oid > st.objectVersions oid := by
  simpa [setObjectVersion] using h

theorem setObjectVersion_preserves_accounts (st : State) (oid v : Nat) (a : Address) :
    (setObjectVersion st oid v).accounts a = st.accounts a := rfl

theorem setObjectVersion_preserves_total (st : State) (oid v : Nat) :
    (setObjectVersion st oid v).totalSupply = st.totalSupply := rfl

theorem setObjectVersion_preserves_checkpoint (st : State) (oid v : Nat) :
    (setObjectVersion st oid v).checkpointSeq = st.checkpointSeq := rfl

/-! ## Finite conservation -/

theorem foldl_add_balance (addrs : List Address) (f : Address → Nat) (acc : Nat) :
    addrs.foldl (fun a b => a + f b) acc = acc + addrs.foldl (fun a b => a + f b) 0 := by
  induction addrs generalizing acc with
  | nil => simp
  | cons a as ih =>
      change as.foldl (fun x y => x + f y) (acc + f a) =
        acc + as.foldl (fun x y => x + f y) (0 + f a)
      rw [ih (acc + f a), ih (0 + f a)]
      simp [Nat.add_assoc]

theorem balanceSum_cons (st : State) (a : Address) (addrs : List Address) :
    balanceSum st (a :: addrs) = (st.accounts a).balance + balanceSum st addrs := by
  unfold balanceSum
  change addrs.foldl (fun acc x => acc + (st.accounts x).balance) (0 + (st.accounts a).balance) =
    (st.accounts a).balance + addrs.foldl (fun acc x => acc + (st.accounts x).balance) 0
  rw [foldl_add_balance]
  simp

theorem balanceSum_eq_of_unchanged {st st' : State} {addrs : List Address}
    (h : ∀ a ∈ addrs, (st'.accounts a).balance = (st.accounts a).balance) :
    balanceSum st' addrs = balanceSum st addrs := by
  induction addrs with
  | nil => rfl
  | cons a as ih =>
      have ha := h a (by simp)
      have has : ∀ x ∈ as, (st'.accounts x).balance = (st.accounts x).balance :=
        fun x hx => h x (by simp [hx])
      simp [balanceSum_cons, ha, ih has]

theorem conservation_of_unchanged {st st' : State} {addrs : List Address}
    (hsum : balanceSum st' addrs = balanceSum st addrs)
    (htotal : st'.totalSupply = st.totalSupply)
    (hcons : conservation st addrs) : conservation st' addrs := by
  unfold conservation at hcons ⊢
  rw [hsum, htotal, hcons]

theorem conservation_execTransfer_of_self (st : State) (a : Address) (amount : Balance)
    (hcons : conservation st addrs) : conservation (execTransfer st a a amount) addrs := by
  simpa [execTransfer_self] using hcons

/-- Transfer conservation when the two changed addresses are at the front of
the finite view and absent from the remainder. This is the scoped form of the
usual "complete, duplicate-free address list" side condition. -/
theorem conservation_transfer_decomp (st : State) (f t : Address) (amount : Balance)
    (rest : List Address) (hft : f ≠ t) (hbal : (st.accounts f).balance >= amount)
    (hfrest : f ∉ rest) (htrest : t ∉ rest)
    (hcons : conservation st (f :: t :: rest)) :
    conservation (execTransfer st f t amount) (f :: t :: rest) := by
  unfold conservation at hcons ⊢
  rw [execTransfer_total]
  rw [← hcons]
  rw [balanceSum_cons, balanceSum_cons, balanceSum_cons, balanceSum_cons]
  have hfBal : ((execTransfer st f t amount).accounts f).balance =
      (st.accounts f).balance - amount := by
    simpa using congrArg Account.balance (execTransfer_from_balance st f t amount hft hbal)
  have htBal : ((execTransfer st f t amount).accounts t).balance =
      (st.accounts t).balance + amount := by
    simpa using congrArg Account.balance (execTransfer_to_balance st f t amount hft hbal)
  have hrest : balanceSum (execTransfer st f t amount) rest = balanceSum st rest := by
    apply balanceSum_eq_of_unchanged
    intro a ha
    have haf : a ≠ f := by intro h; subst a; exact hfrest ha
    have hat : a ≠ t := by intro h; subst a; exact htrest ha
    simpa using congrArg Account.balance (execTransfer_other_balance st f t a amount haf hat)
  rw [hfBal, htBal, hrest]
  calc
    ((st.accounts f).balance - amount) + ((st.accounts t).balance + amount + balanceSum st rest)
        = ((st.accounts f).balance - amount + amount) +
            ((st.accounts t).balance + balanceSum st rest) := by ac_rfl
    _ = (st.accounts f).balance + ((st.accounts t).balance + balanceSum st rest) := by
      rw [Nat.sub_add_cancel hbal]

/-- Mint conservation under the same finite-view decomposition. -/
theorem conservation_mint_decomp (st : State) (to : Address) (amount : Balance)
    (rest : List Address) (htorest : to ∉ rest)
    (hcons : conservation st (to :: rest)) :
    conservation (execMint st to amount) (to :: rest) := by
  unfold conservation at hcons ⊢
  rw [execMint_total]
  rw [← hcons]
  rw [balanceSum_cons, balanceSum_cons]
  have htoBal : ((execMint st to amount).accounts to).balance =
      (st.accounts to).balance + amount := by
    simpa using congrArg Account.balance (execMint_to_balance st to amount)
  have hrest : balanceSum (execMint st to amount) rest = balanceSum st rest := by
    apply balanceSum_eq_of_unchanged
    intro a ha
    have hat : a ≠ to := by intro h; subst a; exact htorest ha
    simpa using congrArg Account.balance (execMint_other_balance st to a amount hat)
  rw [htoBal, hrest]
  ac_rfl

/-- Burn conservation under the same finite-view decomposition. The balance
cover hypothesis is enough to show the authoritative total also covers it. -/
theorem conservation_burn_decomp (st : State) (f : Address) (amount : Balance)
    (rest : List Address) (hfrest : f ∉ rest)
    (hbal : (st.accounts f).balance >= amount)
    (hcons : conservation st (f :: rest)) :
    conservation (execBurn st f amount) (f :: rest) := by
  unfold conservation at hcons ⊢
  rw [execBurn_total st f amount hbal]
  rw [← hcons]
  rw [balanceSum_cons, balanceSum_cons]
  have hfBal : ((execBurn st f amount).accounts f).balance =
      (st.accounts f).balance - amount := by
    simpa using congrArg Account.balance (execBurn_from_balance st f amount hbal)
  have hrest : balanceSum (execBurn st f amount) rest = balanceSum st rest := by
    apply balanceSum_eq_of_unchanged
    intro a ha
    have haf : a ≠ f := by intro h; subst a; exact hfrest ha
    simpa using congrArg Account.balance (execBurn_other_balance st f a amount haf)
  rw [hfBal, hrest]
  exact (Nat.sub_add_comm hbal).symm

/-! ## Admissibility and preservation -/

theorem TxAdmissible.transfer_self (st : State) (h : TxHash) (a : Address) (amount : Balance) :
    TxAdmissible st ⟨h, .Transfer a a amount⟩ := by
  simp [TxAdmissible]

theorem TxAdmissible.of_transfer (st : State) (h : TxHash) (f t : Address) (amount : Balance)
    (hadm : TxAdmissible st ⟨h, .Transfer f t amount⟩) :
    f = t ∨ (st.accounts f).balance >= amount := hadm

theorem execTransfer_preserves_balance_of_admissible (st : State) (h : TxHash)
    (f t : Address) (amount : Balance)
    (hadm : TxAdmissible st ⟨h, .Transfer f t amount⟩) :
    execTransfer st f t amount = st ∨ (st.accounts f).balance >= amount := by
  by_cases hft : f = t
  · subst hft
    exact Or.inl (execTransfer_self st f amount)
  · have hbal : (st.accounts f).balance >= amount := by
      simpa [TxAdmissible, hft] using hadm
    exact Or.inr hbal

theorem execBurn_noop_of_not_admissible (st : State) (h : TxHash) (f : Address) (amount : Balance)
    (hadm : ¬ TxAdmissible st ⟨h, .Burn f amount⟩) : execBurn st f amount = st := by
  have h : (st.accounts f).balance < amount := by
    simpa [TxAdmissible] using Nat.lt_of_not_ge hadm
  exact execBurn_noop_of_insufficient st f amount h

theorem execTx_total_of_admissible (st : State) (tx : Transaction)
    (hadm : TxAdmissible st tx) :
    (execTx st tx).totalSupply =
      match tx.kind with
      | .Transfer _ _ _ => st.totalSupply
      | .Mint _ a => st.totalSupply + a
      | .Burn _ a => st.totalSupply - a := by
  cases tx with
  | mk h k =>
      cases k with
      | Transfer f t a =>
          simp [execTx, execTransfer_total]
      | Mint to a =>
          simp [execTx, execMint_total]
      | Burn f a =>
          have hbal : (st.accounts f).balance >= a := by simpa [TxAdmissible] using hadm
          simp [execTx, execBurn_total st f a hbal]

/-! ## Canonical conservation for execTx -/

theorem conservation_execTx_transfer (st : State) (h : TxHash) (f t : Address) (amount : Balance)
    (rest : List Address) (hft : f ≠ t) (hbal : (st.accounts f).balance >= amount)
    (hfrest : f ∉ rest) (htrest : t ∉ rest)
    (hcons : conservation st (f :: t :: rest)) :
    conservation (execTx st ⟨h, .Transfer f t amount⟩) (f :: t :: rest) := by
  simpa [execTx] using conservation_transfer_decomp st f t amount rest hft hbal hfrest htrest hcons

theorem conservation_execTx_transfer_self (st : State) (h : TxHash) (a : Address)
    (amount : Balance) (addrs : List Address) (hcons : conservation st addrs) :
    conservation (execTx st ⟨h, .Transfer a a amount⟩) addrs := by
  simpa [execTx, execTransfer_self] using hcons

theorem conservation_execTx_mint (st : State) (h : TxHash) (to : Address) (amount : Balance)
    (rest : List Address) (htorest : to ∉ rest)
    (hcons : conservation st (to :: rest)) :
    conservation (execTx st ⟨h, .Mint to amount⟩) (to :: rest) := by
  simpa [execTx] using conservation_mint_decomp st to amount rest htorest hcons

theorem conservation_execTx_burn (st : State) (h : TxHash) (f : Address) (amount : Balance)
    (rest : List Address) (hfrest : f ∉ rest)
    (hbal : (st.accounts f).balance >= amount)
    (hcons : conservation st (f :: rest)) :
    conservation (execTx st ⟨h, .Burn f amount⟩) (f :: rest) := by
  simpa [execTx] using conservation_burn_decomp st f amount rest hfrest hbal hcons

/-! ## applyTxs -/

theorem applyTxs_nil (st : State) : applyTxs st [] = st := rfl

theorem applyTxs_cons (st : State) (tx : Transaction) (txs : List Transaction) :
    applyTxs st (tx :: txs) = applyTxs (execTx st tx) txs := rfl

theorem applyTxs_append (st : State) (xs ys : List Transaction) :
    applyTxs st (xs ++ ys) = applyTxs (applyTxs st xs) ys := by
  induction xs generalizing st with
  | nil => simp [applyTxs]
  | cons x xs ih => simp [applyTxs, ih]

theorem applyTxs_preserves_checkpoint (st : State) (txs : List Transaction) :
    (applyTxs st txs).checkpointSeq = st.checkpointSeq := by
  induction txs generalizing st with
  | nil => simp [applyTxs]
  | cons tx txs ih => simp [applyTxs, ih, execTx_preserves_checkpoint]

theorem applyTxs_preserves_objects (st : State) (txs : List Transaction) (oid : Nat) :
    (applyTxs st txs).objectVersions oid = st.objectVersions oid := by
  induction txs generalizing st with
  | nil => simp [applyTxs]
  | cons tx txs ih => simp [applyTxs, ih, execTx_preserves_objects]

/-! ## Empty state -/

theorem emptyState_conservation (addrs : List Address) : conservation emptyState addrs := by
  unfold conservation balanceSum
  induction addrs with
  | nil => rfl
  | cons a as ih =>
      simp [List.foldl, emptyState]
      exact ih

theorem emptyState_inv (addrs : List Address) (hnd : addrs.Nodup) : Inv emptyState addrs :=
  ⟨hnd, emptyState_conservation addrs⟩

/-! ## Conservation over applyTxs for canonical txs -/

def CanonicalTxCovers (addrs : List Address) : Transaction → Prop
  | ⟨_, .Transfer f t _⟩ =>
      f = t ∧ f ∈ addrs ∨
      ∃ rest, addrs = f :: t :: rest ∧ f ∉ rest ∧ t ∉ rest
  | ⟨_, .Mint to _⟩ => ∃ rest, addrs = to :: rest ∧ to ∉ rest
  | ⟨_, .Burn f _⟩ => ∃ rest, addrs = f :: rest ∧ f ∉ rest

theorem conservation_execTx_of_covers (st : State) (tx : Transaction) (addrs : List Address)
    (hadm : TxAdmissible st tx)
    (hcover : CanonicalTxCovers addrs tx)
    (hcons : conservation st addrs) :
    conservation (execTx st tx) addrs := by
  cases tx with
  | mk h k =>
      cases k with
      | Transfer f t amount =>
          rcases hcover with ⟨hft, _⟩ | ⟨rest, haddrs, hfrest, htrest⟩
          · subst hft
            simpa [execTx, execTransfer_self] using hcons
          · subst haddrs
            by_cases hft : f = t
            · subst hft
              simpa [execTx, execTransfer_self] using hcons
            · have hbal : (st.accounts f).balance >= amount := by
                have hadm' : f = t ∨ (st.accounts f).balance >= amount := by
                  simpa [TxAdmissible] using hadm
                rcases hadm' with hft' | hbal
                · exact False.elim (hft hft')
                · exact hbal
              exact conservation_execTx_transfer st h f t amount rest hft hbal hfrest htrest hcons
      | Mint to amount =>
          rcases hcover with ⟨rest, haddrs, htorest⟩
          subst haddrs
          exact conservation_execTx_mint st h to amount rest htorest hcons
      | Burn f amount =>
          rcases hcover with ⟨rest, haddrs, hfrest⟩
          subst haddrs
          have hbal : (st.accounts f).balance >= amount := by
            simpa [TxAdmissible] using hadm
          exact conservation_execTx_burn st h f amount rest hfrest hbal hcons

/-- Generic block conservation over a precondition that is preserved by each
step. `P st tx` should capture both admissibility/coverage and any facts needed
to show it still holds after `execTx st tx`. -/
theorem conservation_applyTxs_of_inv
    (addrs : List Address)
    (P : State → Transaction → Prop)
    (step : ∀ st tx, conservation st addrs → P st tx → conservation (execTx st tx) addrs)
    (pres : ∀ st tx tx', P st tx → P st tx' → P (execTx st tx) tx')
    : ∀ (st : State) (txs : List Transaction),
      conservation st addrs →
      (∀ tx ∈ txs, P st tx) →
      conservation (applyTxs st txs) addrs := by
  intro st txs hcons hall
  induction txs generalizing st with
  | nil => simpa [applyTxs] using hcons
  | cons tx txs ih =>
      have hall' : P st tx ∧ ∀ tx' ∈ txs, P st tx' := by
        simpa [List.mem_cons] using hall
      rcases hall' with ⟨htx, hrest⟩
      have hcons' := step st tx hcons htx
      have hrest' : ∀ tx' ∈ txs, P (execTx st tx) tx' :=
        fun tx' hmem => pres st tx tx' htx (hrest tx' hmem)
      exact ih (execTx st tx) hcons' hrest'

