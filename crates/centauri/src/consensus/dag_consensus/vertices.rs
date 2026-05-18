// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl DagConsensus {
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
        Self::with_chain_id(authority_id, authorities, "kanari-default".to_string())
    }

    fn with_chain_id_internal(
        authority_id: AuthorityId,
        authorities: Vec<AuthorityId>,
        chain_id: String,
        committee_public_keys: BTreeMap<AuthorityId, Vec<u8>>,
        local_vrf_secret: Option<[u8; 32]>,
        local_signing_key: ed25519_dalek::SigningKey,
    ) -> Self {
        tracing::info!(
            "[DAG Consensus] Initializing with authority_id: {}, chain_id: {}, committee: {:?}",
            authority_id,
            chain_id,
            authorities
        );
        let mut store = DagStore::new(authorities.clone());

        let mut vrf_election = VrfLeaderElection::new();
        if let Some(secret) = local_vrf_secret {
            vrf_election.register_authority_bytes(authority_id.clone(), &secret);
            tracing::info!("[VRF] Registered local VRF secret for {}", authority_id);
        } else {
            tracing::warn!("[VRF] No VRF secret provided - will use fallback round-robin");
        }

        let genesis_state_root = smt::default_hashes()[0].to_vec();
        let total_auths = authorities.len();
        // Genesis vertices must be byte-for-byte deterministic across authorities so
        // round-1 vertices can reference the same parent IDs on every node.
        let genesis_timestamp = 0;

        for authority in &authorities {
            let genesis_vertex = DagVertex::new(
                0,
                authority.clone(),
                chain_id.clone(),
                vec![],
                vec![],
                genesis_state_root.clone(),
                genesis_timestamp,
            );
            let _ = store.add_vertex(genesis_vertex, total_auths);
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

        let metrics = DagMetrics::new();
        let state_sync = StateSynchronizer::new();

        use super::super::vertex_broadcast::AdaptiveBatchConfig;
        let broadcaster = VertexBroadcaster::with_adaptive_config(
            10000,
            std::time::Duration::from_millis(50),
            AdaptiveBatchConfig::extreme_throughput(),
        );

        let persistent_store: Option<PersistentDagStore> = None;
        let pruner = DagPruner::new(PruningConfig::default()).unwrap_or_else(|e| {
            panic!(
                "Failed to create pruner with default config: {}. This is a programming error.",
                e
            )
        });
        let parallel_validator = ParallelValidator::new(ParallelValidatorConfig::high_throughput())
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to create parallel validator: {}. This is a programming error.",
                    e
                )
            });
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

        Self {
            store,
            authority_id,
            chain_id,
            vrf_election,
            byzantine_detector,
            caches,
            committee,
            metrics,
            state_sync,
            broadcaster,
            persistent_store,
            disk_writer_tx,
            pruner,
            parallel_validator,
            local_signing_key,
        }
    }

    pub fn with_chain_id(
        authority_id: AuthorityId,
        authorities: Vec<AuthorityId>,
        chain_id: String,
    ) -> Self {
        tracing::warn!(
            "[DAG Consensus] with_chain_id() uses deterministic demo keys. \
             Use with_chain_id_secure() for production-safe key management."
        );
        let committee_public_keys: BTreeMap<AuthorityId, Vec<u8>> = authorities
            .iter()
            .map(|auth| (auth.clone(), Self::authority_public_key(auth).to_vec()))
            .collect();

        let local_vrf_secret = Some(Self::authority_seed(&authority_id));
        let local_signing_key =
            ed25519_dalek::SigningKey::from_bytes(&Self::authority_seed(&authority_id));
        Self::with_chain_id_internal(
            authority_id,
            authorities,
            chain_id,
            committee_public_keys,
            local_vrf_secret,
            local_signing_key,
        )
    }

    pub fn with_chain_id_secure(
        authority_id: AuthorityId,
        authorities: Vec<AuthorityId>,
        chain_id: String,
        local_signing_key: ed25519_dalek::SigningKey,
        authority_public_keys: BTreeMap<AuthorityId, Vec<u8>>,
        local_vrf_secret: Option<[u8; 32]>,
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

        Ok(Self::with_chain_id_internal(
            authority_id,
            authorities,
            chain_id,
            authority_public_keys,
            local_vrf_secret,
            local_signing_key,
        ))
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

        let total_authorities = self.committee.validators.len();
        let quorum_size = calculate_quorum(total_authorities);

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

        let mut vertex = DagVertex::new(
            target_round,
            self.authority_id.clone(),
            self.chain_id.clone(),
            parents,
            transactions,
            state_root,
            timestamp,
        );
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

        let total_authorities = self.committee.validators.len();

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
            .check_vertex_validity(&vertex, total_authorities)
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
            .add_vertex_arc(Arc::clone(&vertex_arc), total_authorities)?;

        self.caches
            .vertices
            .insert(vertex_id, (*vertex_arc).clone());

        let is_priority = self.vrf_election.is_leader(vertex_arc.round, &author);

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

                self.vrf_election.update_current_round(current_round);
                self.vrf_election.prune_old_rounds(prune_stats.cutoff_round);

                let keep_checkpoints = latest_checkpoint.sequence.saturating_sub(100);
                self.state_sync
                    .prune_old_data(keep_checkpoints, prune_stats.cutoff_round);
            }
        }

        Ok(())
    }
}
