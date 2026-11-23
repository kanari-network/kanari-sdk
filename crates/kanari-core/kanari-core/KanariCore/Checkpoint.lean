-- Kanari Core: Lean 4 model of checkpoint validation and recovery.

import KanariCore.Model

namespace KanariCore.Checkpoint

structure Checkpoint where
  sequence : Nat
  transactions : List Transaction
  stateRoot : Nat
  prevHash : Nat
  deriving Repr

def genesisCheckpoint : Checkpoint := {
  sequence := 0
  transactions := []
  stateRoot := 0
  prevHash := 0
}

def validateSequence (chain : List Checkpoint) (cp : Checkpoint) : Prop :=
  match chain.getLast? with
  | some last => cp.sequence = last.sequence + 1
  | none => cp.sequence = 0

def validatePrevHash (chain : List Checkpoint) (cp : Checkpoint) : Prop :=
  match chain.getLast? with
  | some last => cp.prevHash = last.stateRoot  -- simplified: prev hash = prior root
  | none => True

def noDuplicateWithin (cp : Checkpoint) : Prop :=
  cp.transactions.map (·.hash) = (cp.transactions.map (·.hash)).eraseDups

def noReplay (chain : List Checkpoint) (cp : Checkpoint) : Prop :=
  ∀ tx ∈ cp.transactions, tx.hash ∉ chain.flatMap (·.transactions.map (·.hash))

def CheckpointValid (chain : List Checkpoint) (cp : Checkpoint) : Prop :=
  validateSequence chain cp ∧ validatePrevHash chain cp ∧ noDuplicateWithin cp ∧ noReplay chain cp

-- Genesis is always at sequence 0.
theorem genesis_sequence : genesisCheckpoint.sequence = 0 := rfl

-- Empty chain accepts genesis.
theorem genesis_valid_on_empty : CheckpointValid [] genesisCheckpoint := by
  unfold CheckpointValid validateSequence validatePrevHash noDuplicateWithin noReplay
  simp [genesisCheckpoint]

end KanariCore.Checkpoint