// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use centauri::blockchain::Blockchain;
use centauri::consensus::{Checkpoint, DagMetrics, PersistentDagState};
use kanari_move_runtime_v1::changeset::ChangeSet;
use kanari_move_runtime_v1::move_runtime::MoveRuntime;
use kanari_move_runtime_v1::state::StateManager;
use kanari_move_runtime_v1::storage::persistent_store::PersistentStore;
pub use kanari_rpc_api::{AccountInfo, BlockData, BlockchainStats, FullBlockData, ObjectInfo};
use kanari_types::address::Address as KanariAddress;
use kanari_types::event::Event;
use kanari_types::gas_v2::{GasMeter, GasOperation};
use kanari_types::transaction::{SignedTransaction, Transaction};
use log::{error, info, warn};
use lru::LruCache;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use num_cpus;
use rayon::prelude::*;
use std::collections::HashSet;
use std::env;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

type ProofCache = LruCache<(u64, usize), (String, Vec<Vec<u8>>)>;

mod apply_checkpoint;
mod bootstrap;
mod dag_integration;
mod mempool;
mod produce_dag_vertex;
mod queries;
mod runtime_guards;
pub use produce_dag_vertex::{CheckpointInfo, DagBlockInfo, DagEngine};
pub use runtime_guards::{RuntimeGuardConfig, RuntimeHealthReport};

pub type BlockInfo = DagBlockInfo;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckpointSyncData {
    pub checkpoint: Checkpoint,
}

pub type ExecutionResult = Result<(Vec<u8>, ChangeSet)>;
pub type ParallelTxResult = (SignedTransaction, ExecutionResult);

const MAX_MEMPOOL_SIZE: usize = 50_000;

/// Complete blockchain engine with Move VM integration
pub struct BlockchainEngine {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub state: Arc<RwLock<StateManager>>,
    pub pending_txs: Arc<RwLock<Vec<SignedTransaction>>>,
    pending_tx_hashes: Arc<RwLock<HashSet<Vec<u8>>>>,
    pub persistent_store: Option<Arc<PersistentStore>>,
    // Reusable pool of MoveRuntime instances for parallel execution
    pub runtime_pool: Vec<kanari_move_runtime_v1::move_runtime::MoveRuntime>,
    // LRU cache for frequently requested merkle proofs
    // Cache key: (block_height, tx_index), Value: (tx_hash, proof)
    pub proof_cache: Arc<RwLock<ProofCache>>,
    // DAG engine for high-throughput consensus (lazy-initialized)
    dag_engine: Arc<RwLock<Option<DagEngine>>>,
    // Authority ID for this node (used in DAG mode)
    authority_id: String,
    // List of all authorities (validators) in the network
    authorities: Vec<String>,
    // Persisted DAG state, loaded on startup
    persisted_dag_state: Option<PersistentDagState>,
}

// Basic recursive parser for simple type-argument strings used by RPC/tests.
fn parse_type_tag(s: &str) -> Option<TypeTag> {
    fn split_top_level_commas(s: &str) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut depth: usize = 0;
        let mut start = 0usize;
        for (i, ch) in s.char_indices() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    depth = depth.saturating_sub(1);
                }
                ',' if depth == 0 => {
                    parts.push(s[start..i].trim());
                    start = i + 1;
                }
                _ => {}
            }
        }
        parts.push(s[start..].trim());
        parts
    }

    let s = s.trim();
    match s {
        "bool" => return Some(TypeTag::Bool),
        "u8" => return Some(TypeTag::U8),
        "u64" => return Some(TypeTag::U64),
        "u128" => return Some(TypeTag::U128),
        "address" => return Some(TypeTag::Address),
        _ => {}
    }

    if let Some(inner) = s.strip_prefix("vector<")
        && let Some(inner) = inner.strip_suffix('>')
    {
        return parse_type_tag(inner).map(|t| TypeTag::Vector(Box::new(t)));
    }

    if s.contains("::") {
        let parts = s.split("::").collect::<Vec<_>>();
        if parts.len() >= 3 {
            let addr_str = parts[0].trim();
            let module_str = parts[1].trim();
            let name_and_generics = parts[2..].join("::").trim().to_string();

            let (name_str, generics_opt) = if let Some(idx) = name_and_generics.find('<') {
                if !name_and_generics.ends_with('>') || idx + 1 >= name_and_generics.len() {
                    return None;
                }
                let name = &name_and_generics[..idx];
                let generics = &name_and_generics[idx + 1..name_and_generics.len() - 1];
                (name.trim(), Some(generics))
            } else {
                (name_and_generics.as_str(), None)
            };

            let addr = KanariAddress::parse_to_account_address(addr_str).ok()?;
            let module_id = Identifier::new(module_str).ok()?;
            let name_id = Identifier::new(name_str).ok()?;

            let mut type_params = Vec::new();
            if let Some(r#gen) = generics_opt {
                for g in split_top_level_commas(r#gen) {
                    if g.is_empty() {
                        continue;
                    }
                    let parsed = parse_type_tag(g)?;
                    type_params.push(parsed);
                }
            }

            let st = StructTag {
                address: addr,
                module: module_id,
                name: name_id,
                type_params,
            };
            return Some(TypeTag::Struct(Box::new(st)));
        }
    }

    None
}

impl BlockchainEngine {
    // =====================================================================
    // ⏰ System Prologue
    // =====================================================================
    pub fn execute_system_prologue(&self, timestamp_ms: u64) -> Result<()> {
        let runtime = &self.runtime_pool[0];

        // Acquire the write lock once to ensure atomicity of the entire operation
        let mut state_write = match self.state.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::error!("State lock poisoned in system prologue, recovering...");
                poisoned.into_inner()
            }
        };

        // Get the clock ID
        let clock_id = runtime.ensure_system_clock(&mut state_write)?;

        // Execute the prologue function to get the changeset
        let changeset = runtime.execute_clock_consensus_commit_prologue(clock_id, timestamp_ms)?;

        // Apply the changeset to the state
        state_write.apply_changeset(&changeset)?;
        runtime.persist_created_objects(&changeset);
        runtime.persist_deleted_objects(&changeset);
        Ok(())
    }

    // =====================================================================
    // 💡 HELPER FUNCTIONS
    // =====================================================================

    pub fn get_expected_sequence(&self, address_hex: &str) -> u64 {
        let mut seq = self
            .state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get_account_by_hex(address_hex)
            .map(|acc| acc.sequence_number)
            .unwrap_or(0);

        self.for_each_pending_tx_from_sender(address_hex, |_| seq += 1);
        seq
    }

    fn resolve_account_objects(
        &self,
        state: &StateManager,
        owner_addr: &AccountAddress,
    ) -> Vec<ObjectInfo> {
        let raw_owned_ids = state.get_owned_objects(owner_addr).unwrap_or_default();
        let unique_ids: std::collections::HashSet<_> = raw_owned_ids.into_iter().collect();

        let mut coins = Vec::new();
        let mut others = Vec::new();

        for id in unique_ids {
            if let Ok(Some(obj)) = state.get_object(&id) {
                let info = ObjectInfo {
                    id: id.clone(),
                    owner: format!("{:#x}", obj.owner),
                    type_: obj.type_.clone(),
                    data: obj.data.clone(),
                    version: obj.version,
                };

                if obj.type_.contains("::coin::Coin<") && obj.data.len() >= 40 {
                    let mut arr = [0u8; 8];
                    arr.copy_from_slice(&obj.data[32..40]);
                    let amount = u64::from_le_bytes(arr);
                    if amount > 0 {
                        coins.push((amount, info));
                        continue;
                    }
                }
                others.push(info);
            }
        }

        coins.sort_by(|a, b| b.0.cmp(&a.0));
        coins
            .into_iter()
            .map(|(_, info)| info)
            .chain(others)
            .collect()
    }

    pub(crate) fn execute_tx_waves_parallel(
        &self,
        transactions: Vec<SignedTransaction>,
        state_arc: &Arc<RwLock<StateManager>>,
        timestamp: Option<u64>,
        persist_objects: bool,
        strict_mode: bool,
    ) -> Result<(usize, usize)> {
        let mut executed_count = 0;
        let mut failed_count = 0;
        let has_module_publish = transactions
            .iter()
            .any(|tx| matches!(tx.transaction, Transaction::PublishModule { .. }));

        if strict_mode {
            if has_module_publish {
                self.runtime_pool[0].reload_vm_cache()?;
            }

            let mut executed_count = 0;
            let failed_count = 0;
            for signed_tx in transactions {
                let changeset = self.execute_transaction_with_runtime_internal(
                    &signed_tx.transaction,
                    &self.runtime_pool[0],
                    state_arc,
                    false,
                    timestamp,
                    persist_objects,
                )?;

                let mut state_write = match state_arc.write() {
                    Ok(guard) => guard,
                    Err(poisoned) => {
                        log::error!("State lock poisoned during strict execution, recovering...");
                        poisoned.into_inner()
                    }
                };

                if persist_objects {
                    let runtime = &self.runtime_pool[0];
                    runtime.persist_created_objects(&changeset);
                    runtime.persist_deleted_objects(&changeset);
                }

                state_write
                    .apply_changeset(&changeset)
                    .map_err(|e| anyhow::anyhow!("Failed to apply changeset: {}", e))?;
                executed_count += 1;
            }

            return Ok((executed_count, failed_count));
        }

        let waves = kanari_move_runtime_v1::TransactionScheduler::schedule(transactions);

        if has_module_publish {
            // Keep speculative publish execution deterministic across authorities.
            // PublishModule depends on VM/module cache state more heavily than regular
            // user transactions, so we reset the shared cache and execute on one
            // runtime in a fixed serial order for DAG production / validation.
            self.runtime_pool[0].reload_vm_cache()?;
        }

        for wave in waves {
            let results: Vec<Result<ChangeSet>> = if has_module_publish {
                wave.iter()
                    .map(|signed_tx| {
                        self.execute_transaction_with_runtime_internal(
                            &signed_tx.transaction,
                            &self.runtime_pool[0],
                            state_arc,
                            false,
                            timestamp,
                            persist_objects,
                        )
                    })
                    .collect()
            } else {
                wave.par_iter()
                    .enumerate()
                    .map(|(i, signed_tx)| {
                        let runtime = &self.runtime_pool[i % self.runtime_pool.len()];
                        self.execute_transaction_with_runtime_internal(
                            &signed_tx.transaction,
                            runtime,
                            state_arc,
                            false,
                            timestamp,
                            persist_objects,
                        )
                    })
                    .collect()
            };

            // Apply changesets with proper error handling to prevent node crashes
            let mut state_write = match state_arc.write() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    log::error!("State lock poisoned during wave execution, recovering...");
                    poisoned.into_inner()
                }
            };

            for res in results {
                match res {
                    Ok(cs) => {
                        if persist_objects {
                            let runtime = &self.runtime_pool[0];
                            runtime.persist_created_objects(&cs);
                            runtime.persist_deleted_objects(&cs);
                        }

                        if let Err(e) = state_write.apply_changeset(&cs) {
                            if strict_mode {
                                anyhow::bail!("Failed to apply changeset: {}", e);
                            }
                            log::warn!("apply_changeset failed: {}", e);
                            failed_count += 1;
                        } else {
                            executed_count += 1;
                        }
                    }
                    Err(e) => {
                        if strict_mode {
                            anyhow::bail!("Execution failed: {}", e);
                        }
                        log::warn!("Parallel execution failed: {}", e);
                        failed_count += 1;
                    }
                }
            }
        }

        Ok((executed_count, failed_count))
    }

    pub(crate) fn checkpoint_root_matches(
        &self,
        checkpoint_sequence: u64,
        computed_root: &[u8],
        checkpoint_root: &[u8],
    ) -> Result<bool> {
        if computed_root == checkpoint_root {
            return Ok(true);
        }

        if Self::strict_checkpoint_roots_required() {
            anyhow::bail!(
                "[ENGINE] Strict checkpoint root verification failed for checkpoint {}",
                checkpoint_sequence
            );
        }

        Ok(false)
    }

    fn apply_gas_and_sequence(
        changeset: &mut ChangeSet,
        sender: AccountAddress,
        gas_cost: u64,
        gas_used: u64,
    ) -> Result<()> {
        let sender_change = changeset.get_or_create_change(sender);
        sender_change.increment_sequence();
        sender_change.debit(gas_cost);

        let dao_addr = AccountAddress::from_hex_literal(KanariAddress::DAO_ADDRESS)?;
        changeset.collect_gas(dao_addr, gas_cost);
        changeset.set_gas_used(gas_used);
        Ok(())
    }

    pub fn persist_dag_state(&self, state: PersistentDagState) -> Result<()> {
        if let Some(store) = &self.persistent_store {
            store
                .save(b"dag_state", &state)
                .context("Failed to persist DAG state")?;
        }
        Ok(())
    }

    // =====================================================================
    // 🕸️ DAG Consensus Switch
    // =====================================================================
    pub fn process_dag_checkpoint(
        &self,
        checkpoint_txs: Vec<SignedTransaction>,
        consensus_timestamp_ms: Option<u64>, // Allow timestamp from consensus layer
    ) -> Result<Vec<u8>> {
        log::info!(
            "[DAG CONSENSUS] Applying new checkpoint with {} transactions",
            checkpoint_txs.len()
        );

        info!(
            "Executing {} transactions in checkpoint",
            checkpoint_txs.len()
        );

        // =================================================================

        // 🚨 Update the time on the Blockchain before executing user transactions.
        // =================================================================
        // Use timestamp from consensus layer to ensure all nodes have identical state
        // CRITICAL: All nodes must use the same timestamp to ensure deterministic state transitions
        let current_timestamp_ms = match consensus_timestamp_ms {
            Some(ts) => ts,
            None => {
                error!(
                    "CRITICAL ERROR: Consensus timestamp must be provided for blockchain state consistency."
                );
                return Err(anyhow::anyhow!("Missing consensus timestamp"));
            }
        };

        if let Err(e) = self.execute_system_prologue(current_timestamp_ms) {
            log::error!(
                "Critical Error: System clock failed to update! Halt execution. {:?}",
                e
            );
            return Err(e);
        }
        // =================================================================

        let execution_results = self.execute_transactions_parallel(checkpoint_txs);

        // =================================================================
        // 📝 Process all the transaction results and create a checkpoint
        // =================================================================
        let mut successful_txs = Vec::new();
        let mut all_events_for_block = Vec::new();
        let runtime = &self.runtime_pool[0];

        for (tx, result) in execution_results {
            match result {
                Ok((_tx_hash, changeset)) => {
                    runtime.persist_created_objects(&changeset);
                    runtime.persist_deleted_objects(&changeset);

                    let mut state = match self.state.write() {
                        Ok(guard) => guard,
                        Err(poisoned) => {
                            error!("State lock poisoned during DAG commit, recovering...");
                            poisoned.into_inner()
                        }
                    };
                    if let Err(e) = state.apply_changeset(&changeset) {
                        error!("[DAG COMMIT] Failed to apply changeset to state: {}", e);
                    }

                    all_events_for_block.extend(changeset.events.clone());
                    successful_txs.push(tx);
                }
                Err(e) => {
                    log::warn!("[DAG COMMIT] Transaction execution failed: {}", e);
                }
            }
        }

        let mut chain = match self.blockchain.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("Blockchain lock poisoned during commit, recovering...");
                poisoned.into_inner()
            }
        };
        let height = chain.blocks.len() as u64;
        let prev_hash = chain.blocks.back().map(|b| b.hash()).unwrap_or_default();

        let new_block = kanari_types::block::Block::new(
            height,
            prev_hash,
            vec![0u8; 32],
            successful_txs,
            all_events_for_block,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|e| {
                    error!("System clock error: {}. Using timestamp 0.", e);
                    std::time::Duration::from_secs(0)
                })
                .as_secs(),
        );
        let block_hash = new_block.hash();
        chain.blocks.push_back(new_block);

        log::info!(
            "[DAG CONSENSUS] Checkpoint {} committed! Hash: {}",
            height,
            hex::encode(&block_hash)
        );

        Ok(block_hash)
    }

    fn execute_transaction_with_runtime(
        &self,
        tx: &Transaction,
        runtime: &kanari_move_runtime_v1::move_runtime::MoveRuntime,
        state_arc: &Arc<RwLock<StateManager>>,
        timestamp: Option<u64>,
    ) -> Result<ChangeSet> {
        self.execute_transaction_with_runtime_internal(
            tx, runtime, state_arc, true, timestamp, false,
        )
    }

    pub(crate) fn execute_transaction_with_runtime_internal(
        &self,
        tx: &Transaction,
        runtime: &kanari_move_runtime_v1::move_runtime::MoveRuntime,
        state_arc: &Arc<RwLock<StateManager>>,
        validate_sequence: bool,
        timestamp: Option<u64>,
        persist_runtime_state: bool,
    ) -> Result<ChangeSet> {
        let sender_addr = KanariAddress::parse_to_account_address(tx.sender_address())?;
        let mut gas_meter = GasMeter::new(tx.gas_limit(), tx.gas_price());
        let mut changeset = ChangeSet::new();

        let (gas_op, required_amount) = match tx {
            Transaction::PublishModule { module_bytes, .. } => (
                GasOperation::PublishModule {
                    module_size: module_bytes.len(),
                },
                0,
            ),
            Transaction::ExecuteFunction { .. } => {
                (GasOperation::ExecuteFunction { complexity: 1 }, 0)
            }
            Transaction::Transfer { amount, .. } | Transaction::Burn { amount, .. } => {
                (GasOperation::Transfer, *amount)
            }
        };

        gas_meter.consume(gas_op.gas_units())?;
        let gas_cost = gas_meter.total_cost();
        let total_required = required_amount.saturating_add(gas_cost);

        {
            let state = match state_arc.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    log::error!("State arc lock poisoned in pre-execution checks, recovering...");
                    poisoned.into_inner()
                }
            };
            if validate_sequence {
                state
                    .validate_sequence(&sender_addr, tx.sequence_number())
                    .context("Sequence number validation failed")?;
            }
            let balance = state
                .get_account(&sender_addr)
                .map(|acc| acc.native_balance())
                .unwrap_or(0);
            if balance < total_required {
                let msg = if required_amount > 0 {
                    format!(
                        "Insufficient balance: need {} (amount: {}, gas: {}) but have {}",
                        total_required, required_amount, gas_cost, balance
                    )
                } else {
                    format!(
                        "Insufficient balance for gas: need {}, have {}",
                        gas_cost, balance
                    )
                };
                changeset.mark_failed(msg);
                Self::apply_gas_and_sequence(
                    &mut changeset,
                    sender_addr,
                    gas_cost,
                    gas_meter.gas_used,
                )?;
                return Ok(changeset);
            }
        }

        match tx {
            Transaction::PublishModule {
                sender,
                module_bytes,
                ..
            } => {
                match runtime.publish_module_with_persistence(
                    module_bytes.clone(),
                    KanariAddress::parse_to_account_address(sender)?,
                    Some((tx.gas_limit(), tx.gas_price())),
                    persist_runtime_state,
                ) {
                    Ok(move_cs) => changeset.merge(move_cs),
                    Err(e) => {
                        changeset.mark_failed(format!("Publish failed: {}", e));
                    }
                }
            }

            Transaction::ExecuteFunction {
                module,
                function,
                type_args,
                args,
                ..
            } => {
                let parts: Vec<&str> = module.split("::").collect();
                if parts.len() != 2 {
                    changeset.mark_failed(
                        "Invalid module format. Expected: address::module".to_string(),
                    );
                    changeset.set_gas_used(0);
                    return Ok(changeset);
                }

                let addr = KanariAddress::parse_to_account_address(parts[0])?;
                let module_id = ModuleId::new(
                    addr,
                    move_core_types::identifier::Identifier::new(parts[1])?,
                );

                let type_tags: Vec<move_core_types::language_storage::TypeTag> = type_args
                    .iter()
                    .map(|s| {
                        parse_type_tag(s.as_str())
                            .ok_or_else(|| anyhow::anyhow!("Invalid type argument: {}", s))
                    })
                    .collect::<Result<Vec<_>>>()?;

                match runtime.execute_entry_function_with_tx_hash_and_persistence(
                    &module_id,
                    function,
                    type_tags,
                    args.clone(),
                    Some(sender_addr),
                    Some((tx.gas_limit(), tx.gas_price())),
                    timestamp,
                    Some(tx.hash()),
                    persist_runtime_state,
                ) {
                    Ok(move_cs) => changeset.merge(move_cs),
                    Err(e) => {
                        changeset.mark_failed(format!("Execution failed: {}", e));
                    }
                }
            }

            Transaction::Transfer { to, amount, .. } => {
                let to_addr = KanariAddress::parse_to_account_address(to)?;
                changeset.transfer(sender_addr, to_addr, *amount);
            }

            Transaction::Burn { amount, .. } => {
                changeset.burn(sender_addr, *amount);
            }
        }

        Self::apply_gas_and_sequence(&mut changeset, sender_addr, gas_cost, gas_meter.gas_used)?;
        Ok(changeset)
    }

    fn dag_engine_instance(&self) -> Result<DagEngine> {
        let mut dag_engine_guard = match self.dag_engine.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::error!("DAG engine lock poisoned while initializing, recovering...");
                poisoned.into_inner()
            }
        };
        if dag_engine_guard.is_none() {
            {
                let mut chain = match self.blockchain.write() {
                    Ok(guard) => guard,
                    Err(poisoned) => {
                        log::error!("Blockchain lock poisoned during DAG init, recovering...");
                        poisoned.into_inner()
                    }
                };
                if !chain.dag_mode {
                    chain.enable_dag_mode();
                }
            }

            let engine = DagEngine::new(
                Arc::new(self.clone_for_dag()),
                self.authority_id.clone(),
                self.authorities.clone(),
            )?;
            *dag_engine_guard = Some(engine);
        }

        dag_engine_guard
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Failed to initialize DAG engine"))
    }

    pub fn produce_block(&self) -> Result<BlockInfo> {
        let dag_engine = self.dag_engine_instance()?;

        {
            let consensus_lock = dag_engine.consensus();
            let consensus = match consensus_lock.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    log::error!("Consensus lock poisoned in produce_block, recovering...");
                    poisoned.into_inner()
                }
            };
            let policy = consensus.production_policy();

            if policy.should_wait_for_current_round_quorum() {
                anyhow::bail!(
                    "SYNC_WAITING: have {}/{} vertices in round {} (need quorum for round {})",
                    policy.parent_author_count,
                    policy.quorum_size,
                    policy.current_round,
                    policy.current_round + 1
                );
            }
        }

        dag_engine.produce_vertex()
    }

    pub fn latest_own_dag_vertices(
        &self,
        limit: usize,
    ) -> Result<Vec<centauri::consensus::DagVertex>> {
        Ok(self.dag_engine_instance()?.latest_own_vertices(limit))
    }

    pub fn add_network_dag_vertex(&self, vertex: centauri::consensus::DagVertex) -> Result<bool> {
        let previous_height = self.get_stats().height;
        self.dag_engine_instance()?.add_network_vertex(vertex)?;
        Ok(self.get_stats().height > previous_height)
    }

    pub fn should_produce_dag_progress(&self) -> bool {
        let dag_engine_guard = match self.dag_engine.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::error!(
                    "DAG engine lock poisoned in should_produce_dag_progress, recovering..."
                );
                poisoned.into_inner()
            }
        };

        dag_engine_guard
            .as_ref()
            .is_some_and(|dag_engine| dag_engine.needs_progress())
    }

    fn clone_for_dag(&self) -> BlockchainEngine {
        BlockchainEngine {
            blockchain: self.blockchain.clone(),
            state: self.state.clone(),
            pending_txs: self.pending_txs.clone(),
            pending_tx_hashes: self.pending_tx_hashes.clone(),
            persistent_store: self.persistent_store.clone(),
            runtime_pool: self.runtime_pool.clone(),
            proof_cache: self.proof_cache.clone(),
            dag_engine: Arc::new(RwLock::new(None)),
            authority_id: self.authority_id.clone(),
            authorities: self.authorities.clone(),
            persisted_dag_state: self.persisted_dag_state.clone(),
        }
    }

    pub fn set_authorities(&mut self, authority_id: String, authorities: Vec<String>) {
        fn normalize(s: String) -> String {
            if s.starts_with("0x") {
                s
            } else {
                format!("0x{}", s)
            }
        }
        self.authority_id = normalize(authority_id);
        self.authorities = authorities.into_iter().map(normalize).collect();
        match self.dag_engine.write() {
            Ok(mut guard) => *guard = None,
            Err(poisoned) => {
                log::error!("DAG engine lock poisoned in set_authorities, recovering...");
                *poisoned.into_inner() = None;
            }
        }
    }

    pub fn get_authority_id(&self) -> String {
        self.authority_id.clone()
    }

    pub fn get_authorities(&self) -> Vec<String> {
        self.authorities.clone()
    }

    pub fn should_defer_user_execution_to_consensus(&self) -> bool {
        self.authorities.len() > 1
    }

    pub fn get_dag_engine(&self) -> Option<Arc<RwLock<Option<DagEngine>>>> {
        Some(self.dag_engine.clone())
    }

    pub fn export_consensus_metrics_prometheus(&self) -> Result<String> {
        let dag_engine_guard = match self.dag_engine.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::error!("DAG engine lock poisoned in export_consensus_metrics_prometheus");
                poisoned.into_inner()
            }
        };

        if let Some(dag_engine) = dag_engine_guard.as_ref() {
            let consensus_lock = dag_engine.consensus();
            let consensus = match consensus_lock.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    log::error!("Consensus lock poisoned in metrics export");
                    poisoned.into_inner()
                }
            };
            return consensus.metrics().export_prometheus();
        }

        DagMetrics::default().export_prometheus()
    }
}

#[cfg(test)]
mod tests {
    use super::BlockchainEngine;
    use kanari_crypto::keys::{CurveType, generate_keypair};
    use kanari_types::transaction::{SignedTransaction, Transaction};
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn signed_transfer_from(
        sender: &kanari_crypto::keys::KeyPair,
        sequence_number: u64,
    ) -> SignedTransaction {
        let recipient = generate_keypair(CurveType::Ed25519).unwrap();
        let tx = Transaction::new_transfer(
            sender.tagged_address(),
            recipient.address,
            1,
            sequence_number,
        );
        let mut signed_tx = SignedTransaction::new(tx);
        signed_tx
            .sign(&sender.private_key, sender.curve_type)
            .unwrap();
        signed_tx
    }

    #[test]
    fn mainnet_defaults_enable_strict_runtime_guards() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        unsafe {
            std::env::set_var("KANARI_NETWORK", "mainnet");
            std::env::remove_var("KANARI_REQUIRE_PERSISTENT_STORAGE");
            std::env::remove_var("KANARI_STRICT_CHECKPOINT_ROOTS");
        }

        assert!(BlockchainEngine::strict_persistence_required());
        assert!(BlockchainEngine::strict_checkpoint_roots_required());

        unsafe {
            std::env::remove_var("KANARI_NETWORK");
        }
    }

    #[test]
    fn explicit_env_overrides_strict_runtime_guards() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        unsafe {
            std::env::set_var("KANARI_NETWORK", "mainnet");
            std::env::set_var("KANARI_REQUIRE_PERSISTENT_STORAGE", "false");
            std::env::set_var("KANARI_STRICT_CHECKPOINT_ROOTS", "0");
        }

        assert!(!BlockchainEngine::strict_persistence_required());
        assert!(!BlockchainEngine::strict_checkpoint_roots_required());

        unsafe {
            std::env::remove_var("KANARI_NETWORK");
            std::env::remove_var("KANARI_REQUIRE_PERSISTENT_STORAGE");
            std::env::remove_var("KANARI_STRICT_CHECKPOINT_ROOTS");
        }
    }

    #[test]
    fn single_authority_engine_allows_immediate_execution() {
        let engine = BlockchainEngine::new().unwrap();
        assert!(!engine.should_defer_user_execution_to_consensus());
    }

    #[test]
    fn multi_authority_engine_defers_execution_to_consensus() {
        let mut engine = BlockchainEngine::new().unwrap();
        engine.set_authorities(
            "0x1".to_string(),
            vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()],
        );
        assert!(engine.should_defer_user_execution_to_consensus());
    }

    #[test]
    fn batch_submit_accepts_contiguous_sequences_for_same_sender() {
        let engine = BlockchainEngine::new().unwrap();
        let sender = generate_keypair(CurveType::Ed25519).unwrap();
        let tx0 = signed_transfer_from(&sender, 0);
        let tx1 = signed_transfer_from(&sender, 1);

        let hashes = engine.submit_transactions_batch(vec![tx0, tx1]).unwrap();

        assert_eq!(hashes.len(), 2);
        assert_eq!(
            engine
                .pending_txs
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
            2
        );
    }

    #[test]
    fn batch_submit_rejects_duplicate_transactions() {
        let engine = BlockchainEngine::new().unwrap();
        let sender = generate_keypair(CurveType::Ed25519).unwrap();
        let tx = signed_transfer_from(&sender, 0);

        let err = engine
            .submit_transactions_batch(vec![tx.clone(), tx])
            .unwrap_err();

        assert!(err.to_string().contains("already in pending pool"));
    }

    #[test]
    fn batch_submit_rejects_transaction_already_indexed_in_pending_pool() {
        let engine = BlockchainEngine::new().unwrap();
        let sender = generate_keypair(CurveType::Ed25519).unwrap();
        let tx = signed_transfer_from(&sender, 0);

        engine.submit_transactions_batch(vec![tx.clone()]).unwrap();
        let err = engine.submit_transactions_batch(vec![tx]).unwrap_err();

        assert!(err.to_string().contains("already in pending pool"));
    }

    #[test]
    fn batch_submit_rejects_sequence_gaps() {
        let engine = BlockchainEngine::new().unwrap();
        let sender = generate_keypair(CurveType::Ed25519).unwrap();
        let tx = signed_transfer_from(&sender, 1);

        let err = engine.submit_transactions_batch(vec![tx]).unwrap_err();

        assert!(err.to_string().contains("Sequence number too high"));
    }
}
