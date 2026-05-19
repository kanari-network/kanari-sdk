// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl DagStore {
    fn same_checkpoint_payload_except_state_root(
        existing: &Checkpoint,
        candidate: &Checkpoint,
    ) -> bool {
        let existing_tx_hashes: Vec<_> = existing.transactions.iter().map(|tx| tx.hash()).collect();
        let candidate_tx_hashes: Vec<_> =
            candidate.transactions.iter().map(|tx| tx.hash()).collect();

        existing.sequence == candidate.sequence
            && existing.vertices == candidate.vertices
            && existing_tx_hashes == candidate_tx_hashes
            && existing.timestamp == candidate.timestamp
            && existing.prev_checkpoint_hash == candidate.prev_checkpoint_hash
    }

    fn validate_checkpoint_payload(&self, checkpoint: &Checkpoint) -> Result<()> {
        let mut seen_vertices = HashSet::new();
        let mut seen_tx_hashes = HashSet::new();
        let mut expected_tx_hashes = Vec::new();
        for vertex_id in &checkpoint.vertices {
            if !seen_vertices.insert(*vertex_id) {
                anyhow::bail!(
                    "Checkpoint contains duplicate vertex {}",
                    hex::encode(vertex_id)
                );
            }

            let vertex = self.vertices.get(vertex_id).ok_or_else(|| {
                anyhow::anyhow!(
                    "Checkpoint references missing vertex {}",
                    hex::encode(vertex_id)
                )
            })?;

            for tx in &vertex.transactions {
                let tx_hash = tx.hash();
                if self.executed_tx_hashes.contains(&tx_hash) {
                    continue;
                }
                if seen_tx_hashes.insert(tx_hash.clone()) {
                    expected_tx_hashes.push(tx_hash);
                }
            }
        }

        let actual_tx_hashes: Vec<Vec<u8>> =
            checkpoint.transactions.iter().map(|tx| tx.hash()).collect();
        if actual_tx_hashes != expected_tx_hashes {
            anyhow::bail!("Checkpoint transactions do not match referenced DAG vertices");
        }

        Ok(())
    }

    pub fn should_create_checkpoint(&self) -> bool {
        let rounds_since_last = self
            .current_round
            .saturating_sub(self.last_checkpoint_round);
        let pending_count = self.pending_vertices.len();

        if rounds_since_last >= self.checkpoint_config.max_rounds {
            return true;
        }

        if rounds_since_last < self.checkpoint_config.min_rounds {
            return false;
        }

        if pending_count >= self.checkpoint_config.max_vertices {
            return true;
        }

        pending_count >= self.checkpoint_config.min_vertices
    }

    pub fn get_checkpoint_config(&self) -> &CheckpointConfig {
        &self.checkpoint_config
    }

    pub fn is_vertex_checkpointed(&self, id: &VertexId) -> bool {
        self.vertex_checkpoint_map.contains_key(id)
    }

    pub fn set_checkpoint_config(&mut self, config: CheckpointConfig) -> Result<()> {
        config.validate()?;
        self.checkpoint_config = config;
        Ok(())
    }

    pub fn latest_checkpoint(&self) -> Checkpoint {
        self.checkpoints
            .back()
            .cloned()
            .unwrap_or_else(Checkpoint::genesis)
    }

    pub fn last_checkpoint_round(&self) -> Round {
        self.last_checkpoint_round
    }

    pub fn get_checkpoint(&self, sequence: u64) -> Option<&Checkpoint> {
        self.checkpoints.iter().find(|cp| cp.sequence == sequence)
    }

    pub fn add_checkpoint(&mut self, checkpoint: Checkpoint) -> Result<()> {
        let latest = self.latest_checkpoint();
        let expected_seq = latest.sequence + 1;

        if checkpoint.sequence != expected_seq {
            if checkpoint.sequence == latest.sequence {
                let checkpoint_hash = checkpoint.hash()?;
                let latest_hash = latest.hash()?;
                if checkpoint_hash == latest_hash {
                    return Ok(());
                }
                if Self::same_checkpoint_payload_except_state_root(&latest, &checkpoint) {
                    if let Some(latest_checkpoint) = self.checkpoints.back_mut() {
                        *latest_checkpoint = checkpoint;
                    }
                    return Ok(());
                }
            }
            anyhow::bail!(
                "Invalid checkpoint sequence: expected {}, got {}",
                expected_seq,
                checkpoint.sequence
            );
        }

        let prev_hash = latest.hash()?;
        if checkpoint.prev_checkpoint_hash != prev_hash {
            anyhow::bail!("Invalid previous checkpoint hash");
        }

        self.validate_checkpoint_payload(&checkpoint)?;

        for tx in &checkpoint.transactions {
            self.executed_tx_hashes.insert(tx.hash());
        }

        for vertex_id in &checkpoint.vertices {
            self.vertex_checkpoint_map
                .insert(*vertex_id, checkpoint.sequence);
        }

        let checkpoint_vertices_set: HashSet<_> = checkpoint.vertices.iter().collect();
        self.pending_vertices
            .retain(|id| !checkpoint_vertices_set.contains(id));

        self.last_checkpoint_round = self.current_round;
        self.checkpoints.push_back(checkpoint.clone());

        if checkpoint.sequence > 10 {
            let cutoff_seq = checkpoint.sequence.saturating_sub(10);
            let vertices_to_remove: Vec<VertexId> = self
                .vertex_checkpoint_map
                .iter()
                .filter(|&(_, &seq)| seq <= cutoff_seq)
                .map(|(id, _)| *id)
                .collect();

            for vertex_id in vertices_to_remove {
                self.remove_vertex_fully(&vertex_id);
                self.vertex_checkpoint_map.remove(&vertex_id);
            }
        }

        const MAX_RETAIN_ROUNDS: u64 = 100;
        if self.current_round > MAX_RETAIN_ROUNDS {
            let cutoff_round = self.current_round.saturating_sub(MAX_RETAIN_ROUNDS);
            let orphaned_vertices: Vec<VertexId> = self
                .vertices
                .iter()
                .filter(|(_, v)| v.round < cutoff_round)
                .map(|(id, _)| *id)
                .collect();

            for id in orphaned_vertices {
                self.remove_vertex_fully(&id);
            }
        }

        const TX_RETENTION_WINDOW: usize = 10_000;
        if self.checkpoints.len() > TX_RETENTION_WINDOW
            && let Some(old_checkpoint) = self.checkpoints.pop_front() {
                for tx in &old_checkpoint.transactions {
                    self.executed_tx_hashes.remove(&tx.hash());
                }
            }

        Ok(())
    }

    pub fn get_checkpoint_stats(&self) -> CheckpointStats {
        CheckpointStats {
            pending_vertices: self.pending_vertices.len(),
            rounds_since_last: self
                .current_round
                .saturating_sub(self.last_checkpoint_round),
            total_checkpoints: self.checkpoints.len(),
            should_checkpoint: self.should_create_checkpoint(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckpointStats {
    pub pending_vertices: usize,
    pub rounds_since_last: u64,
    pub total_checkpoints: usize,
    pub should_checkpoint: bool,
}

impl DagConsensus {
    pub(crate) fn collect_checkpoint_transactions(
        &self,
        vertices_to_commit: &[VertexId],
    ) -> Vec<SignedTransaction> {
        let mut seen_tx_hashes = HashSet::new();
        let mut all_transactions = Vec::new();

        for vertex_id in vertices_to_commit {
            if let Some(vertex) = self.store.get_vertex(vertex_id) {
                for tx in &vertex.transactions {
                    let tx_hash = tx.hash();
                    if self.store.executed_tx_hashes.contains(&tx_hash) {
                        continue;
                    }
                    if seen_tx_hashes.insert(tx_hash) {
                        all_transactions.push(tx.clone());
                    }
                }
            }
        }

        all_transactions
    }

    pub(crate) fn checkpoint_state_root(
        &self,
        _vertices_to_commit: &[VertexId],
        _checkpoint_transactions: &[SignedTransaction],
    ) -> Result<Vec<u8>> {
        // DAG vertices carry speculative execution roots that may depend on a broader
        // parent ancestry than the transaction set that will ultimately be committed in
        // this checkpoint. The canonical checkpoint root must therefore be derived by the
        // execution engine when it replays `checkpoint.transactions` against the canonical
        // pre-checkpoint state.
        //
        // Until that canonical root is computed, consensus should advertise only the
        // latest finalized checkpoint root as a provisional placeholder instead of
        // reusing a vertex-local speculative root.
        Ok(self.store.latest_checkpoint().state_root.clone())
    }

    pub fn try_commit(&mut self) -> Result<Option<Checkpoint>> {
        let current_round = self.store.current_round();
        tracing::debug!(
            "[DAG Consensus] try_commit: current_round = {}",
            current_round
        );

        if current_round < 3 {
            return Ok(None);
        }

        let mut start_round = self.store.last_checkpoint_round() + 1;
        let max_commit_round = current_round.saturating_sub(2);

        if start_round > max_commit_round {
            return Ok(None);
        }

        tracing::info!(
            "[DAG Consensus] Catching up on missed rounds ({} to {})",
            start_round,
            max_commit_round
        );

        while start_round <= max_commit_round {
            let commit_round = start_round;

            let leader_id = if let Some(vrf_leader) = self.vrf_election.elect_leader(commit_round) {
                vrf_leader
            } else {
                let mut authorities: Vec<_> = self.committee.validators.keys().cloned().collect();
                authorities.sort();
                if authorities.is_empty() {
                    tracing::warn!(
                        "[DAG Consensus] Empty committee at round {}, skipping",
                        commit_round
                    );
                    start_round += 1;
                    continue;
                }
                let leader_idx = (commit_round as usize) % authorities.len();
                authorities[leader_idx].clone()
            };

            let leader_vertex = self
                .store
                .get_vertices_in_round(commit_round)
                .into_iter()
                .find(|v| v.author == *leader_id);

            if let Some(leader_vertex) = leader_vertex {
                let next_round_vertices = self.store.get_vertices_in_round(commit_round + 1);
                let trusted_support_count = next_round_vertices
                    .iter()
                    .filter(|v| v.parents.contains(&leader_vertex.id))
                    .map(|v| &v.author)
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .filter(|auth| self.store.is_authority_trusted(auth))
                    .count();

                let total_authorities = self.committee.validators.len();
                let quorum = calculate_quorum(total_authorities);

                if trusted_support_count >= quorum {
                    tracing::info!(
                        "[Consensus] Quorum reached! Support: {} / {} (threshold: {})",
                        trusted_support_count,
                        total_authorities,
                        quorum
                    );

                    let vertices_to_commit = self.collect_vertices_to_commit(leader_vertex.id)?;
                    let latest = self.store.latest_checkpoint();
                    let all_transactions =
                        self.collect_checkpoint_transactions(&vertices_to_commit);
                    let prev_hash = latest.hash()?;
                    let checkpoint = Checkpoint::new(
                        latest.sequence + 1,
                        vertices_to_commit.clone(),
                        all_transactions.clone(),
                        self.checkpoint_state_root(&vertices_to_commit, &all_transactions)?,
                        leader_vertex.timestamp,
                        prev_hash,
                    );

                    return Ok(Some(checkpoint));
                }
            }

            start_round += 1;
        }

        Ok(None)
    }

    pub fn add_checkpoint(&mut self, checkpoint: Checkpoint) -> Result<()> {
        self.store.add_checkpoint(checkpoint)
    }

    pub fn latest_checkpoint(&self) -> Checkpoint {
        self.store.latest_checkpoint()
    }
}
