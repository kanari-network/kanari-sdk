// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::calculate_quorum;

impl DagStore {
    fn validate_pending_transactions(&self, vertex: &DagVertex) -> Result<()> {
        let mut local_hashes = HashSet::new();
        for tx in &vertex.transactions {
            let tx_hash = logical_tx_hash(tx);
            if self.executed_tx_hashes.contains(&tx_hash) {
                anyhow::bail!("Duplicate committed transaction");
            }
            if !local_hashes.insert(tx_hash) {
                anyhow::bail!("Duplicate transaction inside vertex");
            }
        }
        Ok(())
    }

    fn index_vertex(&mut self, vertex_id: VertexId, round: Round, author: AuthorityId) {
        self.vertices_by_round
            .entry(round)
            .or_default()
            .push(vertex_id);
        self.vertices_by_authority
            .entry(author)
            .or_default()
            .push(vertex_id);
    }

    pub(super) fn insert_vertex_arc(&mut self, vertex_id: VertexId, vertex_arc: Arc<DagVertex>) {
        let round = vertex_arc.round;
        let author = vertex_arc.author.clone();
        self.vertices.insert(vertex_id, vertex_arc);
        self.index_vertex(vertex_id, round, author);
    }

    fn remove_vertex_indexes(&mut self, vertex_id: &VertexId, round: Round, author: &AuthorityId) {
        if let Some(round_vertices) = self.vertices_by_round.get_mut(&round) {
            round_vertices.retain(|id| id != vertex_id);
            if round_vertices.is_empty() {
                self.vertices_by_round.remove(&round);
            }
        }

        if let Some(auth_vertices) = self.vertices_by_authority.get_mut(author) {
            auth_vertices.retain(|id| id != vertex_id);
            if auth_vertices.is_empty() {
                self.vertices_by_authority.remove(author);
            }
        }
    }

    pub(super) fn remove_vertex_fully(&mut self, vertex_id: &VertexId) -> Option<Arc<DagVertex>> {
        let vertex = self.vertices.remove(vertex_id)?;
        self.remove_vertex_indexes(vertex_id, vertex.round, &vertex.author);
        Some(vertex)
    }

    fn validate_new_vertex(
        &self,
        vertex: &DagVertex,
        required_quorum: usize,
        skip_integrity_check: bool,
    ) -> Result<()> {
        if !skip_integrity_check {
            vertex.verify()?;
        }
        if self.vertices.contains_key(&vertex.id) {
            anyhow::bail!("Vertex already exists");
        }
        self.validate_pending_transactions(vertex)?;

        if vertex.round > 0 {
            if !vertex.has_quorum_unique_authors(self, required_quorum) {
                anyhow::bail!("Vertex does not have quorum from unique authors");
            }

            let mut max_parent_timestamp = 0;
            let mut parent_timestamps = Vec::new();

            for parent_id in &vertex.parents {
                let parent = self
                    .vertices
                    .get(parent_id)
                    .ok_or_else(|| anyhow::anyhow!("Parent vertex not found"))?;

                if parent.round != vertex.round - 1 {
                    anyhow::bail!("Parent from wrong round");
                }

                if parent.timestamp > max_parent_timestamp {
                    max_parent_timestamp = parent.timestamp;
                }

                parent_timestamps.push(parent.timestamp);
            }

            if let Some((min_allowed, max_allowed)) = timestamp_bounds(&parent_timestamps) {
                if vertex.timestamp < min_allowed {
                    anyhow::bail!(
                        "Vertex timestamp {} is older than allowed minimum {}",
                        vertex.timestamp,
                        min_allowed
                    );
                }

                if vertex.timestamp > max_allowed {
                    anyhow::bail!(
                        "Vertex timestamp {} exceeds allowed maximum {}",
                        vertex.timestamp,
                        max_allowed
                    );
                }
            }

            if vertex.timestamp < max_parent_timestamp {
                anyhow::bail!("Vertex timestamp is older than its newest parent");
            }
        }

        Ok(())
    }

    pub fn new(authorities: Vec<AuthorityId>) -> Self {
        Self::with_config(authorities, CheckpointConfig::default())
    }

    pub fn with_config(authorities: Vec<AuthorityId>, config: CheckpointConfig) -> Self {
        let genesis_checkpoint = Checkpoint::genesis();
        let max_pending = if config.min_vertices >= 1000 {
            1_000_000
        } else {
            100_000
        };

        Self {
            vertices: HashMap::new(),
            vertices_by_round: BTreeMap::new(),
            vertices_by_authority: BTreeMap::new(),
            checkpoints: VecDeque::from([genesis_checkpoint]),
            pending_vertices: VecDeque::new(),
            executed_tx_hashes: HashSet::new(),
            current_round: 0,
            authorities: authorities.into_iter().collect(),
            vertex_checkpoint_map: HashMap::new(),
            checkpoint_config: config,
            last_checkpoint_round: 0,
            max_pending_vertices: max_pending,
            banned_authorities: HashSet::new(),
        }
    }

    pub fn should_apply_backpressure(&self) -> bool {
        self.pending_vertices.len() >= self.max_pending_vertices
    }

    pub fn add_vertex(&mut self, vertex: DagVertex, total_authorities: usize) -> Result<()> {
        let required_quorum = calculate_quorum(total_authorities);
        self.add_vertex_with_quorum(vertex, required_quorum)
    }

    pub fn add_vertex_with_quorum(
        &mut self,
        vertex: DagVertex,
        required_quorum: usize,
    ) -> Result<()> {
        self.add_vertex_arc_with_quorum(Arc::new(vertex), required_quorum)
    }

    pub fn add_vertex_arc(
        &mut self,
        vertex: Arc<DagVertex>,
        total_authorities: usize,
    ) -> Result<()> {
        let required_quorum = calculate_quorum(total_authorities);
        self.add_vertex_arc_with_quorum(vertex, required_quorum)
    }

    pub fn add_vertex_arc_with_quorum(
        &mut self,
        vertex: Arc<DagVertex>,
        required_quorum: usize,
    ) -> Result<()> {
        self.add_vertex_arc_with_quorum_internal(vertex, required_quorum, false)
    }

    pub(super) fn add_trusted_local_vertex_arc_with_quorum(
        &mut self,
        vertex: Arc<DagVertex>,
        required_quorum: usize,
    ) -> Result<()> {
        self.add_vertex_arc_with_quorum_internal(vertex, required_quorum, true)
    }

    fn add_vertex_arc_with_quorum_internal(
        &mut self,
        vertex: Arc<DagVertex>,
        required_quorum: usize,
        skip_integrity_check: bool,
    ) -> Result<()> {
        if self.should_apply_backpressure() {
            anyhow::bail!(
                "Backpressure applied: {} pending vertices (max: {})",
                self.pending_vertices.len(),
                self.max_pending_vertices
            );
        }
        self.validate_new_vertex(&vertex, required_quorum, skip_integrity_check)?;
        if vertex.round > self.current_round {
            self.current_round = vertex.round;
        }
        let vertex_id = vertex.id;
        self.insert_vertex_arc(vertex_id, vertex);
        self.pending_vertices.push_back(vertex_id);
        Ok(())
    }

    pub fn get_vertex(&self, id: &VertexId) -> Option<&Arc<DagVertex>> {
        self.vertices.get(id)
    }

    pub fn get_vertices_in_round(&self, round: Round) -> Vec<Arc<DagVertex>> {
        let mut vertices: Vec<Arc<DagVertex>> = self
            .vertices_by_round
            .get(&round)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.vertices.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default();
        vertices.sort_by_key(|vertex| vertex.id);
        vertices
    }

    pub fn get_vertex_ids_in_round(&self, round: Round) -> Vec<VertexId> {
        let mut ids = self
            .vertices_by_round
            .get(&round)
            .cloned()
            .unwrap_or_default();
        ids.sort();
        ids
    }

    pub fn get_vertices_by_authority(&self, authority: &AuthorityId) -> Vec<Arc<DagVertex>> {
        self.vertices_by_authority
            .get(authority)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.vertices.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn current_round(&self) -> Round {
        self.current_round
    }

    pub fn num_authorities(&self) -> usize {
        self.authorities.len()
    }

    pub fn ban_authority(&mut self, authority: &AuthorityId) {
        self.banned_authorities.insert(authority.clone());
        tracing::warn!(
            "[Security] Authority {} has been BANNED - excluded from quorum",
            authority
        );
    }

    pub fn is_authority_trusted(&self, authority: &AuthorityId) -> bool {
        !self.banned_authorities.contains(authority)
    }
}
