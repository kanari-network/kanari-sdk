-- Kanari Core: Lean 4 model of DAG vertex validation.

import KanariCore.Model

namespace KanariCore.Dag

structure DagVertex where
  round : Nat
  author : Nat
  chainId : String
  parents : List (Nat × Nat)  -- (author, round)
  txCount : Nat
  timestamp : Nat
  deriving Repr

def DAG_CHAIN_ID : String := "kanari-v2-mysticeti"

def ValidChainId (v : DagVertex) : Prop := v.chainId = DAG_CHAIN_ID
def ValidRound (v : DagVertex) : Prop := v.round > 0
def KnownAuthor (authorities : List Nat) (v : DagVertex) : Prop := v.author ∈ authorities

def ValidVertex (authorities : List Nat) (v : DagVertex) : Prop :=
  ValidChainId v ∧ ValidRound v ∧ KnownAuthor authorities v

theorem validVertex_chain (authorities : List Nat) (v : DagVertex)
    (h : ValidVertex authorities v) : v.chainId = DAG_CHAIN_ID := h.1

theorem validVertex_round (authorities : List Nat) (v : DagVertex)
    (h : ValidVertex authorities v) : v.round > 0 := h.2.1

theorem validVertex_author (authorities : List Nat) (v : DagVertex)
    (h : ValidVertex authorities v) : v.author ∈ authorities := h.2.2

-- A vertex with round 0 is never valid.
theorem round_zero_invalid (authorities : List Nat) (v : DagVertex) (hr : v.round = 0) :
    ¬ ValidVertex authorities v := by
  intro h
  have hrnd : v.round > 0 := h.2.1
  rw [hr] at hrnd
  exact Nat.lt_irrefl 0 hrnd

-- Unknown author is never valid.
theorem unknown_author_invalid (authorities : List Nat) (v : DagVertex) (h : v.author ∉ authorities) :
    ¬ ValidVertex authorities v := by
  intro hv
  exact h hv.2.2

end KanariCore.Dag