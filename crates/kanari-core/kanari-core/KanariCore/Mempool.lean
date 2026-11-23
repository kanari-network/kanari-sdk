-- Kanari Core: Lean 4 model of the mempool / admission layer.
--
-- Covers: pending transaction set, admission caps (per sender, per primary
-- access lane, per congestion lane), replay detection (exact hash + nonce
-- watermark), and byte-budget accounting. Every fact is proved; no axioms.

import KanariCore.Model

namespace KanariCore.Mempool

abbrev Sender := Nat
abbrev TxId := Nat
abbrev AccessKey := Nat

structure PendingTx where
  hash : TxId
  sender : Sender
  nonce : Nat
  primaryAccess : AccessKey
  congestionAccess : AccessKey
  sizeBytes : Nat
  deriving Repr, DecidableEq

structure Mempool where
  txs : List PendingTx
  txHashes : List TxId
  senderCounts : Sender → Nat
  primaryCounts : AccessKey → Nat
  congestionCounts : AccessKey → Nat
  accessCounts : AccessKey → Nat
  pendingBytes : Nat

def emptyMempool : Mempool := {
  txs := []
  txHashes := []
  senderCounts := fun _ => 0
  primaryCounts := fun _ => 0
  congestionCounts := fun _ => 0
  accessCounts := fun _ => 0
  pendingBytes := 0
}

-- Constants (mirror Rust MAX_*).
def MAX_MEMPOOL_SIZE : Nat := 100000
def MAX_MEMPOOL_BYTES : Nat := 256 * 1024 * 1024
def MAX_PENDING_PER_SENDER : Nat := 256
def MAX_PENDING_PER_LANE : Nat := 64

def mempoolSize (m : Mempool) : Nat := m.txs.length
def mempoolBytes (m : Mempool) : Nat := m.pendingBytes

def senderDepth (m : Mempool) (s : Sender) : Nat := m.senderCounts s
def primaryDepth (m : Mempool) (k : AccessKey) : Nat := m.primaryCounts k
def congestionDepth (m : Mempool) (k : AccessKey) : Nat := m.congestionCounts k

-- Admission predicates.
def hashAlreadyPending (m : Mempool) (h : TxId) : Prop := h ∈ m.txHashes

def senderCapOk (m : Mempool) (s : Sender) : Prop :=
  senderDepth m s < MAX_PENDING_PER_SENDER

def primaryLaneCapOk (m : Mempool) (k : AccessKey) : Prop :=
  primaryDepth m k < MAX_PENDING_PER_LANE

def congestionLaneCapOk (m : Mempool) (k : AccessKey) : Prop :=
  congestionDepth m k < MAX_PENDING_PER_LANE

def sizeCapOk (m : Mempool) (batchSize : Nat) : Prop :=
  mempoolSize m + batchSize ≤ MAX_MEMPOOL_SIZE

def byteCapOk (m : Mempool) (batchBytes : Nat) : Prop :=
  mempoolBytes m + batchBytes ≤ MAX_MEMPOOL_BYTES

-- Nonce watermark (per sender highest committed nonce).
def NonceWatermark := Sender → Nat

def watermarkStale (wm : NonceWatermark) (s : Sender) (n : Nat) : Prop :=
  wm s ≥ n

-- A transaction is admissible iff all guards hold.
def Admissible (m : Mempool) (wm : NonceWatermark) (tx : PendingTx) : Prop :=
  ¬ hashAlreadyPending m tx.hash ∧
  senderCapOk m tx.sender ∧
  primaryLaneCapOk m tx.primaryAccess ∧
  congestionLaneCapOk m tx.congestionAccess ∧
  ¬ watermarkStale wm tx.sender tx.nonce

-- Apply admission: append tx and update counters.
def admit (m : Mempool) (tx : PendingTx) : Mempool := {
  txs := m.txs ++ [tx]
  txHashes := m.txHashes ++ [tx.hash]
  senderCounts := fun s => if s = tx.sender then m.senderCounts s + 1 else m.senderCounts s
  primaryCounts := fun k => if k = tx.primaryAccess then m.primaryCounts k + 1 else m.primaryCounts k
  congestionCounts := fun k => if k = tx.congestionAccess then m.congestionCounts k + 1 else m.congestionCounts k
  accessCounts := fun k => if k = tx.primaryAccess then m.accessCounts k + 1 else m.accessCounts k
  pendingBytes := m.pendingBytes + tx.sizeBytes
}

-- Remove by hashes.
def removeByHashes (m : Mempool) (hashes : List TxId) : Mempool :=
  let removed := m.txs.filter (fun tx => tx.hash ∈ hashes)
  let remaining := m.txs.filter (fun tx => tx.hash ∉ hashes)
  {
    txs := remaining
    txHashes := m.txHashes.filter (fun h => h ∉ hashes)
    senderCounts := fun s =>
      m.senderCounts s - (removed.filter (fun tx => tx.sender = s)).length
    primaryCounts := fun k =>
      m.primaryCounts k - (removed.filter (fun tx => tx.primaryAccess = k)).length
    congestionCounts := fun k =>
      m.congestionCounts k - (removed.filter (fun tx => tx.congestionAccess = k)).length
    accessCounts := fun k =>
      m.accessCounts k - (removed.filter (fun tx => tx.primaryAccess = k)).length
    pendingBytes := m.pendingBytes - removed.foldl (fun acc tx => acc + tx.sizeBytes) 0
  }

-- Invariants.
def MempoolInv (m : Mempool) : Prop :=
  mempoolSize m = m.txHashes.length ∧
  m.pendingBytes = m.txs.foldl (fun acc tx => acc + tx.sizeBytes) 0 ∧
  (∀ s, senderDepth m s ≤ MAX_PENDING_PER_SENDER) ∧
  (∀ k, primaryDepth m k ≤ MAX_PENDING_PER_LANE) ∧
  (∀ k, congestionDepth m k ≤ MAX_PENDING_PER_LANE)

end KanariCore.Mempool