// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::blockchain::{Block, Blockchain, SignedTransaction, Transaction};
use anyhow::{Context, Result};
use crossbeam_channel as cbchan;
use kanari_move_runtime::ContractABI;
use kanari_move_runtime::changeset::{ChangeSet, Event};
use kanari_move_runtime::contract::{
    ContractCall, ContractDeployment, ContractInfo, ContractRegistry,
};
use kanari_move_runtime::gas::{GasMeter, GasOperation};
use kanari_move_runtime::move_runtime::MoveRuntime;
use kanari_move_runtime::state::StateManager;
use kanari_move_runtime::storage::persistent_store::PersistentStore;
use kanari_types::address::Address as KanariAddress;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use num_cpus;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// Complete blockchain engine with Move VM integration
pub struct BlockchainEngine {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub state: Arc<RwLock<StateManager>>,
    pub move_runtime: Arc<RwLock<MoveRuntime>>,
    pub pending_txs: Arc<RwLock<Vec<Transaction>>>,
    pub contract_registry: Arc<RwLock<ContractRegistry>>,
    pub persistent_store: Option<Arc<PersistentStore>>,
    // Optional reusable pool of MoveRuntime instances for parallel execution
    pub runtime_pool:
        Option<Vec<Arc<std::sync::Mutex<kanari_move_runtime::move_runtime::MoveRuntime>>>>,
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
                    if depth > 0 {
                        depth -= 1;
                    }
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

    if let Some(inner) = s.strip_prefix("vector<") {
        if inner.ends_with('>') {
            let inner = &inner[..inner.len() - 1];
            return parse_type_tag(inner).map(|t| TypeTag::Vector(Box::new(t)));
        }
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
    pub fn new() -> Result<Self> {
        // Try to open a persistent store for state + blockchain. If unavailable,
        // fall back to in-memory defaults.
        let persistent_store = match PersistentStore::open_default() {
            Ok(s) => Some(Arc::new(s)),
            Err(_) => None,
        };

        let blockchain = if let Some(store) = &persistent_store {
            if let Ok(Some(b)) = store.load::<Blockchain>("blockchain") {
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
                Arc::new(RwLock::new(StateManager::new()))
            }
        } else {
            Arc::new(RwLock::new(StateManager::new()))
        };

        // Use enhanced runtime with Kanari natives
        let move_runtime = Arc::new(RwLock::new(MoveRuntime::new_with_kanari_natives()?));
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

        Ok(Self {
            blockchain,
            state,
            move_runtime,
            pending_txs,
            contract_registry,
            persistent_store,
            runtime_pool,
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
        // Verify signature before accepting transaction
        if !signed_tx.verify_signature()? {
            anyhow::bail!("Invalid transaction signature");
        }

        let tx_hash = signed_tx.hash();
        let mut pending = self.pending_txs.write().unwrap();
        pending.push(signed_tx.transaction);
        Ok(tx_hash)
    }

    /// Execute transaction immediately and return both hash and changeset
    /// Used by RPC to get object IDs created during execution
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

        // Execute transaction to get changeset
        let changeset = self.execute_transaction(&tx)?;

        // Apply changeset to state immediately
        {
            let mut state = self.state.write().unwrap();
            state.apply_changeset(&changeset)?;
        }

        Ok((tx_hash, changeset))
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
        // 1. Pre-flight validation: Check sequence number
        let sender_addr = AccountAddress::from_hex_literal(tx.sender_address())?;
        {
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
        Self::execute_transaction_with_runtime(tx, &mut *runtime, &self.state)
    }

    /// Mine/produce a new block with pending transactions
    /// Now uses ChangeSet pattern: execute -> collect ChangeSets -> apply atomically
    ///
    /// CRITICAL: ALL ChangeSets (both successful and failed) are applied to state.
    /// Failed transactions still deduct gas and increment sequence to prevent spam and replay attacks.
    pub fn produce_block(&self) -> Result<BlockInfo> {
        let mut pending = self.pending_txs.write().unwrap();

        if pending.is_empty() {
            anyhow::bail!("No pending transactions");
        }

        let transactions = pending.drain(..).collect::<Vec<_>>();
        let tx_count = transactions.len();

        // Parallel execution: create a small pool of MoveRuntime instances and
        // execute transactions in parallel to produce ChangeSets. We still apply
        // all ChangeSets atomically on the main thread to ensure deterministic
        // state updates.
        let mut all_changesets: Vec<ChangeSet> = Vec::with_capacity(tx_count);
        let mut executed = 0usize;
        let mut failed = 0usize;
        let mut _total_gas_used = 0u64;

        // Parallel execution using worker threads and runtime pool when available.
        if tx_count > 1 {
            let workers = std::cmp::min(num_cpus::get().max(1), tx_count);
            let (job_tx, job_rx) = cbchan::unbounded::<(usize, Transaction)>();
            let (res_tx, res_rx) = cbchan::unbounded::<(usize, Result<ChangeSet>)>();
            let mut handles = Vec::new();

            if let Some(pool) = &self.runtime_pool {
                for i in 0..workers {
                    let job_rx = job_rx.clone();
                    let res_tx = res_tx.clone();
                    let pool_entry = pool[i % pool.len()].clone();
                    let state_arc = self.state.clone();

                    let handle = std::thread::spawn(move || {
                        while let Ok((idx, tx)) = job_rx.recv() {
                            let mut guard = pool_entry.lock().unwrap();
                            let res = BlockchainEngine::execute_transaction_with_runtime(
                                &tx,
                                &mut *guard,
                                &state_arc,
                            );
                            let _ = res_tx.send((idx, res));
                        }
                    });
                    handles.push(handle);
                }
            } else {
                // create local runtimes for workers
                let mut created = true;
                for _ in 0..workers {
                    match kanari_move_runtime::move_runtime::MoveRuntime::new_with_kanari_natives()
                    {
                        Ok(mut runtime) => {
                            let job_rx = job_rx.clone();
                            let res_tx = res_tx.clone();
                            let state_arc = self.state.clone();
                            let handle = std::thread::spawn(move || {
                                while let Ok((idx, tx)) = job_rx.recv() {
                                    let res = BlockchainEngine::execute_transaction_with_runtime(
                                        &tx,
                                        &mut runtime,
                                        &state_arc,
                                    );
                                    let _ = res_tx.send((idx, res));
                                }
                            });
                            handles.push(handle);
                        }
                        Err(e) => {
                            eprintln!(
                                "Failed to create runtime for worker: {}. Falling back to sequential.",
                                e
                            );
                            created = false;
                            break;
                        }
                    }
                }

                if !created {
                    // fallback to sequential if worker creation failed
                    for tx in &transactions {
                        match self.execute_transaction(tx) {
                            Ok(changeset) => {
                                if changeset.success {
                                    executed += 1;
                                } else {
                                    failed += 1;
                                }
                                _total_gas_used += changeset.gas_used;
                                all_changesets.push(changeset);
                            }
                            Err(e) => {
                                eprintln!("Transaction execution error: {:?}", e);
                                failed += 1;
                            }
                        }
                    }
                    // skip job dispatch below
                    // ensure we don't try to collect from channels we didn't use
                    // (note: handles may be empty)
                }
            }

            // If we have worker handles, dispatch jobs and collect results
            if !handles.is_empty() {
                // Serialize execution per-sender to avoid races where multiple
                // transactions from the same sender validate the same sequence
                // concurrently. We build per-sender queues, dispatch the first
                // tx for every sender, and when a result arrives we dispatch
                // the next tx for that sender.
                use std::collections::{HashMap, VecDeque};

                // Build per-sender queues preserving original indices
                let mut per_sender: HashMap<String, VecDeque<(usize, Transaction)>> =
                    HashMap::new();
                for (i, tx) in transactions.iter().cloned().enumerate() {
                    per_sender
                        .entry(tx.sender().to_string())
                        .or_insert_with(VecDeque::new)
                        .push_back((i, tx));
                }

                let mut results: Vec<Option<ChangeSet>> = vec![None; tx_count];
                let mut idx_to_sender: HashMap<usize, String> = HashMap::new();

                // Initially dispatch one tx per sender
                for (sender, queue) in per_sender.iter_mut() {
                    if let Some((idx, tx)) = queue.pop_front() {
                        job_tx.send((idx, tx)).unwrap();
                        idx_to_sender.insert(idx, sender.clone());
                    }
                }

                // Track how many results we've collected
                let mut collected = 0usize;

                while collected < tx_count {
                    if let Ok((idx, res)) = res_rx.recv() {
                        match res {
                            Ok(cs) => {
                                results[idx] = Some(cs);
                            }
                            Err(e) => {
                                eprintln!("Transaction execution error in worker: {:?}", e);
                                results[idx] = None;
                            }
                        }

                        // Determine sender for this finished tx and dispatch next
                        if let Some(sender) = idx_to_sender.remove(&idx) {
                            if let Some(queue) = per_sender.get_mut(&sender) {
                                if let Some((next_idx, next_tx)) = queue.pop_front() {
                                    job_tx.send((next_idx, next_tx)).unwrap();
                                    idx_to_sender.insert(next_idx, sender.clone());
                                }
                            }
                        }

                        collected += 1;
                    }
                }

                drop(job_tx);
                drop(res_tx);
                for h in handles {
                    let _ = h.join();
                }

                for opt in results.into_iter() {
                    if let Some(cs) = opt {
                        if cs.success {
                            executed += 1;
                        } else {
                            failed += 1;
                        }
                        _total_gas_used += cs.gas_used;
                        all_changesets.push(cs);
                    } else {
                        failed += 1;
                    }
                }
            }
        } else {
            // Single transaction -> sequential path
            for tx in &transactions {
                match self.execute_transaction(tx) {
                    Ok(changeset) => {
                        if changeset.success {
                            executed += 1;
                        } else {
                            eprintln!("Transaction failed: {:?}", changeset.error_message);
                            failed += 1;
                        }
                        _total_gas_used += changeset.gas_used;
                        all_changesets.push(changeset);
                    }
                    Err(e) => {
                        eprintln!("Transaction execution error: {:?}", e);
                        failed += 1;
                    }
                }
            }
        }

        // Apply ALL ChangeSets atomically (both successful and failed) and collect events.
        let block_events: Vec<Event> = {
            let mut state = self.state.write().unwrap();
            for changeset in &all_changesets {
                state
                    .apply_changeset(changeset)
                    .context("Failed to apply changeset to state")?;
            }

            // Drain events accumulated in state into block-level events
            state.drain_events()
        };

        // Create new block
        let mut chain = self.blockchain.write().unwrap();
        let prev_hash = chain.latest_block().hash();
        let height = chain.height() + 1;

        let state_root = {
            let state_guard = self.state.read().unwrap();
            state_guard.compute_state_root()
        };

        let block = Block::new(
            height,
            prev_hash,
            state_root,
            transactions,
            block_events.clone(),
        );
        let block_hash = block.hash();

        chain.add_block(block)?;

        // Persist updated blockchain and state if a persistent store is available
        if let Some(store) = &self.persistent_store {
            // Persist blockchain
            store
                .save("blockchain", &*chain)
                .context("Failed to persist blockchain")?;

            // Persist state snapshot
            let state_guard = self.state.read().unwrap();
            store
                .save("state_manager", &*state_guard)
                .context("Failed to persist state manager")?;

            // Also snapshot SMT backing store for historical proofs when available
            let block_height = height;
            if let Err(e) = store.save_smt_snapshot(block_height) {
                eprintln!(
                    "Failed to save SMT snapshot for height {}: {}",
                    block_height, e
                );
            }
        }

        Ok(BlockInfo {
            height,
            hash: hex::encode(&block_hash),
            tx_count,
            executed,
            failed,
            events: block_events,
        })
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
                        id: obj.id.clone(),
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
                token_balances: acc.token_balances.clone(),
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
            .map(|acc| acc.token_balances.clone())
            .unwrap_or_default()
    }

    /// Deploy a contract (publish Move module)
    pub fn deploy_contract(&self, deployment: ContractDeployment) -> Result<Vec<u8>> {
        // Determine current sequence number for the publisher from state
        let sequence_number = {
            let state = self.state.read().unwrap();
            state
                .get_account_by_hex(&deployment.publisher_address())
                .map(|acc| acc.sequence_number)
                .unwrap_or(0)
        };

        let tx = Transaction::PublishModule {
            sender: deployment.publisher_address(),
            module_bytes: deployment.bytecode.clone(),
            module_name: deployment.module_name.clone(),
            gas_limit: deployment.gas_limit,
            gas_price: deployment.gas_price,
            sequence_number,
        };

        // Submit transaction
        let signed_tx = SignedTransaction::new(tx.clone());
        let tx_hash = self.submit_transaction(signed_tx)?;

        // Register contract in registry
        let block_height = self.blockchain.read().unwrap().height();
        let contract_info = ContractInfo {
            address: deployment.publisher_address(),
            module_name: deployment.module_name,
            bytecode: deployment.bytecode,
            deployment_tx: tx_hash.clone(),
            deployed_at: block_height,
            abi: ContractABI::new(),
            metadata: deployment.metadata,
        };

        self.contract_registry
            .write()
            .unwrap()
            .register(contract_info);

        Ok(tx_hash)
    }

    /// Call a contract function
    pub fn call_contract(&self, call: ContractCall) -> Result<Vec<u8>> {
        let sender_hex = format!("0x{}", hex::encode(call.sender.to_vec()));

        // Read current sequence number from state (fallback to 0 if account not found)
        let sequence_number = {
            let state = self.state.read().unwrap();
            state
                .get_account_by_hex(&sender_hex)
                .map(|acc| acc.sequence_number)
                .unwrap_or(0)
        };

        let tx = Transaction::ExecuteFunction {
            sender: sender_hex,
            module: call.module_address(),
            function: call.function.clone(),
            type_args: call.type_args.iter().map(|t| format!("{}", t)).collect(),
            args: call.args.clone(),
            gas_limit: call.gas_limit,
            gas_price: call.gas_price,
            sequence_number,
        };

        // Submit transaction
        let signed_tx = SignedTransaction::new(tx);
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
            hash: hex::encode(&block.hash()),
            prev_hash: hex::encode(&block.header.prev_hash),
            state_root: hex::encode(&block.header.state_root),
            tx_count: block.transactions.len(),
            events: block.events.clone(),
        })
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

    /// Produce an SMT proof for the given account key (hex or address string).
    /// Returns Ok(None) if no persistent SMT is configured or the key wasn't found.
    pub fn get_account_proof(&self, key: &str) -> Result<Option<(bool, Vec<u8>, Vec<Vec<u8>>)>> {
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
    pub fn get_account_proof_at_height(
        &self,
        height: u64,
        key: &str,
    ) -> Result<Option<(bool, Vec<u8>, Vec<Vec<u8>>)>> {
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
                    let prefix_bits = depth;
                    let prefix_bytes = (prefix_bits + 7) / 8;
                    let mut prefix = vec![0u8; prefix_bytes as usize];
                    prefix.copy_from_slice(&kh[..prefix_bytes as usize]);
                    let excess = (prefix_bytes * 8) - prefix_bits as usize;
                    if excess > 0 {
                        let mask = 0xFF << excess;
                        let last = prefix_bytes as usize - 1;
                        prefix[last] &= mask as u8;
                    }

                    let last_bit_index = prefix_bits - 1;
                    let byte_idx = (last_bit_index / 8) as usize;
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
pub struct BlockInfo {
    pub height: u64,
    pub hash: String,
    pub tx_count: usize,
    pub executed: usize,
    pub failed: usize,
    pub events: Vec<Event>,
}
