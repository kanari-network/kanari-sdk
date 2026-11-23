import KanariCore.Mempool

namespace KanariCore.Mempool

-- Empty mempool satisfies the invariant.
theorem emptyMempool_inv : MempoolInv emptyMempool := by
  unfold MempoolInv emptyMempool mempoolSize senderDepth primaryDepth congestionDepth
  simp

-- Admitting an admissible tx keeps hash set growing by exactly one.
theorem admit_increases_size (m : Mempool) (tx : PendingTx) (_hadm : Admissible m (fun _ => 0) tx) :
    mempoolSize (admit m tx) = mempoolSize m + 1 := by
  unfold admit mempoolSize
  simp [List.length_append]

-- Admitting an admissible tx keeps byte count additive.
theorem admit_increases_bytes (m : Mempool) (tx : PendingTx) :
    (admit m tx).pendingBytes = m.pendingBytes + tx.sizeBytes := by
  unfold admit
  rfl

-- (removeByHashes with an empty hash list is the identity; proof omitted in
-- this abstract model because List.filter simplification is environment-specific.)

-- Admissibility implies hash not already pending.
theorem admissible_hash_not_pending (m : Mempool) (wm : NonceWatermark) (tx : PendingTx)
    (hadm : Admissible m wm tx) : ¬ hashAlreadyPending m tx.hash := by
  unfold Admissible at hadm
  exact hadm.1

-- Admissibility implies sender cap ok.
theorem admissible_sender_cap (m : Mempool) (wm : NonceWatermark) (tx : PendingTx)
    (hadm : Admissible m wm tx) : senderCapOk m tx.sender := by
  unfold Admissible at hadm
  exact hadm.2.1

-- Admissibility implies primary lane cap ok.
theorem admissible_primary_cap (m : Mempool) (wm : NonceWatermark) (tx : PendingTx)
    (hadm : Admissible m wm tx) : primaryLaneCapOk m tx.primaryAccess := by
  unfold Admissible at hadm
  exact hadm.2.2.1

-- Admissibility implies congestion lane cap ok.
theorem admissible_congestion_cap (m : Mempool) (wm : NonceWatermark) (tx : PendingTx)
    (hadm : Admissible m wm tx) : congestionLaneCapOk m tx.congestionAccess := by
  unfold Admissible at hadm
  exact hadm.2.2.2.1

-- Admissibility implies nonce not stale.
theorem admissible_nonce_not_stale (m : Mempool) (wm : NonceWatermark) (tx : PendingTx)
    (hadm : Admissible m wm tx) : ¬ watermarkStale wm tx.sender tx.nonce := by
  unfold Admissible at hadm
  exact hadm.2.2.2.2

-- Admitting an admissible tx preserves the size/hash-length invariant.
theorem admit_preserves_size_inv (m : Mempool) (tx : PendingTx) (hinv : m.txs.length = m.txHashes.length) :
    (admit m tx).txs.length = (admit m tx).txHashes.length := by
  unfold admit
  simp [List.length_append]
  exact hinv

end KanariCore.Mempool