// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use centauri::blockchain::Blockchain;
use centauri::consensus::{Checkpoint, PersistentDagState};
use kanari_move_runtime::changeset::ChangeSet;
use kanari_move_runtime::gas::{GasMeter, GasOperation};
use kanari_move_runtime::move_runtime::MoveRuntime;
use kanari_move_runtime::state::StateManager;
use kanari_move_runtime::storage::persistent_store::PersistentStore;
pub use kanari_rpc_api::{AccountInfo, BlockData, BlockchainStats, FullBlockData, ObjectInfo};
use kanari_types::address::Address as KanariAddress;
use kanari_types::event::Event;
use kanari_types::transaction::{SignedTransaction, Transaction};
use log::{debug, error, info, warn};
use lru::LruCache;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use num_cpus;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

type ProofCache = LruCache<(u64, usize), (String, Vec<Vec<u8>>)>;

mod apply_checkpoint;
mod produce_dag_vertex;
pub use produce_dag_vertex::{CheckpointInfo, DagBlockInfo, DagEngine};

pub type BlockInfo = DagBlockInfo;

/// Complete blockchain engine with Move VM integration
pub struct BlockchainEngine {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub state: Arc<RwLock<StateManager>>,
    pub pending_txs: Arc<RwLock<Vec<SignedTransaction>>>,
    pub persistent_store: Option<Arc<PersistentStore>>,
    // Reusable pool of MoveRuntime instances for parallel execution
    pub runtime_pool: Vec<kanari_move_runtime::move_runtime::MoveRuntime>,
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
// Supports: bool, u8, u64, u128, address, vector<T>, and struct tags like
// "0x1::Module::Name" or "0x1::Module::Name<address, u64, vector<u8>>".
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

    // Attempt to parse struct: address::Module::Name or address::Module::Name<...>
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

#[cfg(test)]
mod parse_type_tag_tests {
    use super::parse_type_tag;

    #[test]
    fn parse_type_tag_rejects_unclosed_generic() {
        assert!(parse_type_tag("0x1::m::T<").is_none());
        assert!(parse_type_tag("0x1::m::T<u64").is_none());
    }
}

impl BlockchainEngine {
    pub fn new_dir(dir: &str) -> Result<Self> {
        let persistent_store = Self::try_open_store(
            || PersistentStore::open_with_path(Some(std::path::PathBuf::from(dir))),
            &format!("at '{}'", dir),
        );
        Self::init(persistent_store)
    }

    pub fn new() -> Result<Self> {
        let persistent_store = Self::try_open_store(PersistentStore::open_default, "default");
        Self::init(persistent_store)
    }

    fn try_open_store<F>(opener: F, context: &str) -> Option<Arc<PersistentStore>>
    where
        F: FnOnce() -> Result<PersistentStore>,
    {
        if cfg!(miri) {
            None
        } else {
            match opener() {
                Ok(s) => Some(Arc::new(s)),
                Err(e) => {
                    eprintln!(
                        "WARN: Failed to open {} persistent store: {}. Falling back to in-memory mode.",
                        context, e
                    );
                    None
                }
            }
        }
    }

    fn init(persistent_store: Option<Arc<PersistentStore>>) -> Result<Self> {
        let blockchain = Self::load_blockchain(&persistent_store);
        let state = Self::load_state(&persistent_store);

        // Initialize runtime pool (mandatory: one runtime per CPU)
        // OPTIMIZATION: Use multiple independent MoveVM instances to avoid lock contention.
        // We use `spawn_worker` to create new VMs that share the underlying RocksDB/State
        // so they can read modules/objects but don't block each other on VM locks.
        let workers = num_cpus::get().max(1);
        let mut runtime_pool = Vec::new();

        // Create base runtime:
        // - persistent mode -> runtime backed by default persistent store
        // - fallback mode   -> fully in-memory runtime
        let base_runtime = match if persistent_store.is_some() {
            MoveRuntime::new_with_kanari_natives()
        } else {
            MoveRuntime::new_with_kanari_natives_in_memory()
        } {
            Ok(rt) => rt,
            Err(e) => {
                log::error!("FATAL: Failed to initialize base MoveRuntime: {}", e);
                anyhow::bail!("Failed to initialize runtime pool: {}", e);
            }
        };

        log::info!(
            "Initializing runtime pool with {} workers (independent VMs sharing DB)",
            workers
        );

        // Add base runtime as first worker
        runtime_pool.push(base_runtime.clone());

        // Spawn remaining workers sharing state with base runtime
        for i in 1..workers {
            match base_runtime.spawn_worker() {
                Ok(rt) => runtime_pool.push(rt),
                Err(e) => {
                    log::error!("Failed to spawn worker runtime #{}: {}", i, e);
                    anyhow::bail!("Failed to initialize runtime pool: {}", e);
                }
            }
        }

        // Note: treasuries are loaded above when StateManager is initialized
        let pending_txs = Arc::new(RwLock::new(Vec::new()));

        // Initialize LRU cache for merkle proofs (cache up to 1000 proofs)
        let proof_cache = Arc::new(RwLock::new(LruCache::new(
            NonZeroUsize::new(1000).expect("NonZeroUsize::new(1000) should never fail"),
        )));

        // Setup default authorities for DAG mode (single node by default)
        let authority_id = "0xDEFAULT_AUTHORITY".to_string();
        let authorities = vec![authority_id.clone()];

        let persisted_dag_state = Self::load_dag_state(&persistent_store);

        Ok(Self {
            blockchain,
            state,
            pending_txs,
            persistent_store,
            runtime_pool,
            proof_cache,
            dag_engine: Arc::new(RwLock::new(None)),
            authority_id,
            authorities,
            persisted_dag_state,
        })
    }

    fn load_blockchain(store: &Option<Arc<PersistentStore>>) -> Arc<RwLock<Blockchain>> {
        if let Some(store) = store {
            match store.load::<Blockchain>(b"blockchain") {
                Ok(Some(mut b)) => {
                    info!(
                        "Successfully loaded blockchain from persistent store (height: {}, checkpoints: {})",
                        b.height(),
                        b.dag_checkpoints.len()
                    );
                    // Rebuild transaction hash index after loading from disk
                    b.rebuild_tx_hash_index();
                    Arc::new(RwLock::new(b))
                }
                Ok(None) => {
                    info!(
                        "No persisted blockchain found at the provided store. Creating fresh genesis."
                    );
                    Arc::new(RwLock::new(Blockchain::new()))
                }
                Err(e) => {
                    error!(
                        "FATAL ERROR loading blockchain from persistent store: {}. Falling back to fresh genesis to prevent crash, but state may be inconsistent!",
                        e
                    );
                    Arc::new(RwLock::new(Blockchain::new()))
                }
            }
        } else {
            info!("Running in-memory mode: No persistent store provided for blockchain.");
            Arc::new(RwLock::new(Blockchain::new()))
        }
    }

    fn load_state(store: &Option<Arc<PersistentStore>>) -> Arc<RwLock<StateManager>> {
        let store = store
            .clone()
            .unwrap_or_else(|| Arc::new(PersistentStore::open_in_memory().unwrap()));

        info!("Initializing StateManager with persistent store support (RocksDB)");
        Arc::new(RwLock::new(StateManager::new(store)))
    }

    fn load_dag_state(store: &Option<Arc<PersistentStore>>) -> Option<PersistentDagState> {
        if let Some(store) = store {
            match store.load::<PersistentDagState>(b"dag_state") {
                Ok(Some(s)) => {
                    info!("Successfully loaded DAG consensus state from persistent store");
                    Some(s)
                }
                Ok(None) => {
                    info!("No persisted DAG state found.");
                    None
                }
                Err(e) => {
                    error!(
                        "Failed to load DAG state from persistent store: {}. Falling back to fresh DAG.",
                        e
                    );
                    None
                }
            }
        } else {
            None
        }
    }

    /// Apply gas deduction and increment sequence on a ChangeSet (static helper).
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

    /// Persist DAG consensus state
    pub fn persist_dag_state(&self, state: PersistentDagState) -> Result<()> {
        if let Some(store) = &self.persistent_store {
            info!("[ENGINE] Persisting DAG consensus state...");
            store
                .save(b"dag_state", &state)
                .context("Failed to persist DAG state")?;
            info!("[ENGINE] DAG state persisted successfully");
        }
        Ok(())
    }

    /// Add signed transaction to pending pool after verifying signature
    pub fn submit_transaction(&self, signed_tx: SignedTransaction) -> Result<Vec<u8>> {
        // 1. Verify signature
        if !signed_tx.verify_signature()? {
            anyhow::bail!("Invalid or missing transaction signature");
        }

        let tx_hash = signed_tx.hash();
        let tx_hash_hex = hex::encode(&tx_hash);
        let sender_address = signed_tx.transaction.sender_address();

        // 1b. Validate sequence number (committed + pending)
        {
            let state = self.state.read().unwrap();
            let mut expected_seq = 0;
            if let Some(acc) = state.get_account_by_hex(sender_address) {
                expected_seq = acc.sequence_number;
            }

            // Add pending count
            self.for_each_pending_tx_from_sender(sender_address, |_| expected_seq += 1);

            let tx_seq = signed_tx.transaction.sequence_number();
            if tx_seq < expected_seq {
                anyhow::bail!(
                    "Sequence number too low: expected {}, got {}",
                    expected_seq,
                    tx_seq
                );
            }
            if tx_seq > expected_seq {
                anyhow::bail!(
                    "Sequence number too high: expected {}, got {} (out-of-order execution not supported yet)",
                    expected_seq,
                    tx_seq
                );
            }
        }

        // 2. Check if already executed in blockchain
        {
            let chain = self.blockchain.read().unwrap();
            if chain.is_transaction_executed(&tx_hash_hex) {
                anyhow::bail!("Transaction {} already executed", tx_hash_hex);
            }
        }

        // 3. Check if already in pending pool
        let mut pending = self.pending_txs.write().unwrap();
        for ptx in pending.iter() {
            if ptx.hash() == tx_hash {
                anyhow::bail!("Transaction {} already in pending pool", tx_hash_hex);
            }
        }

        pending.push(signed_tx);
        info!("[ENGINE] Transaction {} added to mempool", tx_hash_hex);
        Ok(tx_hash)
    }

    /// Execute transaction immediately and return both hash and changeset
    /// Used by RPC to get object IDs created during execution
    ///
    /// In DAG mode: This will execute AND apply the changeset to state immediately,
    /// bypassing DAG consensus. This is necessary for RPC calls/publishes to work.
    pub fn execute_transaction_immediate(
        &self,
        signed_tx: SignedTransaction,
    ) -> Result<(Vec<u8>, ChangeSet)> {
        // Verify signature
        if !signed_tx.verify_signature()? {
            anyhow::bail!("Invalid transaction signature");
        }

        let tx_hash = signed_tx.hash();
        let tx = signed_tx.transaction;

        // For immediate (RPC) execution, whether in DAG mode or not, we run
        // the Move execution against a cloned snapshot of the current StateManager.
        // This makes the call a read-only/simulated run: it returns the
        // ChangeSet that would be applied, but it does NOT mutate the engine's
        // canonical `state`. This is crucial because the same transaction is
        // also submitted for consensus. Modifying the state here would cause
        // validation to fail during consensus, leading to state divergence.
        let changeset = {
            // Clone the current state for a safe simulation
            let mut state_snapshot = { self.state.read().unwrap().clone() };

            // Adjust the cloned snapshot to account for any pending transactions
            // from the same sender that haven't been committed yet.
            // This is needed for correct sequence number validation.
            let sender_addr = tx.sender_address();
            let addr = KanariAddress::parse_to_account_address(sender_addr)?;

            self.for_each_pending_tx_from_sender(sender_addr, |_| {
                if let Some(mut acct) = state_snapshot.get_account(&addr) {
                    acct.increment_sequence();
                    state_snapshot.save_account(&acct).unwrap();
                }
            });
            let state_arc = Arc::new(RwLock::new(state_snapshot));

            // Use a runtime from the pool for execution
            let runtime = &self.runtime_pool[0];
            self.execute_transaction_with_runtime(&tx, runtime, &state_arc, None)?
        };

        Ok((tx_hash, changeset))
    }

    /// Execute a single transaction and return ChangeSet
    /// This is the correct way: Move VM produces ChangeSet, StateManager applies it
    /// Execute a transaction using a provided `runtime` and `state_arc`.
    /// This is a static helper so worker threads can call it without borrowing `self`.
    fn execute_transaction_with_runtime(
        &self,
        tx: &Transaction,
        runtime: &kanari_move_runtime::move_runtime::MoveRuntime,
        state_arc: &Arc<RwLock<StateManager>>,
        timestamp: Option<u64>,
    ) -> Result<ChangeSet> {
        self.execute_transaction_with_runtime_internal(
            tx, runtime, state_arc, true, timestamp, false,
        )
    }

    /// Execute a transaction with option to skip sequence validation
    /// Used for syncing blocks where sequence is already validated by the original node
    pub(crate) fn execute_transaction_with_runtime_skip_seq(
        &self,
        tx: &Transaction,
        runtime: &kanari_move_runtime::move_runtime::MoveRuntime,
        state_arc: &Arc<RwLock<StateManager>>,
        timestamp: Option<u64>,
    ) -> Result<ChangeSet> {
        self.execute_transaction_with_runtime_internal(
            tx, runtime, state_arc, false, timestamp, false,
        )
    }

    pub(crate) fn execute_transaction_with_runtime_skip_seq_persist(
        &self,
        tx: &Transaction,
        runtime: &kanari_move_runtime::move_runtime::MoveRuntime,
        state_arc: &Arc<RwLock<StateManager>>,
        timestamp: Option<u64>,
    ) -> Result<ChangeSet> {
        self.execute_transaction_with_runtime_internal(
            tx, runtime, state_arc, false, timestamp, true,
        )
    }

    pub(crate) fn execute_transaction_with_runtime_internal(
        &self,
        tx: &Transaction,
        runtime: &kanari_move_runtime::move_runtime::MoveRuntime,
        state_arc: &Arc<RwLock<StateManager>>,
        validate_sequence: bool,
        timestamp: Option<u64>,
        persist_runtime_state: bool,
    ) -> Result<ChangeSet> {
        // 1. Pre-flight validation: Check sequence number (skip for synced transactions)
        let sender_addr = KanariAddress::parse_to_account_address(tx.sender_address())?;
        if validate_sequence {
            let state = state_arc.read().unwrap();
            state
                .validate_sequence(&sender_addr, tx.sequence_number())
                .context("Sequence number validation failed")?;
        }

        // 2. Calculate gas and check balance
        let mut gas_meter = GasMeter::new(tx.gas_limit(), tx.gas_price());
        let mut changeset = ChangeSet::new();

        // Determine gas operation and required balance (amount + gas)
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

        // Balance check
        {
            let state = state_arc.read().unwrap();
            let balance = state
                .get_account(&sender_addr)
                .map(|acc| acc.balance)
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

        // 3. Execute transaction logic
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
                // Synchronize object snapshots for object-id arguments (32-byte IDs)
                // from canonical state into runtime object storage to avoid stale reads.
                {
                    let state = state_arc.read().unwrap();
                    for arg in args.iter() {
                        if arg.len() != 32 {
                            continue;
                        }
                        let object_id = format!("0x{}", hex::encode(arg));
                        if let Ok(Some(obj)) = state.get_object(&object_id) {
                            // Best-effort sync; VM/state logic still validates semantics.
                            let _ = runtime.preload_object_snapshot(
                                &object_id,
                                obj.owner,
                                &obj.type_,
                                obj.data.clone(),
                                obj.version,
                            );
                        }
                    }
                }

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

        // 4. Apply gas and sequence
        Self::apply_gas_and_sequence(&mut changeset, sender_addr, gas_cost, gas_meter.gas_used)?;

        Ok(changeset)
    }

    /// Produce a new block with pending transactions using DAG-based consensus
    ///
    /// This method creates a DAG vertex and automatically commits checkpoints
    /// when consensus is reached. Uses parallel transaction execution.
    pub fn produce_block(&self) -> Result<BlockInfo> {
        // Lazy-initialize DAG engine on first use and enable DAG mode
        {
            let mut dag_engine_guard = self.dag_engine.write().unwrap();
            if dag_engine_guard.is_none() {
                // Enable DAG mode on blockchain before initializing DAG engine
                {
                    let mut chain = self.blockchain.write().unwrap();
                    if !chain.dag_mode {
                        chain.enable_dag_mode();
                    }
                }

                let dag_engine = DagEngine::new(
                    Arc::new(self.clone_for_dag()),
                    self.authority_id.clone(),
                    self.authorities.clone(),
                )?;
                *dag_engine_guard = Some(dag_engine);
            }
        }

        // Produce DAG vertex
        // We don't hold state/blockchain locks here because produce_vertex and
        // apply_checkpoint will acquire them in the correct order.
        let dag_engine = {
            let guard = self.dag_engine.read().unwrap();
            guard.as_ref().unwrap().clone()
        };
        dag_engine.produce_vertex()
    }

    /// Clone engine for DAG usage (internal helper)
    fn clone_for_dag(&self) -> BlockchainEngine {
        BlockchainEngine {
            blockchain: self.blockchain.clone(),
            state: self.state.clone(),
            pending_txs: self.pending_txs.clone(),
            persistent_store: self.persistent_store.clone(),
            runtime_pool: self.runtime_pool.clone(),
            proof_cache: self.proof_cache.clone(),
            dag_engine: Arc::new(RwLock::new(None)), // Don't clone DAG engine (prevent recursion)
            authority_id: self.authority_id.clone(),
            authorities: self.authorities.clone(),
            persisted_dag_state: self.persisted_dag_state.clone(),
        }
    }

    /// Configure authorities for DAG mode
    pub fn set_authorities(&mut self, authority_id: String, authorities: Vec<String>) {
        // Normalize authority IDs to ensure consistency (ensure 0x prefix)
        fn normalize(s: String) -> String {
            if s.starts_with("0x") {
                s
            } else {
                format!("0x{}", s)
            }
        }

        self.authority_id = normalize(authority_id);
        self.authorities = authorities.into_iter().map(normalize).collect();

        // Reset DAG engine to use new authorities
        *self.dag_engine.write().unwrap() = None;
    }

    /// Get current authority ID
    pub fn get_authority_id(&self) -> String {
        self.authority_id.clone()
    }

    /// Get list of authorities
    pub fn get_authorities(&self) -> Vec<String> {
        self.authorities.clone()
    }

    /// Get DAG engine if initialized (for vertex sync)
    pub fn get_dag_engine(&self) -> Option<Arc<RwLock<Option<DagEngine>>>> {
        Some(self.dag_engine.clone())
    }

    /// Get blockchain stats
    pub fn get_stats(&self) -> BlockchainStats {
        let state = self.state.read().unwrap();
        let chain = self.blockchain.read().unwrap();
        let pending = self.pending_txs.read().unwrap();

        BlockchainStats {
            height: chain.height(),
            total_blocks: chain.blocks.len(),
            total_transactions: chain.get_transaction_count(),
            pending_transactions: pending.len(),
            total_accounts: state.account_count(),
            total_supply: state.total_supply,
        }
    }

    /// Get account info
    pub fn get_account_info(&self, address: &str) -> Option<AccountInfo> {
        debug!("[ENGINE] get_account_info called for {}", address);
        let state = self.state.read().unwrap();
        state.get_account_by_hex(address).map(|acc| {
            debug!("[ENGINE] Found account {} in state", address);
            // collect owned object ids for this account and map to ObjectInfo
            let owned_ids = state.get_owned_objects(&acc.address).unwrap_or_default();

            let mut owned_objs: Vec<ObjectInfo> = Vec::new();
            for id in owned_ids {
                if let Ok(Some(obj)) = state.get_object(&id) {
                    owned_objs.push(ObjectInfo {
                        id: id.clone(),
                        owner: format!("{:#x}", obj.owner),
                        type_: obj.type_.clone(),
                        data: obj.data.clone(),
                        version: obj.version,
                    });
                }
            }

            // Rebuild spendable token balances from owned Coin<T> objects so RPC output
            // does not report stale non-spendable balances.
            let mut coin_balances: std::collections::BTreeMap<String, u64> =
                std::collections::BTreeMap::new();
            for obj in &owned_objs {
                if let Some(start) = obj.type_.find('<')
                    && let Some(end) = obj.type_.rfind('>')
                {
                    let outer = &obj.type_[..start];
                    if (outer.ends_with("::coin::Coin") || outer.ends_with("::coin::coin::Coin"))
                        && obj.data.len() >= 40
                    {
                        let token_type = obj.type_[start + 1..end].to_string();
                        let mut arr = [0u8; 8];
                        arr.copy_from_slice(&obj.data[32..40]);
                        let amount = u64::from_le_bytes(arr);
                        let entry = coin_balances.entry(token_type).or_insert(0);
                        *entry = entry.saturating_add(amount);
                    }
                }
            }

            let mut sequence_number = acc.sequence_number;

            // Add pending transactions count to sequence number
            self.for_each_pending_tx_from_sender(address, |_| sequence_number += 1);

            let info = AccountInfo {
                address: format!("{:#x}", acc.address),
                balance: acc.balance,
                sequence_number,
                modules: acc.modules.iter().cloned().collect(),
                token_balances: coin_balances,
                owned_objects: Some(owned_objs),
            };
            debug!("[ENGINE] get_account_info completed for {}", address);
            info
        })
    }

    /// Get token balance for specific token type
    pub fn get_token_balance(&self, address: &str, token_type: &str) -> u64 {
        let state = self.state.read().unwrap();
        state
            .get_account_by_hex(address)
            .map(|acc| acc.get_token_balance(token_type))
            .unwrap_or(0)
    }

    /// Get all token balances for an address
    pub fn get_all_token_balances(&self, address: &str) -> std::collections::BTreeMap<String, u64> {
        let state = self.state.read().unwrap();
        state
            .get_account_by_hex(address)
            .map(|acc| {
                acc.token_balances
                    .iter()
                    .map(|(k, v)| (k.clone(), v.value()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Iterate over pending transactions from a specific sender
    fn for_each_pending_tx_from_sender<F>(&self, sender: &str, mut f: F)
    where
        F: FnMut(&SignedTransaction),
    {
        if let Ok(pending) = self.pending_txs.read() {
            let normalized_sender = Self::normalize_addr(sender);
            for ptx in pending.iter() {
                if Self::normalize_addr(ptx.transaction.sender_address()) == normalized_sender {
                    f(ptx);
                }
            }
        }
    }

    /// Normalize address for comparison (converts to raw hex address)
    fn normalize_addr(addr: &str) -> String {
        use std::str::FromStr;
        // Use the central Address type to handle tagged addresses, public keys, and hex literals
        KanariAddress::from_str(addr)
            .map(|a| a.to_hex())
            .unwrap_or_else(|_| addr.trim_start_matches("0x").to_lowercase())
    }

    /// Get module bytecode from Move storage
    pub fn get_module_bytecode(&self, address: &str, module_name: &str) -> Option<Vec<u8>> {
        use move_core_types::{identifier::Identifier, language_storage::ModuleId};

        let addr = match KanariAddress::parse_to_account_address(address) {
            Ok(a) => a,
            Err(_) => return None,
        };

        let ident = match Identifier::new(module_name) {
            Ok(i) => i,
            Err(_) => return None,
        };

        let module_id = ModuleId::new(addr, ident);
        let runtime = &self.runtime_pool[0];
        runtime.get_module_bytes(&module_id)
    }

    /// List all published modules in Move storage
    pub fn list_all_modules(&self) -> Vec<(String, String)> {
        let runtime = &self.runtime_pool[0];
        runtime
            .list_modules()
            .into_iter()
            .map(|module_id| {
                (
                    format!("0x{}", module_id.address()),
                    module_id.name().to_string(),
                )
            })
            .collect()
    }

    /// Get block by height
    pub fn get_block(&self, height: u64) -> Option<BlockData> {
        let chain = self.blockchain.read().unwrap();
        chain.get_block(height).map(|block| BlockData {
            height: block.header.height,
            timestamp: block.header.timestamp,
            hash: hex::encode(block.hash()),
            prev_hash: hex::encode(&block.header.prev_hash),
            state_root: hex::encode(&block.header.state_root),
            tx_count: block.transactions.len(),
            events: block.events.clone(),
        })
    }

    /// Get full block with transactions by height
    pub fn get_full_block(&self, height: u64) -> Option<FullBlockData> {
        let chain = self.blockchain.read().unwrap();
        let block = chain.get_block(height)?;
        let checkpoint = chain.get_checkpoint(height);

        let vertices = checkpoint
            .map(|cp| cp.vertices.iter().map(hex::encode).collect())
            .unwrap_or_default();

        Some(FullBlockData {
            height: block.header.height,
            timestamp: block.header.timestamp,
            hash: hex::encode(block.hash()),
            prev_hash: hex::encode(&block.header.prev_hash),
            state_root: hex::encode(&block.header.state_root),
            tx_count: block.transactions.len(),
            events: block.events.clone(),
            transactions: block.transactions.clone(),
            vertices,
        })
    }

    /// Helper to decode hex string (with optional 0x)
    fn decode_hex(s: &str) -> Result<Vec<u8>> {
        hex::decode(s.trim_start_matches("0x")).context("Invalid hex string")
    }

    /// Helper to decode hex string to 32-byte array
    fn decode_hex_32(s: &str) -> [u8; 32] {
        let bytes = Self::decode_hex(s).unwrap_or_default();
        let mut arr = [0u8; 32];
        if bytes.len() == 32 {
            arr.copy_from_slice(&bytes);
        }
        arr
    }

    /// Sync full block with transactions from network data
    /// This method executes all transactions to rebuild the state
    pub fn sync_full_block_from_data(&self, block_data: &FullBlockData) -> Result<()> {
        let stats = self.get_stats();
        info!(
            "[SYNC] Attempting to sync block #{} (our height: {})",
            block_data.height, stats.height
        );

        // Check if we already have this block
        if block_data.height <= stats.height {
            info!("[SYNC] Already have block #{}, skipping", block_data.height);
            return Ok(()); // Already have it
        }

        // Verify this is the next block
        if block_data.height != stats.height + 1 {
            warn!(
                "[SYNC] Block #{} is not consecutive (need {})",
                block_data.height,
                stats.height + 1
            );
            anyhow::bail!(
                "Cannot sync block #{}: current height is {}",
                block_data.height,
                stats.height
            );
        }

        // Verify all transaction signatures before executing
        info!(
            "[SYNC] Verifying {} transaction signatures from block #{}",
            block_data.transactions.len(),
            block_data.height
        );
        for (i, signed_tx) in block_data.transactions.iter().enumerate() {
            let signed_tx: &SignedTransaction = signed_tx;
            if !signed_tx.verify_signature()? {
                anyhow::bail!(
                    "Invalid or missing signature for transaction {} in block #{}",
                    i + 1,
                    block_data.height,
                );
            }
        }

        // Create a checkpoint from block data
        let state_root = Self::decode_hex(&block_data.state_root)
            .context("Invalid state root format in block data")?;

        let prev_hash = {
            let chain = self.blockchain.read().unwrap();
            chain.latest_checkpoint().hash()
        };

        let vertices: Vec<[u8; 32]> = block_data
            .vertices
            .iter()
            .map(|v| Self::decode_hex_32(v))
            .collect();

        let checkpoint = Checkpoint::new(
            block_data.height,
            vertices,
            block_data.transactions.clone(),
            state_root,
            block_data.timestamp,
            prev_hash,
        );

        // Apply checkpoint using the unified atomic path
        self.apply_checkpoint(checkpoint)?;

        info!(
            "Synced block #{} with {} transactions",
            block_data.height,
            block_data.transactions.len()
        );

        Ok(())
    }

    /// Get state root for a specific block height or latest if None.
    pub fn get_state_root(&self, height: Option<u64>) -> Option<String> {
        let chain = self.blockchain.read().unwrap();
        let header = match height {
            Some(h) => &chain.get_block(h)?.header,
            None => &chain.latest_block().header,
        };
        Some(hex::encode(&header.state_root))
    }

    /// Return list of registered token types and their total supplies
    pub fn list_tokens(&self) -> Vec<(String, u64)> {
        use std::collections::BTreeSet;

        let state = self.state.read().unwrap();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut out: Vec<(String, u64)> = Vec::new();

        // Include native KANARI token (total supply tracked in state)
        seen.insert("KANARI".to_string());
        out.push(("KANARI".to_string(), state.total_supply));

        // Include registered treasuries (known supplies from RocksDB index)
        if let Ok(treasuries) = state.load_treasuries() {
            for (_owner, token_type, cap) in treasuries {
                if seen.insert(token_type.clone()) {
                    out.push((token_type, cap.total_supply));
                }
            }
        }

        out
    }

    /// Get token decimals
    pub fn get_token_decimals(&self, token_type: &str) -> Option<u8> {
        // Special case for native KANARI token
        if token_type == "KANARI" {
            return Some(9);
        }
        let state = self.state.read().unwrap();
        state.get_token_decimals(token_type).unwrap_or(None)
    }
}
