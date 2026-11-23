-- Kanari Core: Lean 4 model of the blockchain engine.
--
-- This file contains definitions only. Every algebraic fact that used to be
-- asserted as an axiom is now proved in `KanariCore.Safety`.

namespace KanariCore

abbrev Address := Nat
abbrev Balance := Nat
abbrev TxHash := Nat

-- Object versioning
structure ObjectRef where
  id : Nat
  version : Nat
deriving DecidableEq, Repr

-- Transaction
inductive TxKind where
  | Transfer (from_ to : Address) (amount : Balance)
  | Mint (to : Address) (amount : Balance)
  | Burn (from_ : Address) (amount : Balance)
deriving Repr

structure Transaction where
  hash : TxHash
  kind : TxKind
deriving Repr

-- Account state
structure Account where
  balance : Balance
deriving Repr

-- Blockchain state
structure State where
  accounts : Address → Account
  totalSupply : Balance
  objectVersions : Nat → Nat  -- object id → version
  checkpointSeq : Nat

def emptyState : State := {
  accounts := fun _ => { balance := 0 }
  totalSupply := 0
  objectVersions := fun _ => 0
  checkpointSeq := 0
}

/-- Sum of the balances of the listed accounts. The list is a finite view of
the state; `conservation` is meaningful when it contains every account whose
balance can change, without duplicates. -/
def balanceSum (st : State) (addrs : List Address) : Nat :=
  addrs.foldl (fun acc a => acc + (st.accounts a).balance) 0

/-- Conservation: the finite account sum equals the authoritative total. -/
def conservation (st : State) (addrs : List Address) : Prop :=
  balanceSum st addrs = st.totalSupply

/-- Execute transfer. A self-transfer is a no-op; this closes the previously
documented bug where `from_ = to` debited the account without crediting it. -/
def execTransfer (st : State) (from_ to : Address) (amount : Balance) : State :=
  if from_ = to then st
  else if (st.accounts from_).balance >= amount then
    { st with accounts := fun a =>
        if a = from_ then { balance := (st.accounts from_).balance - amount }
        else if a = to then { balance := (st.accounts to).balance + amount }
        else st.accounts a }
  else st

-- Execute mint
def execMint (st : State) (to : Address) (amount : Balance) : State :=
  { st with
    accounts := fun a => if a = to then { balance := (st.accounts to).balance + amount } else st.accounts a
    totalSupply := st.totalSupply + amount }

-- Execute burn
def execBurn (st : State) (from_ : Address) (amount : Balance) : State :=
  if (st.accounts from_).balance >= amount then
    { st with
      accounts := fun a => if a = from_ then { balance := (st.accounts from_).balance - amount } else st.accounts a
      totalSupply := st.totalSupply - amount }
  else st

-- Execute transaction
def execTx (st : State) (tx : Transaction) : State :=
  match tx.kind with
  | TxKind.Transfer f t a => execTransfer st f t a
  | TxKind.Mint t a => execMint st t a
  | TxKind.Burn f a => execBurn st f a

/-- Apply a block of transactions in order. -/
def applyTxs : State → List Transaction → State
  | st, [] => st
  | st, tx :: txs => applyTxs (execTx st tx) txs

-- Checkpoint: monotonic sequence
def makeCheckpoint (st : State) (_txs : List Transaction) : State :=
  { st with checkpointSeq := st.checkpointSeq + 1 }

/-- Write an object version. Added so version monotonicity is a theorem about
an actual transition instead of an axiom about an arbitrary number. -/
def setObjectVersion (st : State) (oid : Nat) (v : Nat) : State :=
  { st with objectVersions := fun o => if o = oid then v else st.objectVersions o }

/-- A transaction is admissible when it cannot spend more balance than exists.
Mint is always admissible in this abstract model. -/
def TxAdmissible (st : State) : Transaction → Prop
  | ⟨_, .Transfer f t amount⟩ => f = t ∨ (st.accounts f).balance >= amount
  | ⟨_, .Mint _ _⟩ => True
  | ⟨_, .Burn f amount⟩ => (st.accounts f).balance >= amount

/-- A state invariant used by the safety proofs: the listed addresses form a
complete, duplicate-free finite view whose sum equals `totalSupply`. -/
structure Inv (st : State) (addrs : List Address) : Prop where
  nodup : addrs.Nodup
  cons : conservation st addrs

end KanariCore
