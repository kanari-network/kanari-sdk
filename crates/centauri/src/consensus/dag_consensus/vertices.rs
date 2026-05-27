// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl DagConsensus {
    pub fn ensure_production_allowed(
        &self,
        plan: &DagProductionPlan,
        tx_count: usize,
    ) -> Result<()> {
        let is_genesis_bootstrap_round = plan.policy.parent_round == 0
            && plan.policy.parent_author_count >= plan.policy.quorum_size;

        if tx_count == 0 && plan.history_vertices.is_empty() && !is_genesis_bootstrap_round {
            anyhow::bail!("No new transactions and no history to commit");
        }

        if tx_count == 0
            && plan.policy.parent_round > 0
            && plan.policy.parent_author_count < plan.policy.quorum_size
        {
            anyhow::bail!(
                "DAG_WAITING: not producing empty vertex for round {} with partial parents ({}/{})",
                plan.policy.target_round,
                plan.policy.parent_author_count,
                plan.policy.quorum_size
            );
        }

        Ok(())
    }

    pub fn production_plan(&self) -> Result<DagProductionPlan> {
        let policy = self.production_policy();
        let history_vertices = self.collect_history_for_parents(&policy.parent_ids)?;
        let mut history_tx_hashes = BTreeSet::new();
        for vertex_id in &history_vertices {
            if let Some(vertex) = self.store.get_vertex(vertex_id) {
                for tx in &vertex.transactions {
                    history_tx_hashes.insert(tx.transaction.hash());
                }
            }
        }

        Ok(DagProductionPlan {
            policy,
            history_vertices,
            history_tx_hashes,
        })
    }

    pub fn select_pending_transactions<F>(
        &self,
        plan: &DagProductionPlan,
        pending: &[SignedTransaction],
        mut is_externally_executed: F,
    ) -> DagPendingSelection
    where
        F: FnMut(&[u8]) -> bool,
    {
        let mut included = Vec::new();
        let mut remove_hashes = Vec::new();

        for tx in pending.iter().take(500_000) {
            let hash = tx.transaction.hash();

            if plan.history_tx_hashes.contains(&hash) {
                continue;
            }

            if self.has_executed_transaction(&hash) || is_externally_executed(&hash) {
                remove_hashes.push(hash);
            } else {
                included.push(tx.clone());
            }
        }

        DagPendingSelection {
            included,
            remove_hashes,
        }
    }

    pub fn execution_plan_for_current_txs<F>(
        &self,
        history_vertices: &[VertexId],
        current_txs: &[SignedTransaction],
        include_history: bool,
        mut is_externally_executed: F,
    ) -> DagExecutionPlan
    where
        F: FnMut(&[u8]) -> bool,
    {
        let mut seen_tx_hashes = HashSet::new();
        let mut transactions = Vec::new();

        if include_history {
            for vertex_id in history_vertices {
                if let Some(vertex) = self.store.get_vertex(vertex_id) {
                    for tx in &vertex.transactions {
                        let tx_hash = tx.transaction.hash();
                        if seen_tx_hashes.insert(tx_hash.clone())
                            && !self.has_executed_transaction(&tx_hash)
                            && !is_externally_executed(&tx_hash)
                        {
                            transactions.push(tx.clone());
                        }
                    }
                }
            }
        }

        for tx in current_txs {
            let tx_hash = tx.transaction.hash();
            if seen_tx_hashes.insert(tx_hash) {
                transactions.push(tx.clone());
            }
        }

        DagExecutionPlan {
            transactions,
            history_vertices: history_vertices.to_vec(),
        }
    }

    pub fn execution_plan_for_network_vertex<F>(
        &self,
        vertex: &DagVertex,
        mut is_externally_executed: F,
    ) -> Result<DagExecutionPlan>
    where
        F: FnMut(&[u8]) -> bool,
    {
        let history_vertices = self.collect_history_for_parents(&vertex.parents)?;
        Ok(self.execution_plan_for_current_txs(
            &history_vertices,
            &vertex.transactions,
            !vertex.transactions.is_empty(),
            |tx_hash| is_externally_executed(tx_hash),
        ))
    }

    pub fn production_policy(&self) -> DagProductionPolicy {
        let current_round = self.store.current_round();
        let current_round_vertices = self.store.get_vertices_in_round(current_round);
        let local_has_vertex_in_current_round = current_round_vertices
            .iter()
            .any(|vertex| vertex.author == self.authority_id);
        let current_round_parent_author_count = current_round_vertices
            .iter()
            .map(|vertex| vertex.author.clone())
            .collect::<HashSet<_>>()
            .len();
        let quorum_size = self.quorum_threshold();

        let (parent_round, target_round, parent_ids, parent_author_count, using_catch_up_round) =
            if current_round > 0
                && !local_has_vertex_in_current_round
                && current_round_parent_author_count < quorum_size
            {
                let catch_up_parent_round = current_round.saturating_sub(1);
                let parent_ids = self.store.get_vertex_ids_in_round(catch_up_parent_round);
                let parent_author_count = parent_ids
                    .iter()
                    .filter_map(|parent_id| self.store.get_vertex(parent_id))
                    .map(|vertex| vertex.author.clone())
                    .collect::<HashSet<_>>()
                    .len();

                (
                    catch_up_parent_round,
                    current_round,
                    parent_ids,
                    parent_author_count,
                    true,
                )
            } else {
                (
                    current_round,
                    current_round + 1,
                    current_round_vertices
                        .iter()
                        .map(|vertex| vertex.id)
                        .collect(),
                    current_round_parent_author_count,
                    false,
                )
            };

        DagProductionPolicy {
            current_round,
            parent_round,
            target_round,
            parent_ids,
            parent_author_count,
            quorum_size,
            local_has_vertex_in_current_round,
            using_catch_up_round,
        }
    }

    pub fn progress_policy(&self) -> DagProgressPolicy {
        let current_round = self.store.current_round();
        let last_checkpoint_round = self.store.last_checkpoint_round();
        let latest_local_round = self
            .store
            .get_vertices_by_authority(&self.authority_id)
            .into_iter()
            .map(|vertex| vertex.round)
            .max()
            .unwrap_or(0);

        DagProgressPolicy {
            current_round,
            last_checkpoint_round,
            latest_local_round,
        }
    }

    pub fn needs_progress(&self) -> bool {
        self.progress_policy().needs_progress()
    }

    pub fn suggest_vertex_timestamp_for_plan(
        &self,
        plan: &DagProductionPlan,
        proposed_timestamp: u64,
    ) -> u64 {
        self.suggest_vertex_timestamp_for_parents(&plan.policy.parent_ids, proposed_timestamp)
    }

    pub fn create_vertex_from_plan(
        &mut self,
        plan: &DagProductionPlan,
        transactions: Vec<SignedTransaction>,
        state_root: Vec<u8>,
        timestamp: u64,
    ) -> Result<DagVertex> {
        self.create_vertex_for_round(
            plan.policy.target_round,
            plan.policy.parent_ids.clone(),
            transactions,
            state_root,
            timestamp,
        )
    }

    pub fn latest_vertices_by_authority(
        &self,
        authority: &AuthorityId,
        limit: usize,
    ) -> Vec<DagVertex> {
        let mut vertices: Vec<_> = self
            .store
            .get_vertices_by_authority(authority)
            .into_iter()
            .map(|vertex| (*vertex).clone())
            .collect();

        vertices.sort_by_key(|vertex| vertex.round);
        let keep_from = vertices.len().saturating_sub(limit);
        vertices.split_off(keep_from)
    }

    pub fn should_ignore_far_future_empty_vertex(
        &self,
        vertex_round: Round,
        max_future_rounds: u64,
    ) -> bool {
        self.store.current_round() > 0
            && vertex_round > self.store.current_round().saturating_add(max_future_rounds)
    }

    pub fn classify_network_vertex(
        &self,
        vertex: &DagVertex,
        max_future_rounds: u64,
    ) -> DagNetworkVertexAction {
        if self.has_vertex(&vertex.id) {
            return DagNetworkVertexAction::IgnoreExisting;
        }

        if vertex.transactions.is_empty()
            && self.should_ignore_far_future_empty_vertex(vertex.round, max_future_rounds)
        {
            return DagNetworkVertexAction::IgnoreFarFutureEmpty {
                current_round: self.store.current_round(),
            };
        }

        DagNetworkVertexAction::Accept
    }

    pub fn add_vertex_and_try_commit(&mut self, vertex: DagVertex) -> Result<Option<Checkpoint>> {
        self.add_vertex(vertex)?;
        self.try_commit()
    }

    pub fn try_new(authority_id: AuthorityId, authorities: Vec<AuthorityId>) -> Result<Self> {
        Self::try_with_chain_id(authority_id, authorities, "kanari-default".to_string())
    }

    pub fn try_mysticeti(authority_id: AuthorityId, authorities: Vec<AuthorityId>) -> Result<Self> {
        let protocol = ConsensusProtocol::default_for_committee_size(authorities.len());
        Self::try_with_chain_id_and_protocol(
            authority_id,
            authorities,
            "kanari-default".to_string(),
            protocol,
        )
    }

    fn authority_seed(authority: &str) -> [u8; 32] {
        let mut seed = [0u8; 32];
        let digest = hash_data_blake3(authority.as_bytes());
        seed.copy_from_slice(&digest[..32]);
        seed
    }

    fn authority_public_key(authority: &str) -> [u8; 32] {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&Self::authority_seed(authority));
        signing_key.verifying_key().to_bytes()
    }

    fn signing_payload(vertex: &DagVertex) -> Result<Vec<u8>> {
        let mut to_sign = vertex.clone();
        to_sign.signature.clear();
        bcs::to_bytes(&to_sign).map_err(|e| anyhow::anyhow!("Failed to serialize vertex: {}", e))
    }

    fn sign_vertex_with_key(
        signing_key: &ed25519_dalek::SigningKey,
        vertex: &mut DagVertex,
    ) -> Result<()> {
        let payload = Self::signing_payload(vertex)?;
        let keypair = Ed25519Keypair {
            signing_key: signing_key.clone(),
            verifying_key: signing_key.verifying_key(),
        };
        vertex.signature = keypair.sign(&payload);
        Ok(())
    }

    fn verify_vertex_signature(&mut self, vertex: &DagVertex) -> Result<()> {
        let validator = self
            .committee
            .get_validator(&vertex.author)
            .ok_or_else(|| anyhow::anyhow!("Unknown vertex author {}", vertex.author))?;
        let key_bytes: [u8; 32] = validator
            .public_key
            .as_slice()
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid public key length for {}", vertex.author))?;
        let public_key = ed25519_dalek::VerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| anyhow::anyhow!("Invalid public key for {}: {}", vertex.author, e))?;
        let result = self
            .parallel_validator
            .validate_vertex_with_public_key(vertex, &public_key)?;
        if result.is_valid {
            return Ok(());
        }
        anyhow::bail!(
            "Vertex validation failed: {}",
            result.error.as_deref().unwrap_or("unknown error")
        )
    }

    pub fn new(authority_id: AuthorityId, authorities: Vec<AuthorityId>) -> Self {
        Self::try_new(authority_id, authorities)
            .expect("DagConsensus::new failed - use try_new() for error handling")
    }

    fn with_chain_id_internal(
        authority_id: AuthorityId,
        authorities: Vec<AuthorityId>,
        chain_id: String,
        consensus_protocol: ConsensusProtocol,
        committee_public_keys: BTreeMap<AuthorityId, Vec<u8>>,
        local_signing_key: ed25519_dalek::SigningKey,
    ) -> Result<Self> {
        tracing::info!(
            "[DAG Consensus] Initializing with authority_id: {}, chain_id: {}, committee: {:?}",
            authority_id,
            chain_id,
            authorities
        );
        let mut store = DagStore::new(authorities.clone());

        let genesis_state_root = smt::default_hashes()[0].to_vec();
        let genesis_quorum = 0;
        // Genesis vertices must be byte-for-byte deterministic across authorities so
        // round-1 vertices can reference the same parent IDs on every node.
        let genesis_timestamp = 0;

        for authority in &authorities {
            let genesis_vertex = DagVertex::try_new(
                0,
                authority.clone(),
                chain_id.clone(),
                vec![],
                vec![],
                genesis_state_root.clone(),
                genesis_timestamp,
            )?;
            store.add_vertex(genesis_vertex, genesis_quorum)?;
        }

        let mut byzantine_detector = ByzantineDetector::new();
        for authority in &authorities {
            byzantine_detector.init_authority(authority.clone());
        }

        let caches = DagCaches::extreme_throughput();

        let validator_infos: Vec<ValidatorInfo> = authorities
            .iter()
            .enumerate()
            .map(|(i, auth)| ValidatorInfo {
                authority_id: auth.clone(),
                public_key: committee_public_keys
                    .get(auth)
                    .cloned()
                    .unwrap_or_else(|| Self::authority_public_key(auth).to_vec()),
                network_address: format!("validator-{}", i),
                active: true,
            })
            .collect();
        let committee = Committee::new(0, validator_infos);
        let protocol = consensus_protocol.to_protocol(committee.validators.len())?;

        let metrics = DagMetrics::new();
        let state_sync = StateSynchronizer::new();

        use super::super::vertex_broadcast::AdaptiveBatchConfig;
        let broadcaster = VertexBroadcaster::with_adaptive_config(
            10000,
            std::time::Duration::from_millis(50),
            AdaptiveBatchConfig::extreme_throughput(),
        );

        let persistent_store: Option<PersistentDagStore> = None;
        let pruner = DagPruner::new(PruningConfig::default())?;
        let parallel_validator =
            ParallelValidator::new(ParallelValidatorConfig::high_throughput())?;
        let disk_writer_tx = if persistent_store.is_some() {
            let (tx, mut rx) = mpsc::channel::<Arc<DagVertex>>(100_000);
            let persistent_clone = persistent_store.clone();

            tokio::spawn(async move {
                while let Some(vertex) = rx.recv().await {
                    if let Some(ref store) = persistent_clone
                        && let Err(e) = store.put_vertex(&vertex)
                    {
                        tracing::error!(
                            "Failed to persist vertex {}: {}",
                            hex::encode(vertex.id),
                            e
                        );
                    }
                }
            });

            Some(tx)
        } else {
            None
        };

        Ok(Self {
            store,
            authority_id,
            chain_id,
            byzantine_detector,
            caches,
            committee,
            protocol,
            metrics,
            state_sync,
            broadcaster,
            persistent_store,
            disk_writer_tx,
            pruner,
            parallel_validator,
            local_signing_key,
        })
    }

    pub fn with_chain_id(
        authority_id: AuthorityId,
        authorities: Vec<AuthorityId>,
        chain_id: String,
    ) -> Self {
        Self::try_with_chain_id(authority_id, authorities, chain_id).expect(
            "DagConsensus::with_chain_id failed - use try_with_chain_id() for error handling",
        )
    }

    pub fn try_with_chain_id(
        authority_id: AuthorityId,
        authorities: Vec<AuthorityId>,
        chain_id: String,
    ) -> Result<Self> {
        let protocol = ConsensusProtocol::default_for_committee_size(authorities.len());
        Self::try_with_chain_id_and_protocol(authority_id, authorities, chain_id, protocol)
    }

    pub fn try_with_chain_id_and_protocol(
        authority_id: AuthorityId,
        authorities: Vec<AuthorityId>,
        chain_id: String,
        consensus_protocol: ConsensusProtocol,
    ) -> Result<Self> {
        tracing::warn!(
            "[DAG Consensus] with_chain_id() uses deterministic demo keys. \
             Use with_chain_id_secure() for production-safe key management."
        );
        let committee_public_keys: BTreeMap<AuthorityId, Vec<u8>> = authorities
            .iter()
            .map(|auth| (auth.clone(), Self::authority_public_key(auth).to_vec()))
            .collect();

        let local_signing_key =
            ed25519_dalek::SigningKey::from_bytes(&Self::authority_seed(&authority_id));
        Self::with_chain_id_internal(
            authority_id,
            authorities,
            chain_id,
            consensus_protocol,
            committee_public_keys,
            local_signing_key,
        )
    }

    pub fn with_chain_id_secure(
        authority_id: AuthorityId,
        authorities: Vec<AuthorityId>,
        chain_id: String,
        local_signing_key: ed25519_dalek::SigningKey,
        authority_public_keys: BTreeMap<AuthorityId, Vec<u8>>,
    ) -> Result<Self> {
        for auth in &authorities {
            let key = authority_public_keys
                .get(auth)
                .ok_or_else(|| anyhow::anyhow!("Missing public key for authority {}", auth))?;
            let key_bytes: [u8; 32] = key
                .as_slice()
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid public key length for {}", auth))?;
            ed25519_dalek::VerifyingKey::from_bytes(&key_bytes)
                .map_err(|e| anyhow::anyhow!("Invalid public key for {}: {}", auth, e))?;
        }
        let local_pk = local_signing_key.verifying_key().to_bytes().to_vec();
        let expected_local = authority_public_keys.get(&authority_id).ok_or_else(|| {
            anyhow::anyhow!("Missing local authority public key {}", authority_id)
        })?;
        if *expected_local != local_pk {
            anyhow::bail!("Local signing key does not match authority public key");
        }

        Self::with_chain_id_internal(
            authority_id,
            authorities,
            chain_id,
            ConsensusProtocol::default_for_committee_size(authority_public_keys.len()),
            authority_public_keys,
            local_signing_key,
        )
    }

    pub fn suggest_vertex_timestamp(&self, proposed_timestamp: u64) -> u64 {
        let current_round = self.store.current_round();
        let parent_ids = self.store.get_vertex_ids_in_round(current_round);
        self.suggest_vertex_timestamp_for_parents(&parent_ids, proposed_timestamp)
    }

    fn suggest_vertex_timestamp_for_parents(
        &self,
        parent_ids: &[VertexId],
        proposed_timestamp: u64,
    ) -> u64 {
        let parent_timestamps: Vec<u64> = parent_ids
            .iter()
            .filter_map(|parent_id| self.store.get_vertex(parent_id).map(|v| v.timestamp))
            .collect();

        timestamp_bounds(&parent_timestamps)
            .map_or(proposed_timestamp, |(min_allowed, max_allowed)| {
                proposed_timestamp.clamp(min_allowed, max_allowed)
            })
    }

    pub fn create_vertex(
        &mut self,
        transactions: Vec<SignedTransaction>,
        state_root: Vec<u8>,
        timestamp: u64,
    ) -> Result<DagVertex> {
        let current_round = self.store.current_round();
        let next_round = current_round + 1;
        let parents = self.store.get_vertex_ids_in_round(current_round);
        self.create_vertex_for_round(next_round, parents, transactions, state_root, timestamp)
    }

    pub fn create_vertex_for_round(
        &mut self,
        target_round: Round,
        mut parents: Vec<VertexId>,
        transactions: Vec<SignedTransaction>,
        state_root: Vec<u8>,
        timestamp: u64,
    ) -> Result<DagVertex> {
        if target_round == 0 {
            anyhow::bail!("Cannot create a non-genesis vertex for round 0");
        }

        let mut unique_authors = HashSet::new();
        for parent_id in &parents {
            if let Some(parent_vertex) = self.store.get_vertex(parent_id)
                && self.store.is_authority_trusted(&parent_vertex.author)
            {
                unique_authors.insert(parent_vertex.author.clone());
            }
        }

        let quorum_size = self.quorum_threshold();

        if unique_authors.len() < quorum_size {
            anyhow::bail!(
                "Cannot create vertex for round {}: Not enough parents for quorum. Have {}, need {}",
                target_round,
                unique_authors.len(),
                quorum_size
            );
        }

        parents.sort();
        let timestamp = self.suggest_vertex_timestamp_for_parents(&parents, timestamp);

        let mut vertex = DagVertex::try_new(
            target_round,
            self.authority_id.clone(),
            self.chain_id.clone(),
            parents,
            transactions,
            state_root,
            timestamp,
        )?;
        Self::sign_vertex_with_key(&self.local_signing_key, &mut vertex)?;
        Ok(vertex)
    }

    pub fn add_vertex(&mut self, vertex: DagVertex) -> Result<()> {
        if vertex.chain_id != self.chain_id {
            anyhow::bail!(
                "Cross-chain replay attack detected! Expected chain_id '{}', got '{}'",
                self.chain_id,
                vertex.chain_id
            );
        }

        let vertex_id = vertex.id;
        let author = vertex.author.clone();

        if !self.committee.contains(&author) {
            tracing::error!(
                "[DAG Consensus] Committee check failed for author: '{}'. Committee members: {:?}",
                author,
                self.committee.validators.keys().collect::<Vec<_>>()
            );
            anyhow::bail!("Vertex author '{}' is not in current committee", author);
        }

        if !self.store.is_authority_trusted(&author) {
            tracing::warn!(
                "[Security] REJECTED vertex from BANNED authority: {}",
                author
            );
            return Err(anyhow::anyhow!(
                "Vertex from banned authority '{}' rejected",
                author
            ));
        }

        self.verify_vertex_signature(&vertex)?;

        for parent_id in &vertex.parents {
            if self.caches.vertices.get(parent_id).is_none()
                && !self.store.vertices.contains_key(parent_id)
            {
                anyhow::bail!("Parent vertex {} not found", hex::encode(parent_id));
            }
        }

        let required_quorum = self.quorum_threshold();

        if let Err(e) = self.byzantine_detector.check_double_voting(&vertex) {
            if self.byzantine_detector.get_reputation(&author) == 0 {
                tracing::error!(
                    "[Security] Authority {} SLASHED to 0 reputation - BANNING from consensus",
                    author
                );
                self.store.ban_authority(&author);
            }
            return Err(e);
        }

        if let Err(e) = self
            .byzantine_detector
            .check_vertex_validity(&vertex, required_quorum)
        {
            if self.byzantine_detector.get_reputation(&author) == 0 {
                tracing::error!(
                    "[Security] Authority {} SLASHED to 0 reputation for invalid vertex - BANNING",
                    author
                );
                self.store.ban_authority(&author);
            }
            return Err(e);
        }

        let vertex_arc = Arc::new(vertex);

        if let Some(ref tx) = self.disk_writer_tx {
            match tx.try_send(Arc::clone(&vertex_arc)) {
                Ok(()) => {}
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    self.metrics.inc_disk_queue_full_count();
                    tracing::error!(
                        "[CRITICAL] Disk write queue FULL! Rejecting vertex {} to prevent data loss. \
                         Node must slow down or increase queue capacity.",
                        hex::encode(vertex_id)
                    );
                    return Err(anyhow::anyhow!(
                        "Disk write queue saturated. Vertex {} rejected to prevent data loss",
                        hex::encode(vertex_id)
                    ));
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    tracing::error!(
                        "[FATAL] Disk writer task closed! Vertex {} will not be persisted.",
                        hex::encode(vertex_id)
                    );

                    return Err(anyhow::anyhow!(
                        "Disk writer task crashed - system unhealthy"
                    ));
                }
            }
        }

        self.store
            .add_vertex_arc_with_quorum(Arc::clone(&vertex_arc), required_quorum)?;

        self.caches
            .vertices
            .insert(vertex_id, (*vertex_arc).clone());

        let is_priority = self
            .mysticeti_commit_leaders(vertex_arc.round)
            .iter()
            .any(|leader| leader == &author);

        self.broadcaster
            .add_vertex_arc(Arc::clone(&vertex_arc), is_priority);
        self.state_sync.add_vertex_arc(vertex_arc);

        let current_round = self.store.current_round();
        if self.pruner.should_prune(current_round)
            && let Some(persistent) = &self.persistent_store
        {
            let latest_checkpoint = self.store.latest_checkpoint();
            if let Ok(prune_stats) =
                self.pruner
                    .prune(persistent, current_round, Some(latest_checkpoint.sequence))
            {
                self.parallel_validator
                    .invalidate_pruned_vertices(&prune_stats.pruned_vertex_ids);

                for vertex_id in &prune_stats.pruned_vertex_ids {
                    self.caches.vertices.remove(vertex_id);
                }

                self.byzantine_detector
                    .prune_old_rounds(prune_stats.cutoff_round);

                let keep_checkpoints = latest_checkpoint.sequence.saturating_sub(100);
                self.state_sync
                    .prune_old_data(keep_checkpoints, prune_stats.cutoff_round);
            }
        }

        Ok(())
    }
}
