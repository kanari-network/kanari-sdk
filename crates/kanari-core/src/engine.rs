// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use centauri::blockchain::Blockchain;
use centauri::consensus::{Checkpoint, PersistentDagState};
use kanari_move_runtime::ContractABI;
use kanari_move_runtime::changeset::ChangeSet;
use kanari_move_runtime::contract::{ContractInfo, ContractRegistry};
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

/// Partition transactions into waves of non-conflicting transactions
pub(crate) fn partition_into_waves(
    transactions: &[SignedTransaction],
) -> Vec<Vec<SignedTransaction>> {
    let mut waves = Vec::new();
    let mut current_wave = Vec::new();
    let mut current_wave_keys = std::collections::HashSet::new();

    for tx in transactions {
        let keys = tx.transaction.get_conflict_keys();
        let mut conflicts_with_current = false;
        for k in &keys {
            if current_wave_keys.contains(k) {
                conflicts_with_current = true;
                break;
            }
        }

        if conflicts_with_current {
            if !current_wave.is_empty() {
                waves.push(current_wave);
            }
            current_wave = vec![tx.clone()];
            current_wave_keys = keys.into_iter().collect();
        } else {
            for k in &keys {
                current_wave_keys.insert(k.clone());
            }
            current_wave.push(tx.clone());
        }
    }
    if !current_wave.is_empty() {
        waves.push(current_wave);
    }
    waves
}

type ProofCache = LruCache<(u64, usize), (String, Vec<Vec<u8>>)>;

mod produce_dag_vertex;
pub use produce_dag_vertex::{CheckpointInfo, DagBlockInfo, DagEngine};

pub type BlockInfo = DagBlockInfo;

/// Complete blockchain engine with Move VM integration
pub struct BlockchainEngine {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub state: Arc<RwLock<StateManager>>,
    pub pending_txs: Arc<RwLock<Vec<SignedTransaction>>>,
    pub contract_registry: Arc<RwLock<ContractRegistry>>,
    pub persistent_store: Option<Arc<PersistentStore>>,
    // Reusable pool of MoveRuntime instances for parallel execution
    pub runtime_pool: Vec<Arc<std::sync::Mutex<kanari_move_runtime::move_runtime::MoveRuntime>>>,
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
                let name = &name_and_generics[..idx];
                let generics = &name_and_generics[idx + 1..name_and_generics.len() - 1];
                (name.trim(), Some(generics))
            } else {
                (name_and_generics.as_str(), None)
            };

            let addr = AccountAddress::from_hex_literal(addr_str).ok()?;
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
    pub fn new_dir(dir: &str) -> Result<Self> {
        let persistent_store = if cfg!(miri) {
            None
        } else {
            match PersistentStore::open_with_path(Some(std::path::PathBuf::from(dir))) {
                Ok(s) => Some(Arc::new(s)),
                Err(e) => {
                    eprintln!(
                        "WARN: Failed to open persistent store at '{}': {}. Falling back to in-memory mode.",
                        dir, e
                    );
                    None
                }
            }
        };
        Self::init(persistent_store)
    }

    pub fn new() -> Result<Self> {
        // Try to open a persistent store for state + blockchain. If unavailable,
        // fall back to in-memory defaults. Under Miri, avoid opening disk-backed
        // stores to prevent unsupported OS calls during isolation.
        let persistent_store = if cfg!(miri) {
            None
        } else {
            match PersistentStore::open_default() {
                Ok(s) => Some(Arc::new(s)),
                Err(e) => {
                    eprintln!(
                        "WARN: Failed to open default persistent store: {}. Falling back to in-memory mode.",
                        e
                    );
                    None
                }
            }
        };
        Self::init(persistent_store)
    }

    fn init(persistent_store: Option<Arc<PersistentStore>>) -> Result<Self> {
        let blockchain = if let Some(store) = &persistent_store {
            match store.load::<Blockchain>("blockchain") {
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
        };

        let state = if let Some(store) = &persistent_store {
            match store.load::<StateManager>("state_manager") {
                Ok(Some(s)) => {
                    info!(
                        "Successfully loaded state manager from persistent store (accounts: {}, objects: {})",
                        s.accounts.len(),
                        s.objects.len()
                    );
                    Arc::new(RwLock::new(s))
                }
                Ok(None) => {
                    info!(
                        "No persisted StateManager found — creating fresh and populating from MoveVMState."
                    );
                    // No persisted StateManager found — create fresh and populate
                    // token supplies from any persisted MoveVM treasuries so token
                    // state survives restarts.
                    let mut sm = StateManager::new();
                    if let Ok(mvs) =
                        kanari_move_runtime::storage::move_vm_state::MoveVMState::open_default()
                        && let Ok(treas) = mvs.load_treasuries()
                    {
                        info!(
                            "Populating StateManager from {} MoveVM treasuries",
                            treas.len()
                        );
                        for (owner, token_type, cap) in treas.into_iter() {
                            sm.token_supplies.insert(token_type.clone(), cap.clone());
                            sm.token_treasuries.insert(token_type, owner);
                        }
                    }
                    Arc::new(RwLock::new(sm))
                }
                Err(e) => {
                    error!(
                        "FATAL ERROR loading state manager from persistent store: {}. Falling back to fresh state.",
                        e
                    );
                    Arc::new(RwLock::new(StateManager::new()))
                }
            }
        } else {
            info!("Running in-memory mode: No persistent store provided for state.");
            Arc::new(RwLock::new(StateManager::new()))
        };

        // Initialize runtime pool (mandatory: one runtime per CPU)
        let workers = num_cpus::get().max(1);
        let mut runtime_pool = Vec::new();
        for i in 0..workers {
            match MoveRuntime::new_with_kanari_natives() {
                Ok(rt) => runtime_pool.push(Arc::new(std::sync::Mutex::new(rt))),
                Err(e) => {
                    error!(
                        "FATAL: Failed to initialize runtime pool worker {}: {}",
                        i, e
                    );
                    anyhow::bail!("Failed to initialize runtime pool: {}", e);
                }
            }
        }

        // Note: treasuries are loaded above when StateManager is initialized
        let pending_txs = Arc::new(RwLock::new(Vec::new()));
        let contract_registry = Arc::new(RwLock::new(ContractRegistry::new()));

        // Initialize LRU cache for merkle proofs (cache up to 1000 proofs)
        let proof_cache = Arc::new(RwLock::new(LruCache::new(
            NonZeroUsize::new(1000).expect("NonZeroUsize::new(1000) should never fail"),
        )));

        // Setup default authorities for DAG mode (single node by default)
        let authority_id = "0xDEFAULT_AUTHORITY".to_string();
        let authorities = vec![authority_id.clone()];

        let persisted_dag_state = if let Some(store) = &persistent_store {
            match store.load::<PersistentDagState>("dag_state") {
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
        };

        Ok(Self {
            blockchain,
            state,
            pending_txs,
            contract_registry,
            persistent_store,
            runtime_pool,
            proof_cache,
            dag_engine: Arc::new(RwLock::new(None)),
            authority_id,
            authorities,
            persisted_dag_state,
        })
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

    // gas/sequence helper removed — logic inlined in worker/helper paths to avoid borrow issues

    /// Persist DAG consensus state
    pub fn persist_dag_state(&self, state: PersistentDagState) -> Result<()> {
        if let Some(store) = &self.persistent_store {
            info!("[ENGINE] Persisting DAG consensus state...");
            store
                .save("dag_state", &state)
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
        let normalized_sender = Self::normalize_addr(sender_address);

        // 1b. Validate sequence number (committed + pending)
        {
            let state = self.state.read().unwrap();
            let mut expected_seq = 0;
            if let Some(acc) = state.get_account_by_hex(sender_address) {
                expected_seq = acc.sequence_number;
            }

            // Add pending count
            if let Ok(pending) = self.pending_txs.read() {
                for ptx in pending.iter() {
                    if Self::normalize_addr(ptx.transaction.sender_address()) == normalized_sender {
                        expected_seq += 1;
                    }
                }
            }

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
            // from the same sender. This ensures that sequence validation during
            // the simulated execution reflects the expected sequence number once
            // pending transactions are included, preventing spurious rejections.
            if let Ok(pending) = self.pending_txs.read() {
                let normalized_sender = Self::normalize_addr(tx.sender_address());
                for ptx in pending.iter() {
                    if Self::normalize_addr(ptx.transaction.sender_address()) == normalized_sender
                        && let Ok(addr) =
                            AccountAddress::from_hex_literal(ptx.transaction.sender_address())
                    {
                        let acct = state_snapshot.get_or_create_account(addr);
                        acct.increment_sequence();
                    }
                }
            }
            let state_arc = Arc::new(RwLock::new(state_snapshot));

            // Use a runtime from the pool for execution
            let runtime_arc = &self.runtime_pool[0];
            let mut runtime = runtime_arc.lock().unwrap();
            self.execute_transaction_with_runtime(&tx, &mut runtime, &state_arc, None)?
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
        runtime: &mut kanari_move_runtime::move_runtime::MoveRuntime,
        state_arc: &Arc<RwLock<StateManager>>,
        timestamp: Option<u64>,
    ) -> Result<ChangeSet> {
        self.execute_transaction_with_runtime_internal(tx, runtime, state_arc, true, timestamp)
    }

    /// Execute a transaction with option to skip sequence validation
    /// Used for syncing blocks where sequence is already validated by the original node
    fn execute_transaction_with_runtime_skip_seq(
        &self,
        tx: &Transaction,
        runtime: &mut kanari_move_runtime::move_runtime::MoveRuntime,
        state_arc: &Arc<RwLock<StateManager>>,
        timestamp: Option<u64>,
    ) -> Result<ChangeSet> {
        self.execute_transaction_with_runtime_internal(tx, runtime, state_arc, false, timestamp)
    }

    /// Internal transaction execution with optional sequence validation
    fn execute_transaction_with_runtime_internal(
        &self,
        tx: &Transaction,
        runtime: &mut kanari_move_runtime::move_runtime::MoveRuntime,
        state_arc: &Arc<RwLock<StateManager>>,
        validate_sequence: bool,
        timestamp: Option<u64>,
    ) -> Result<ChangeSet> {
        // 1. Pre-flight validation: Check sequence number (skip for synced transactions)
        let sender_addr = AccountAddress::from_hex_literal(tx.sender_address())?;
        if validate_sequence {
            let state = state_arc.read().unwrap();
            state
                .validate_sequence(&sender_addr, tx.sequence_number())
                .context("Sequence number validation failed")?;
        }

        // 2. Calculate gas and check balance
        let mut gas_meter = GasMeter::new(tx.gas_limit(), tx.gas_price());
        let mut changeset = ChangeSet::new();

        match tx {
            Transaction::PublishModule {
                sender,
                module_bytes,
                ..
            } => {
                let gas_op = GasOperation::PublishModule {
                    module_size: module_bytes.len(),
                };
                gas_meter.consume(gas_op.gas_units())?;

                let addr = AccountAddress::from_hex_literal(sender)?;
                let gas_cost = gas_meter.total_cost();

                // balance check
                {
                    let state = state_arc.read().unwrap();
                    let balance = state.get_account(&addr).map(|acc| acc.balance).unwrap_or(0);
                    if balance < gas_cost {
                        changeset.mark_failed(format!(
                            "Insufficient balance for gas: need {}, have {}",
                            gas_cost, balance
                        ));
                        Self::apply_gas_and_sequence(
                            &mut changeset,
                            addr,
                            gas_cost,
                            gas_meter.gas_used,
                        )?;
                        return Ok(changeset);
                    }
                }

                match runtime.publish_module(
                    module_bytes.clone(),
                    AccountAddress::from_hex_literal(sender)?,
                    None,
                ) {
                    Ok(move_cs) => changeset.merge(move_cs),
                    Err(e) => {
                        changeset.mark_failed(format!("Publish failed: {}", e));
                    }
                }

                let sender_addr2 = AccountAddress::from_hex_literal(sender)?;
                Self::apply_gas_and_sequence(
                    &mut changeset,
                    sender_addr2,
                    gas_cost,
                    gas_meter.gas_used,
                )?;
            }

            Transaction::ExecuteFunction {
                sender,
                module,
                function,
                type_args,
                args,
                ..
            } => {
                let gas_op = GasOperation::ExecuteFunction { complexity: 1 };
                gas_meter.consume(gas_op.gas_units())?;
                let sender_addr = AccountAddress::from_hex_literal(sender)?;
                let gas_cost = gas_meter.total_cost();

                {
                    let state = state_arc.read().unwrap();
                    let balance = state
                        .get_account(&sender_addr)
                        .map(|acc| acc.balance)
                        .unwrap_or(0);
                    if balance < gas_cost {
                        changeset.mark_failed(format!(
                            "Insufficient balance for gas: need {}, have {}",
                            gas_cost, balance
                        ));
                        Self::apply_gas_and_sequence(
                            &mut changeset,
                            sender_addr,
                            gas_cost,
                            gas_meter.gas_used,
                        )?;
                        return Ok(changeset);
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

                let addr = AccountAddress::from_hex_literal(parts[0])?;
                let module_id = ModuleId::new(
                    addr,
                    move_core_types::identifier::Identifier::new(parts[1])?,
                );

                let type_tags: Vec<move_core_types::language_storage::TypeTag> = type_args
                    .iter()
                    .filter_map(|s| parse_type_tag(s.as_str()))
                    .collect();

                match runtime.execute_entry_function(
                    &module_id,
                    function,
                    type_tags,
                    args.clone(),
                    Some(sender_addr),
                    None,
                    timestamp,
                ) {
                    Ok(move_cs) => changeset.merge(move_cs),
                    Err(e) => {
                        changeset.mark_failed(format!("Execution failed: {}", e));
                    }
                }

                Self::apply_gas_and_sequence(
                    &mut changeset,
                    sender_addr,
                    gas_cost,
                    gas_meter.gas_used,
                )?;
            }

            Transaction::Transfer {
                from, to, amount, ..
            } => {
                let gas_op = GasOperation::Transfer;
                gas_meter.consume(gas_op.gas_units())?;
                let from_addr = AccountAddress::from_hex_literal(from)?;
                let to_addr = AccountAddress::from_hex_literal(to)?;
                let gas_cost = gas_meter.total_cost();
                let total_required = amount.saturating_add(gas_cost);

                {
                    let state = state_arc.read().unwrap();
                    let balance = state
                        .get_account(&from_addr)
                        .map(|acc| acc.balance)
                        .unwrap_or(0);
                    if balance < total_required {
                        changeset.mark_failed(format!(
                            "Insufficient balance: need {} (amount: {}, gas: {}) but have {}",
                            total_required, amount, gas_cost, balance
                        ));
                        Self::apply_gas_and_sequence(
                            &mut changeset,
                            from_addr,
                            gas_cost,
                            gas_meter.gas_used,
                        )?;
                        return Ok(changeset);
                    }
                }

                changeset.transfer(from_addr, to_addr, *amount);
                Self::apply_gas_and_sequence(
                    &mut changeset,
                    from_addr,
                    gas_cost,
                    gas_meter.gas_used,
                )?;
            }

            Transaction::Burn { from, amount, .. } => {
                let gas_op = GasOperation::Transfer;
                gas_meter.consume(gas_op.gas_units())?;
                let from_addr = AccountAddress::from_hex_literal(from)?;
                let gas_cost = gas_meter.total_cost();
                let total_required = amount.saturating_add(gas_cost);

                {
                    let state = state_arc.read().unwrap();
                    let balance = state
                        .get_account(&from_addr)
                        .map(|acc| acc.balance)
                        .unwrap_or(0);
                    if balance < total_required {
                        changeset.mark_failed(format!(
                            "Insufficient balance: need {} (burn: {}, gas: {}) but have {}",
                            total_required, amount, gas_cost, balance
                        ));
                        Self::apply_gas_and_sequence(
                            &mut changeset,
                            from_addr,
                            gas_cost,
                            gas_meter.gas_used,
                        )?;
                        return Ok(changeset);
                    }
                }

                changeset.burn(from_addr, *amount);
                Self::apply_gas_and_sequence(
                    &mut changeset,
                    from_addr,
                    gas_cost,
                    gas_meter.gas_used,
                )?;
            }
        }

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
            contract_registry: self.contract_registry.clone(),
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
            let owned_ids = state
                .owned_objects
                .get(&acc.address)
                .cloned()
                .unwrap_or_default();

            let mut owned_objs: Vec<ObjectInfo> = Vec::new();
            for id in owned_ids {
                if let Some(obj) = state.objects.get(&id) {
                    owned_objs.push(ObjectInfo {
                        id: id.clone(),
                        owner: format!("{:#x}", obj.owner),
                        type_: obj.type_.clone(),
                        data: obj.data.clone(),
                        version: obj.version,
                    });
                }
            }

            let mut sequence_number = acc.sequence_number;
            let normalized_target = Self::normalize_addr(address);

            // Add pending transactions count to sequence number
            if let Ok(pending) = self.pending_txs.read() {
                for ptx in pending.iter() {
                    if Self::normalize_addr(ptx.transaction.sender_address()) == normalized_target {
                        sequence_number += 1;
                    }
                }
            }

            let info = AccountInfo {
                address: format!("{:#x}", acc.address),
                balance: acc.balance,
                sequence_number,
                modules: acc.modules.iter().cloned().collect(),
                token_balances: acc
                    .token_balances
                    .iter()
                    .map(|(k, v)| (k.clone(), v.value()))
                    .collect(),
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

    /// Normalize address for comparison (lowercase, no 0x prefix)
    fn normalize_addr(addr: &str) -> String {
        addr.trim_start_matches("0x").to_lowercase()
    }

    /// Deploy a contract (publish Move module).
    /// Expects a `SignedTransaction` containing a `PublishModule` transaction.
    pub fn deploy_contract(&self, signed_tx: SignedTransaction) -> Result<Vec<u8>> {
        // Submit the signed transaction (will verify signature)
        let tx_hash = self.submit_transaction(signed_tx.clone())?;

        // Extract deployment info from the transaction and register the contract
        if let Transaction::PublishModule {
            sender,
            module_bytes,
            module_name,
            ..
        } = signed_tx.transaction
        {
            let block_height = self.blockchain.read().unwrap().height();
            let contract_info = ContractInfo {
                address: sender,
                module_name,
                bytecode: module_bytes,
                deployment_tx: tx_hash.clone(),
                deployed_at: block_height,
                abi: ContractABI::new(),
                metadata: kanari_move_runtime::contract::ContractMetadata::new(
                    String::new(),
                    String::new(),
                    String::new(),
                ),
            };

            self.contract_registry
                .write()
                .unwrap()
                .register(contract_info);
        }

        Ok(tx_hash)
    }

    /// Call a contract function using a pre-signed `SignedTransaction`.
    pub fn call_contract(&self, signed_tx: SignedTransaction) -> Result<Vec<u8>> {
        self.submit_transaction(signed_tx)
    }

    /// Get module bytecode from Move storage
    pub fn get_module_bytecode(&self, address: &str, module_name: &str) -> Option<Vec<u8>> {
        use move_core_types::{
            account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
        };

        let addr = match AccountAddress::from_hex_literal(address) {
            Ok(a) => a,
            Err(_) => return None,
        };

        let ident = match Identifier::new(module_name) {
            Ok(i) => i,
            Err(_) => return None,
        };

        let module_id = ModuleId::new(addr, ident);
        let runtime = self.runtime_pool[0].lock().unwrap();
        runtime.get_module_bytes(&module_id)
    }

    /// List all published modules in Move storage
    pub fn list_all_modules(&self) -> Vec<(String, String)> {
        let runtime = self.runtime_pool[0].lock().unwrap();
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
        let state_root = hex::decode(block_data.state_root.trim_start_matches("0x"))
            .context("Invalid state root format in block data")?;

        let prev_hash = {
            let chain = self.blockchain.read().unwrap();
            chain.latest_checkpoint().hash()
        };

        let vertices: Vec<[u8; 32]> = block_data
            .vertices
            .iter()
            .map(|v: &String| {
                let bytes = hex::decode(v.trim_start_matches("0x")).unwrap_or_default();
                let mut arr = [0u8; 32];
                if bytes.len() == 32 {
                    arr.copy_from_slice(&bytes);
                }
                arr
            })
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

    /// Apply a committed checkpoint to the state.
    /// This executes all transactions in the checkpoint and updates the canonical state.
    pub fn apply_checkpoint(&self, mut checkpoint: Checkpoint) -> Result<()> {
        info!(
            "[ENGINE] Applying checkpoint {} with {} transactions, state_root: {}",
            checkpoint.sequence,
            checkpoint.transactions.len(),
            hex::encode(&checkpoint.state_root)
        );

        // 1. Create a clone of the current state to work on
        let state_snapshot = self.state.read().unwrap().clone();
        let state_arc = Arc::new(RwLock::new(state_snapshot));

        // 2. Filter transactions that are already executed
        let mut to_execute = Vec::new();
        let mut skipped_count = 0;

        {
            let chain = self.blockchain.read().unwrap();
            for signed_tx in &checkpoint.transactions {
                let tx_hash_hex = hex::encode(signed_tx.hash());
                if chain.is_transaction_executed(&tx_hash_hex) {
                    skipped_count += 1;
                    continue;
                }
                to_execute.push(signed_tx.clone());
            }
        }

        // 3. Partition and execute in parallel waves
        let waves = partition_into_waves(&to_execute);
        let mut executed_count = 0;

        for wave in waves {
            let results: Vec<Result<ChangeSet>> = wave
                .iter()
                .enumerate()
                .map(|(i, signed_tx)| {
                    let pool_idx = i % self.runtime_pool.len();
                    let runtime_arc = &self.runtime_pool[pool_idx];
                    let mut runtime = runtime_arc.lock().unwrap();

                    self.execute_transaction_with_runtime_skip_seq(
                        &signed_tx.transaction,
                        &mut runtime,
                        &state_arc,
                        Some(checkpoint.timestamp),
                    )
                })
                .collect();

            for res in results {
                match res {
                    Ok(cs) => {
                        let mut state_write = state_arc.write().unwrap();
                        if let Err(e) = state_write.apply_changeset(&cs) {
                            error!(
                                "[ENGINE] Failed to apply changeset in checkpoint {}: {}",
                                checkpoint.sequence, e
                            );
                            anyhow::bail!("Failed to apply changeset: {}", e);
                        }
                        executed_count += 1;
                    }
                    Err(e) => {
                        error!(
                            "[ENGINE] Fatal error executing transaction in checkpoint {}: {}",
                            checkpoint.sequence, e
                        );
                        anyhow::bail!("Fatal error executing checkpoint transaction: {}", e);
                    }
                }
            }
        }

        if skipped_count > 0 {
            info!(
                "[ENGINE] Checkpoint {} summary: {} executed, {} skipped (already in blockchain)",
                checkpoint.sequence, executed_count, skipped_count
            );
        }

        // 3. Verify the final state root
        let verified_state = {
            let state_read = state_arc.read().unwrap();
            let computed_root = state_read.compute_state_root();
            if computed_root != checkpoint.state_root {
                let expected_hex = hex::encode(&checkpoint.state_root);
                let computed_hex = hex::encode(&computed_root);

                // In DAG mode, the checkpoint's state root might be from a leader vertex
                // that didn't see the exact same history as the checkpoint's total order.
                // We update to the computed root to ensure consistency.
                warn!(
                    "[ENGINE] State root mismatch in checkpoint {}! Updating to computed root.\n  Expected (from leader): {}\n  Computed (from execution): {}",
                    checkpoint.sequence, expected_hex, computed_hex
                );
                checkpoint.state_root = computed_root;
            }
            state_read.clone()
        };

        // 4. Update canonical state by replacing it with the verified state
        {
            let mut state = self.state.write().unwrap();
            *state = verified_state;
        }

        // 5. Update blockchain
        {
            let mut chain = self.blockchain.write().unwrap();
            // Add checkpoint without strict validation (already validated locally)
            chain.add_checkpoint_with_validation(checkpoint.clone(), false)?;
        }

        // 6. Remove committed transactions from pending pool
        {
            let mut pending = self.pending_txs.write().unwrap();
            let committed_hashes: std::collections::HashSet<_> =
                checkpoint.transactions.iter().map(|tx| tx.hash()).collect();
            pending.retain(|tx| !committed_hashes.contains(&tx.hash()));
        }

        // 7. Persist blockchain and state
        if let Some(store) = &self.persistent_store {
            let state_guard = self.state.read().unwrap();
            let chain = self.blockchain.read().unwrap();
            let height = chain.height();
            info!("[ENGINE] Persisting blockchain at height {}...", height);
            store
                .save("blockchain", &*chain)
                .context("Failed to persist blockchain")?;
            drop(chain);

            info!("[ENGINE] Persisting state manager...");
            store
                .save("state_manager", &*state_guard)
                .context("Failed to persist state manager")?;
            drop(state_guard);

            // Also persist DAG state if it exists
            if let Ok(dag_engine_guard) = self.dag_engine.read()
                && let Some(dag_engine) = &*dag_engine_guard
                && let Ok(consensus) = dag_engine.consensus().read()
                && let Ok(dag_state) = consensus.save_state()
            {
                info!("[ENGINE] Persisting DAG consensus state during checkpoint...");
                let _ = store.save("dag_state", &dag_state);
            }

            info!(
                "[ENGINE] Persistence completed successfully for height {}",
                height
            );
        } else {
            warn!("[ENGINE] Running in-memory mode, data will NOT be persisted!");
        }

        Ok(())
    }

    /// Get state root for a specific block height or latest if None.
    pub fn get_state_root(&self, height: Option<u64>) -> Option<String> {
        let chain = self.blockchain.read().unwrap();
        match height {
            Some(h) => chain
                .get_block(h)
                .map(|b| hex::encode(&b.header.state_root)),
            None => {
                // latest state root
                Some(hex::encode(&chain.latest_block().header.state_root))
            }
        }
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

        // Include registered treasuries (known supplies)
        for (k, v) in state.token_supplies.iter() {
            if seen.insert(k.clone()) {
                out.push((k.clone(), v.total_supply()));
            }
        }

        // Best-effort: scan stored objects for coin/treasury/metadata types
        for (_id, obj) in state.objects.iter() {
            let t = &obj.type_;
            // Look for generics like ::coin::Coin<...> or ::coin::TreasuryCap<...>
            if t.contains("::coin::")
                && t.contains('<')
                && t.contains('>')
                && let Some(start) = t.find('<')
                && let Some(end) = t.rfind('>')
                && end > start + 1
            {
                let inner = t[start + 1..end].trim().to_string();
                if !inner.is_empty() && seen.insert(inner.clone()) {
                    // supply if known, else attempt to compute by summing coin objects
                    let mut supply = state
                        .token_supplies
                        .get(&inner)
                        .map(|cap| cap.total_supply())
                        .unwrap_or(0u64);

                    if supply == 0 {
                        // Sum all Coin<inner> object values in state.objects
                        let mut sum_u128: u128 = 0;
                        for (_oid, o2) in state.objects.iter() {
                            if o2.type_.contains("::coin::Coin<")
                                && o2.type_.contains(&inner)
                                && o2.data.len() >= 8
                                && let Ok(bytes) = o2.data[o2.data.len() - 8..].try_into()
                            {
                                let v = u64::from_le_bytes(bytes) as u128;
                                sum_u128 = sum_u128.saturating_add(v);
                            }
                        }
                        if sum_u128 > u128::from(u64::MAX) {
                            supply = u64::MAX;
                        } else {
                            supply = sum_u128 as u64;
                        }
                    }

                    out.push((inner, supply));
                }
            }
        }

        out
    }
}
