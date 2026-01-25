// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use centauri::blockchain::Blockchain;
use centauri::consensus::Checkpoint;
use kanari_move_runtime::ContractABI;
use kanari_move_runtime::changeset::ChangeSet;
use kanari_move_runtime::contract::{ContractInfo, ContractRegistry};
use kanari_move_runtime::gas::{GasMeter, GasOperation};
use kanari_move_runtime::move_runtime::MoveRuntime;
use kanari_move_runtime::state::StateManager;
use kanari_move_runtime::storage::persistent_store::PersistentStore;
use kanari_types::address::Address as KanariAddress;
use kanari_types::block::Block;
use kanari_types::event::Event;
use kanari_types::transaction::{SignedTransaction, Transaction};
use lru::LruCache;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use num_cpus;
use serde::{Deserialize, Serialize};
use smt::generate_merkle_proof;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

type ProofCache = LruCache<(u64, usize), (String, Vec<Vec<u8>>)>;

mod produce_dag_vertex;
pub use produce_dag_vertex::{CheckpointInfo, DagBlockInfo, DagEngine};

pub type BlockInfo = DagBlockInfo;

/// Complete blockchain engine with Move VM integration
pub struct BlockchainEngine {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub state: Arc<RwLock<StateManager>>,
    pub move_runtime: Arc<RwLock<MoveRuntime>>,
    pub pending_txs: Arc<RwLock<Vec<SignedTransaction>>>,
    pub contract_registry: Arc<RwLock<ContractRegistry>>,
    pub persistent_store: Option<Arc<PersistentStore>>,
    // Optional reusable pool of MoveRuntime instances for parallel execution
    pub runtime_pool:
        Option<Vec<Arc<std::sync::Mutex<kanari_move_runtime::move_runtime::MoveRuntime>>>>,
    // LRU cache for frequently requested merkle proofs
    // Cache key: (block_height, tx_index), Value: (tx_hash, proof)
    pub proof_cache: Arc<RwLock<ProofCache>>,
    // DAG engine for high-throughput consensus (lazy-initialized)
    dag_engine: Arc<RwLock<Option<DagEngine>>>,
    // Authority ID for this node (used in DAG mode)
    authority_id: String,
    // List of all authorities (validators) in the network
    authorities: Vec<String>,
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

type AccountProof = Option<(bool, Vec<u8>, Vec<Vec<u8>>)>;

impl BlockchainEngine {
    pub fn new() -> Result<Self> {
        // Try to open a persistent store for state + blockchain. If unavailable,
        // fall back to in-memory defaults. Under Miri, avoid opening disk-backed
        // stores to prevent unsupported OS calls during isolation.
        let persistent_store = if cfg!(miri) {
            None
        } else {
            match PersistentStore::open_default() {
                Ok(s) => Some(Arc::new(s)),
                Err(_) => None,
            }
        };

        let blockchain = if let Some(store) = &persistent_store {
            if let Ok(Some(mut b)) = store.load::<Blockchain>("blockchain") {
                // Rebuild transaction hash index after loading from disk
                b.rebuild_tx_hash_index();
                Arc::new(RwLock::new(b))
            } else {
                Arc::new(RwLock::new(Blockchain::new()))
            }
        } else {
            Arc::new(RwLock::new(Blockchain::new()))
        };

        let state = if let Some(store) = &persistent_store {
            if let Ok(Some(s)) = store.load::<StateManager>("state_manager") {
                Arc::new(RwLock::new(s))
            } else {
                // No persisted StateManager found — create fresh and populate
                // token supplies from any persisted MoveVM treasuries so token
                // state survives restarts.
                let mut sm = StateManager::new();
                if let Ok(mvs) =
                    kanari_move_runtime::storage::move_vm_state::MoveVMState::open_default()
                    && let Ok(treas) = mvs.load_treasuries()
                {
                    for (owner, token_type, cap) in treas.into_iter() {
                        sm.token_supplies.insert(token_type.clone(), cap.clone());
                        sm.token_treasuries.insert(token_type, owner);
                    }
                }
                Arc::new(RwLock::new(sm))
            }
        } else {
            Arc::new(RwLock::new(StateManager::new()))
        };

        // Use enhanced runtime with Kanari natives
        let move_runtime = Arc::new(RwLock::new(MoveRuntime::new_with_kanari_natives()?));

        // Note: treasuries are loaded above when StateManager is initialized
        let pending_txs = Arc::new(RwLock::new(Vec::new()));
        let contract_registry = Arc::new(RwLock::new(ContractRegistry::new()));

        // Initialize runtime pool (attempt to create one runtime per CPU)
        let mut runtime_pool: Option<
            Vec<Arc<std::sync::Mutex<kanari_move_runtime::move_runtime::MoveRuntime>>>,
        > = None;
        let workers = num_cpus::get().max(1);
        let mut pool_vec = Vec::new();
        for _ in 0..workers {
            match MoveRuntime::new_with_kanari_natives() {
                Ok(rt) => pool_vec.push(Arc::new(std::sync::Mutex::new(rt))),
                Err(e) => {
                    eprintln!("Failed to initialize runtime pool: {}", e);
                    pool_vec.clear();
                    break;
                }
            }
        }
        if !pool_vec.is_empty() {
            runtime_pool = Some(pool_vec);
        }

        // Initialize LRU cache for merkle proofs (cache up to 1000 proofs)
        let proof_cache = Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(1000).unwrap())));

        // Setup default authorities for DAG mode (single node by default)
        let authority_id = "0xDEFAULT_AUTHORITY".to_string();
        let authorities = vec![authority_id.clone()];

        Ok(Self {
            blockchain,
            state,
            move_runtime,
            pending_txs,
            contract_registry,
            persistent_store,
            runtime_pool,
            proof_cache,
            dag_engine: Arc::new(RwLock::new(None)),
            authority_id,
            authorities,
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

    /// Add signed transaction to pending pool after verifying signature
    pub fn submit_transaction(&self, signed_tx: SignedTransaction) -> Result<Vec<u8>> {
        // Require a signature and verify it. Empty signature is rejected.
        if !signed_tx.verify_signature()? {
            anyhow::bail!("Invalid or missing transaction signature");
        }

        let tx_hash = signed_tx.hash();
        let mut pending = self.pending_txs.write().unwrap();
        pending.push(signed_tx);
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

        // Check if DAG mode is enabled
        let is_dag_mode = self.blockchain.read().unwrap().dag_mode;

        if is_dag_mode {
            // In DAG mode: Execute directly against live state and apply changeset
            // This bypasses DAG consensus for immediate execution (needed for RPC)
            let changeset = {
                let mut runtime = self.move_runtime.write().unwrap();
                Self::execute_transaction_with_runtime(&tx, &mut runtime, &self.state)?
            };

            // Apply changeset to live state immediately
            if changeset.success {
                let mut state = self.state.write().unwrap();
                state.apply_changeset(&changeset)?;
            }

            Ok((tx_hash, changeset))
        } else {
            // Original behavior for non-DAG mode: Use snapshot (read-only simulation)
            // For immediate (RPC) execution we run the Move execution against a
            // cloned snapshot of the current StateManager so the call is a
            // read-only/simulated run: it returns the ChangeSet that would be
            // applied, but it does NOT mutate the engine's canonical `state`.
            // This prevents sequence number / balance drift when the same signed
            // transaction is later submitted for inclusion in a block.
            let changeset = {
                // Clone the current state for a safe simulation
                let mut state_snapshot = { self.state.read().unwrap().clone() };

                // Adjust the cloned snapshot to account for any pending transactions
                // from the same sender so that sequence validation during the
                // simulated execution reflects the expected sequence number once
                // pending transactions are included. This prevents immediate
                // execution from rejecting a transaction whose sequence has been
                // advanced by earlier pending submissions.
                if let Ok(pending) = self.pending_txs.read() {
                    for ptx in pending.iter() {
                        if ptx.transaction.sender_address() == tx.sender_address()
                            && let Ok(addr) =
                                AccountAddress::from_hex_literal(ptx.transaction.sender_address())
                        {
                            let acct = state_snapshot.get_or_create_account(addr);
                            acct.increment_sequence();
                        }
                    }
                }
                let state_arc = Arc::new(RwLock::new(state_snapshot));

                // Use the engine's runtime to execute against the cloned state.
                let mut runtime = self.move_runtime.write().unwrap();
                Self::execute_transaction_with_runtime(&tx, &mut runtime, &state_arc)?
            };

            Ok((tx_hash, changeset))
        }
    }

    /// Execute a single transaction and return ChangeSet
    /// This is the correct way: Move VM produces ChangeSet, StateManager applies it
    /// Execute a transaction using a provided `runtime` and `state_arc`.
    /// This is a static helper so worker threads can call it without borrowing `self`.
    fn execute_transaction_with_runtime(
        tx: &Transaction,
        runtime: &mut kanari_move_runtime::move_runtime::MoveRuntime,
        state_arc: &Arc<RwLock<StateManager>>,
    ) -> Result<ChangeSet> {
        Self::execute_transaction_with_runtime_internal(tx, runtime, state_arc, true)
    }

    /// Execute a transaction with option to skip sequence validation
    /// Used for syncing blocks where sequence is already validated by the original node
    fn execute_transaction_with_runtime_skip_seq(
        tx: &Transaction,
        runtime: &mut kanari_move_runtime::move_runtime::MoveRuntime,
        state_arc: &Arc<RwLock<StateManager>>,
    ) -> Result<ChangeSet> {
        Self::execute_transaction_with_runtime_internal(tx, runtime, state_arc, false)
    }

    /// Internal transaction execution with optional sequence validation
    fn execute_transaction_with_runtime_internal(
        tx: &Transaction,
        runtime: &mut kanari_move_runtime::move_runtime::MoveRuntime,
        state_arc: &Arc<RwLock<StateManager>>,
        validate_sequence: bool,
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

                let move_cs = runtime.publish_module(
                    module_bytes.clone(),
                    AccountAddress::from_hex_literal(sender)?,
                    None,
                )?;
                changeset.merge(move_cs);

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

                let move_cs = runtime.execute_entry_function(
                    &module_id,
                    function,
                    type_tags,
                    args.clone(),
                    Some(sender_addr),
                    None,
                )?;
                changeset.merge(move_cs);

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

    /// Original single-threaded entry that uses the engine's shared runtime.
    fn execute_transaction(&self, tx: &Transaction) -> Result<ChangeSet> {
        let mut runtime = self.move_runtime.write().unwrap();
        Self::execute_transaction_with_runtime(tx, &mut runtime, &self.state)
    }

    /// Execute transaction for sync (without affecting pending pool)
    /// Used when syncing blocks from network - executes transactions to rebuild state
    /// Skips sequence validation since transactions are already validated by the original node
    /// IMPORTANT: Uses main runtime (not pool) to ensure module bytecode is persisted correctly
    fn execute_transaction_sync(&self, tx: &Transaction) -> Result<ChangeSet> {
        // Always use main runtime for synced transactions to ensure modules are persisted
        let mut runtime = self.move_runtime.write().unwrap();
        Self::execute_transaction_with_runtime_skip_seq(tx, &mut runtime, &self.state)
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
        let dag_engine = self.dag_engine.read().unwrap();
        let dag_engine = dag_engine.as_ref().unwrap();
        dag_engine.produce_vertex()
    }

    /// Clone engine for DAG usage (internal helper)
    fn clone_for_dag(&self) -> BlockchainEngine {
        BlockchainEngine {
            blockchain: self.blockchain.clone(),
            state: self.state.clone(),
            move_runtime: self.move_runtime.clone(),
            pending_txs: self.pending_txs.clone(),
            contract_registry: self.contract_registry.clone(),
            persistent_store: self.persistent_store.clone(),
            runtime_pool: self.runtime_pool.clone(),
            proof_cache: self.proof_cache.clone(),
            dag_engine: Arc::new(RwLock::new(None)), // Don't clone DAG engine (prevent recursion)
            authority_id: self.authority_id.clone(),
            authorities: self.authorities.clone(),
        }
    }

    /// Configure authorities for DAG mode
    pub fn set_authorities(&mut self, authority_id: String, authorities: Vec<String>) {
        self.authority_id = authority_id;
        self.authorities = authorities;
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
        let chain = self.blockchain.read().unwrap();
        let state = self.state.read().unwrap();
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
        let state = self.state.read().unwrap();
        state.get_account_by_hex(address).map(|acc| {
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

            AccountInfo {
                address: format!("{:#x}", acc.address),
                balance: acc.balance,
                sequence_number: acc.sequence_number,
                modules: acc.modules.iter().cloned().collect(),
                token_balances: acc
                    .token_balances
                    .iter()
                    .map(|(k, v)| (k.clone(), v.value()))
                    .collect(),
                owned_objects: owned_objs,
            }
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
    pub fn get_all_token_balances(&self, address: &str) -> std::collections::HashMap<String, u64> {
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
        let runtime = self.move_runtime.read().unwrap();
        runtime.get_module_bytes(&module_id)
    }

    /// List all published modules in Move storage
    pub fn list_all_modules(&self) -> Vec<(String, String)> {
        let runtime = self.move_runtime.read().unwrap();
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
        chain.get_block(height).map(|block| {
            eprintln!(
                "[GET_FULL_BLOCK] Block #{} has {} transactions",
                height,
                block.transactions.len()
            );
            FullBlockData {
                height: block.header.height,
                timestamp: block.header.timestamp,
                hash: hex::encode(block.hash()),
                prev_hash: hex::encode(&block.header.prev_hash),
                state_root: hex::encode(&block.header.state_root),
                tx_count: block.transactions.len(),
                events: block.events.clone(),
                transactions: block.transactions.clone(),
            }
        })
    }

    /// Sync block from network data (simplified sync without full transaction re-execution)
    pub fn sync_block_from_data(&self, block_data: &BlockData) -> Result<()> {
        let mut chain = self.blockchain.write().unwrap();

        // Check if we already have this block
        if chain.get_block(block_data.height).is_some() {
            return Ok(()); // Already have it
        }

        // Verify this is the next block
        let current_height = chain.height();
        if block_data.height != current_height + 1 {
            anyhow::bail!(
                "Cannot sync block #{}: current height is {}",
                block_data.height,
                current_height
            );
        }

        // Create a placeholder block with the data we have
        // Note: This is simplified sync - we don't have the actual transactions
        let prev_hash = hex::decode(&block_data.prev_hash).context("Failed to decode prev_hash")?;
        let state_root =
            hex::decode(&block_data.state_root).context("Failed to decode state_root")?;

        let block = Block::new(
            block_data.height,
            prev_hash,
            state_root,
            vec![], // No transactions in simplified sync
            block_data.events.clone(),
        );

        // Add to blockchain without validation (trusted sync from peer)
        chain.add_block_with_validation(block, false)?;

        // Persist if we have a store
        if let Some(store) = &self.persistent_store {
            let _ = store.save("blockchain", &*chain);
        }

        Ok(())
    }

    /// Sync full block with transactions from network data
    /// This method executes all transactions to rebuild the state
    pub fn sync_full_block_from_data(&self, block_data: &FullBlockData) -> Result<()> {
        let mut chain = self.blockchain.write().unwrap();

        // Check if we already have this block
        if chain.get_block(block_data.height).is_some() {
            return Ok(()); // Already have it
        }

        // Verify this is the next block
        let current_height = chain.height();
        if block_data.height != current_height + 1 {
            anyhow::bail!(
                "Cannot sync block #{}: current height is {}",
                block_data.height,
                current_height
            );
        }

        // Always enable DAG mode for all nodes
        if !chain.dag_mode {
            chain.enable_dag_mode();
        }

        // Release chain lock before executing transactions
        drop(chain);

        // Verify all transaction signatures before executing
        eprintln!(
            "[SYNC] Verifying {} transaction signatures from block #{}",
            block_data.transactions.len(),
            block_data.height
        );
        for (i, signed_tx) in block_data.transactions.iter().enumerate() {
            // Require signature verification for all transactions from the network
            if !signed_tx.verify_signature()? {
                anyhow::bail!(
                    "Invalid or missing signature for transaction {} in block #{}",
                    i + 1,
                    block_data.height,
                );
            }
        }
        eprintln!("[SYNC] All transaction signatures verified");

        // Execute all transactions in the block to rebuild state
        eprintln!(
            "[SYNC] Executing {} transactions from block #{}",
            block_data.transactions.len(),
            block_data.height
        );
        let mut executed = 0;
        let mut _failed = 0;
        let mut all_changesets: Vec<ChangeSet> = Vec::new();

        for (i, signed_tx) in block_data.transactions.iter().enumerate() {
            eprintln!(
                "[SYNC] Executing transaction {}/{} from block #{}",
                i + 1,
                block_data.transactions.len(),
                block_data.height
            );
            match self.execute_transaction_sync(&signed_tx.transaction) {
                Ok(changeset) => {
                    eprintln!(
                        "[SYNC] Transaction {} executed, success={}",
                        i + 1,
                        changeset.success
                    );
                    if changeset.success {
                        executed += 1;
                    } else {
                        _failed += 1;
                    }
                    all_changesets.push(changeset);
                }
                Err(e) => {
                    eprintln!(
                        "[SYNC] Failed to execute synced transaction {}: {}",
                        i + 1,
                        e
                    );
                    _failed += 1;
                }
            }
        }

        // Apply all changesets to state
        eprintln!(
            "[SYNC] Applying {} changesets to state",
            all_changesets.len()
        );
        let mut state = self.state.write().unwrap();
        for (i, cs) in all_changesets.iter().enumerate() {
            eprintln!(
                "[SYNC] Applying changeset {}/{}",
                i + 1,
                all_changesets.len()
            );
            state.apply_changeset(cs)?;
        }

        // Compute new state root after applying all changes
        let computed_state_root = state.compute_state_root();
        eprintln!(
            "[SYNC] Computed state root: {}",
            hex::encode(&computed_state_root)
        );

        // Verify state root matches the one from the block
        let expected_state_root_bytes =
            hex::decode(&block_data.state_root.trim_start_matches("0x"))
                .context("Invalid state root format in block data")?;

        if computed_state_root != expected_state_root_bytes {
            drop(state);
            anyhow::bail!(
                "[SYNC] STATE ROOT MISMATCH!\n  Expected: {}\n  Computed: {}\n\nThis indicates state divergence. The node's state after executing transactions does not match the sender's state.\nPossible causes:\n  - Different genesis state\n  - Different transaction execution order\n  - Determinism issues in Move VM execution\n  - Missing prior blocks/transactions\n\nRecommendation: Clear state and resync from genesis.",
                block_data.state_root,
                hex::encode(&computed_state_root)
            );
        }

        eprintln!("[SYNC] ✅ State root verification passed!");
        let state_root = computed_state_root;
        drop(state);

        // Add checkpoint to blockchain (DAG mode)
        let add_result = {
            let mut chain = self.blockchain.write().unwrap();
            let prev_cp_hash = chain.latest_checkpoint().hash();
            let checkpoint = Checkpoint::new(
                block_data.height,
                Vec::new(), // vertices not available via simple block sync
                block_data.transactions.clone(),
                state_root.clone(),
                prev_cp_hash,
            );

            // Add checkpoint without strict validation (trusted peer data)
            let res = chain.add_checkpoint_with_validation(checkpoint, false);
            drop(chain);
            res
        };

        // Handle errors with DAG fallback for compatibility
        if let Err(e) = add_result {
            let emsg = format!("{}", e);
            if emsg.contains("Invalid previous checkpoint hash") {
                // Attempt DAG fallback: enable DAG mode and add checkpoint
                let mut chain = self.blockchain.write().unwrap();
                if !chain.dag_mode {
                    chain.enable_dag_mode();
                }

                let prev_cp_hash = chain.latest_checkpoint().hash();
                let checkpoint = Checkpoint::new(
                    block_data.height,
                    Vec::new(),
                    block_data.transactions.clone(),
                    state_root.clone(),
                    prev_cp_hash,
                );

                // Add checkpoint without strict validation (trusted sync)
                chain.add_checkpoint_with_validation(checkpoint, false)?;
                drop(chain);

                // Persist blockchain and state
                if let Some(store) = &self.persistent_store {
                    let chain = self.blockchain.read().unwrap();
                    store
                        .save("blockchain", &*chain)
                        .context("Failed to persist blockchain")?;
                    drop(chain);

                    let state_guard = self.state.read().unwrap();
                    store
                        .save("state_manager", &*state_guard)
                        .context("Failed to persist state manager")?;
                    drop(state_guard);
                }

                return Ok(());
            }
            return Err(e);
        }

        // Persist blockchain and state
        if let Some(store) = &self.persistent_store {
            let chain = self.blockchain.read().unwrap();
            store
                .save("blockchain", &*chain)
                .context("Failed to persist blockchain")?;
            drop(chain);

            let state_guard = self.state.read().unwrap();
            store
                .save("state_manager", &*state_guard)
                .context("Failed to persist state manager")?;
            drop(state_guard);

            if let Err(e) = store.save_smt_snapshot(block_data.height) {
                eprintln!(
                    "Failed to save SMT snapshot for synced block {}: {}",
                    block_data.height, e
                );
            }
        }

        eprintln!(
            "Synced block #{} with {}/{} successful transactions",
            block_data.height,
            executed,
            block_data.transactions.len()
        );

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
        use std::collections::HashSet;

        let state = self.state.read().unwrap();
        let mut seen: HashSet<String> = HashSet::new();
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

    /// Produce an SMT proof for the given account key (hex or address string).
    /// Returns Ok(None) if no persistent SMT is configured or the key wasn't found.
    pub fn get_account_proof(&self, key: &str) -> Result<AccountProof> {
        if let Some(store) = &self.persistent_store {
            if let Some((is_member, leaf, siblings)) = store.proof(key)? {
                let leaf_v = leaf.to_vec();
                let sibs_v = siblings.into_iter().map(|s| s.to_vec()).collect();
                return Ok(Some((is_member, leaf_v, sibs_v)));
            }
            Ok(None)
        } else {
            Ok(None)
        }
    }

    /// Produce an account proof at a historical block height using the SMT
    /// snapshot persisted at that height. Returns Ok(None) if snapshot
    /// unavailable.
    pub fn get_account_proof_at_height(&self, height: u64, key: &str) -> Result<AccountProof> {
        if let Some(store) = &self.persistent_store {
            if let Some(pairs) = store.load_smt_snapshot(height)? {
                use std::collections::HashMap;

                // Build map for lookup
                let mut map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
                for (k, v) in pairs.into_iter() {
                    map.insert(k, v);
                }

                // helper closures matching SMT key format
                let data_key = |kh: &[u8; 32]| -> Vec<u8> {
                    let mut out = b"smt:data:".to_vec();
                    out.extend(kh);
                    out
                };

                let node_key = |depth: usize, prefix: &[u8]| -> Vec<u8> {
                    let mut out = b"smt:node:".to_vec();
                    let d = (depth as u16).to_be_bytes();
                    out.extend(&d);
                    out.extend(prefix);
                    out
                };

                // compute key hash
                let kh = smt::digest(key.as_bytes());

                // default hashes
                let default_hashes = smt::default_hashes();

                // membership
                let is_member = map.contains_key(&data_key(&kh));

                // leaf hash
                let leaf_hash = if is_member {
                    let val = map.get(&data_key(&kh)).unwrap();
                    smt::hash_leaf(&kh, val.as_slice()).to_vec()
                } else {
                    default_hashes[256].to_vec()
                };

                let mut siblings: Vec<Vec<u8>> = Vec::new();
                for depth in (1..=256).rev() {
                    let prefix_bits: usize = depth;
                    let prefix_bytes = prefix_bits.div_ceil(8usize);
                    let mut prefix = vec![0u8; prefix_bytes];
                    prefix.copy_from_slice(&kh[..prefix_bytes]);
                    let excess = (prefix_bytes * 8) - prefix_bits;
                    if excess > 0 {
                        let mask = 0xFF << excess;
                        let last = prefix_bytes - 1;
                        prefix[last] &= mask as u8;
                    }

                    let last_bit_index = prefix_bits - 1;
                    let byte_idx = last_bit_index / 8;
                    let bit_in_byte = 7 - (last_bit_index % 8);

                    let mut sibling_prefix = prefix.clone();
                    sibling_prefix[byte_idx] ^= 1u8 << bit_in_byte;
                    let nk = node_key(depth, &sibling_prefix);
                    if let Some(v) = map.get(&nk) {
                        siblings.push(v.clone());
                    } else {
                        siblings.push(default_hashes[depth].to_vec());
                    }
                }

                return Ok(Some((is_member, leaf_hash, siblings)));
            }
            Ok(None)
        } else {
            Ok(None)
        }
    }

    /// Generate merkle proof for a transaction at given index in a block
    /// Uses LRU cache for frequently requested proofs
    pub fn get_transaction_merkle_proof(
        &self,
        block_height: u64,
        tx_index: usize,
    ) -> Result<Option<(String, Vec<Vec<u8>>)>> {
        let cache_key = (block_height, tx_index);

        // Check cache first
        {
            let mut cache = self.proof_cache.write().unwrap();
            if let Some(cached_proof) = cache.get(&cache_key) {
                return Ok(Some(cached_proof.clone()));
            }
        }

        // Not in cache, compute proof
        let chain = self.blockchain.read().unwrap();
        let block = chain
            .get_block(block_height)
            .ok_or_else(|| anyhow::anyhow!("Block not found at height {}", block_height))?;

        if tx_index >= block.transactions.len() {
            anyhow::bail!(
                "Transaction index {} out of bounds (block has {} transactions)",
                tx_index,
                block.transactions.len()
            );
        }

        // Collect transaction hashes
        let tx_hashes: Vec<Vec<u8>> = block.transactions.iter().map(|tx| tx.hash()).collect();

        // Generate proof
        let proof = generate_merkle_proof(&tx_hashes, tx_index);
        let tx_hash = hex::encode(&tx_hashes[tx_index]);

        let result = (tx_hash, proof);

        // Store in cache
        {
            let mut cache = self.proof_cache.write().unwrap();
            cache.put(cache_key, result.clone());
        }

        Ok(Some(result))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainStats {
    pub height: u64,
    pub total_blocks: usize,
    pub total_transactions: usize,
    pub pending_transactions: usize,
    pub total_accounts: usize,
    pub total_supply: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub address: String,
    pub balance: u64,
    pub sequence_number: u64,
    pub modules: Vec<String>,
    pub token_balances: std::collections::HashMap<String, u64>,
    /// Owned objects (object id, owner, type, data, version)
    pub owned_objects: Vec<ObjectInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub id: String,
    pub owner: String,
    pub type_: String,
    pub data: Vec<u8>,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    pub height: u64,
    pub timestamp: u64,
    pub hash: String,
    pub prev_hash: String,
    pub state_root: String,
    pub tx_count: usize,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullBlockData {
    pub height: u64,
    pub timestamp: u64,
    pub hash: String,
    pub prev_hash: String,
    pub state_root: String,
    pub tx_count: usize,
    pub events: Vec<Event>,
    pub transactions: Vec<SignedTransaction>,
}
