// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// This file contains the MoveRuntime wrapper implementation.
// It utilizes MoveVM and InMemoryStorage for executing functions and publishing modules.
// Enhanced with native function support, gas metering, and session management.
use crate::storage::resolver::KanariMoveResolver;
use anyhow::Result;
use kanari_crypto::hash_data_blake3;
use kanari_types::event::Event;
use log::debug;
use move_binary_format::file_format::CompiledModule;
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::{IdentStr, Identifier};
use move_core_types::language_storage::{ModuleId, StructTag, TypeTag};
use move_vm_runtime::move_vm::MoveVM;
use move_vm_runtime::native_functions::NativeFunctionTable;
use move_vm_types::gas::UnmeteredGasMeter;
mod gas_ops;
mod helpers;
mod load_system_modules;
mod object_ops;
mod parsers;
use crate::gas::GasOperation;
use kanari_types::address::Address as KanariAddress;
use kanari_types::tx_context::TxContextModule;
pub mod move_runtime_extensions;
use crate::changeset::ChangeSet;
use crate::storage::move_vm_state::MoveVMState;
use crate::storage::object_storage::{ObjectStorage, ObjectStore, StoredObject};
use kanari_types::tx_context::TxContextRecord;
use move_vm_types::loaded_data::runtime_types::Type as RuntimeType;

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// Enhanced runtime wrapper around `move-vm` for executing functions, publishing modules,
/// and running scripts. Supports custom native functions, gas metering, and atomic sessions.
#[derive(Clone)]
pub struct MoveRuntime {
    pub(crate) vm: Arc<MoveVM>,
    pub(crate) resolver: KanariMoveResolver,
    pub(crate) state: MoveVMState,
    /// Index of published modules for faster listing
    pub(crate) published_modules: Arc<RwLock<HashSet<ModuleId>>>,
    /// Persistent object storage for transferred objects
    pub(crate) object_storage: Arc<dyn ObjectStore>,
}

// 🚨 เพิ่ม Struct สำหรับใบเสร็จไว้ด้านบนสุดของไฟล์
#[derive(serde::Serialize)]
struct AutoMergeReceiptData {
    owner: AccountAddress,
    merged_count: u64,
    total_amount: u64,
}

type LoadedMutableObject = (usize, String, AccountAddress, String, u64);

impl MoveRuntime {
    /// Open the runtime using the default persistent DB path (see README).
    /// Initializes VM without custom natives (for basic usage).
    pub fn new() -> Result<Self> {
        Self::new_with_natives(vec![])
    }

    /// Create a new MoveRuntime with custom native functions.
    ///
    /// # Arguments
    /// * `natives` - Native function tables for custom functions (e.g., crypto operations)
    ///
    /// # Example
    /// ```ignore
    /// use kanari_crypto::move_natives;
    /// use move_core_types::account_address::AccountAddress;
    ///
    /// let system_addr = AccountAddress::from_hex_literal("0x2").unwrap();
    /// let natives = move_natives::all_natives(system_addr);
    /// let runtime = MoveRuntime::new_with_natives(vec![natives])?;
    /// ```
    pub fn new_with_natives(natives: Vec<NativeFunctionTable>) -> Result<Self> {
        let state = if cfg!(miri) {
            MoveVMState::new_in_memory()?
        } else {
            MoveVMState::open_default()?
        };
        Self::new_internal(natives, state)
    }

    /// Create a new MoveRuntime with custom native functions and in-memory state.
    /// Useful for creating multiple runtime instances in parallel without DB lock contention.
    pub fn new_with_natives_in_memory(natives: Vec<NativeFunctionTable>) -> Result<Self> {
        let state = MoveVMState::new_in_memory()?;
        Self::new_internal(natives, state)
    }

    fn new_internal(natives: Vec<NativeFunctionTable>, state: MoveVMState) -> Result<Self> {
        // Flatten all native tables into a single iterator
        let all_natives: Vec<_> = natives
            .into_iter()
            .flat_map(|table| table.into_iter())
            .collect();

        // Initialize VM with custom natives
        // Before creating VM, log what natives we are about to register for visibility.
        debug!(
            "[RUNTIME] registering {} native functions for MoveVM",
            all_natives.len()
        );
        for (addr, module_id, func_id, _nf) in all_natives.iter() {
            debug!(
                "[RUNTIME] native -> module: {}  function: {}  address: {:?}",
                module_id, func_id, addr
            );
        }

        let vm = MoveVM::new(all_natives)
            .map_err(|e| anyhow::anyhow!(format!("VM init error: {:?}", e)))?;

        debug!("[RUNTIME] MoveVM initialized (custom natives registered)");

        // Try to initialize persistent object storage; fall back to in-memory if it fails.
        // If state is in-memory, prefer in-memory object storage too.
        let object_storage: Arc<dyn ObjectStore> = if cfg!(miri) {
            Arc::from(ObjectStorage::boxed_inmemory())
        } else {
            match ObjectStorage::boxed_with_persistence() {
                Ok(store) => Arc::from(store),
                Err(e) => {
                    log::warn!(
                        "[RUNTIME] failed to initialize persistent ObjectStorage: {}. Falling back to in-memory.",
                        e
                    );
                    Arc::from(ObjectStorage::boxed_inmemory())
                }
            }
        };

        // Initialize resolver with state and object storage
        let resolver = KanariMoveResolver {
            state: state.clone(),
            _object_storage: object_storage.clone(),
        };

        // Initialize published modules set from state
        // Populate the set from the persistent index to support listing modules.
        let published_modules: HashSet<ModuleId> = state
            .get_all_module_ids()
            .unwrap_or_default()
            .into_iter()
            .collect();

        Ok(MoveRuntime {
            vm: Arc::new(vm),
            resolver,
            state,
            published_modules: Arc::new(RwLock::new(published_modules)),
            object_storage,
        })
    }

    /// Create runtime with Kanari system natives (crypto + stdlib + object + tx_context)
    /// Also loads pre-compiled Kanari system modules (0x2::*)
    pub fn new_with_kanari_natives() -> Result<Self> {
        let natives = Self::get_kanari_natives_list();
        let runtime = Self::new_with_natives(natives)?;

        // Load pre-compiled Kanari system modules (skip under Miri to avoid
        // invoking verification paths that rely on stack-borrows-unsafe ops).
        if !cfg!(miri) {
            runtime.load_system_modules()?;
        }

        Ok(runtime)
    }

    /// Create runtime with Kanari system natives but using in-memory state.
    pub fn new_with_kanari_natives_in_memory() -> Result<Self> {
        let natives = Self::get_kanari_natives_list();
        let runtime = Self::new_with_natives_in_memory(natives)?;

        // Load pre-compiled Kanari system modules
        runtime.load_system_modules()?;

        Ok(runtime)
    }

    fn get_kanari_natives_list() -> Vec<NativeFunctionTable> {
        // Standard library natives at 0x1
        let std_addr = KanariAddress::std_account_address();
        let std_natives =
            move_stdlib_natives::all_natives(std_addr, move_stdlib_natives::GasParameters::zeros());

        // Kanari crypto natives at 0x2
        let system_addr = KanariAddress::kanari_system_account_address();
        let crypto_natives = kanari_system_natives::crypto::all_natives(system_addr);

        // Transfer natives at 0x2 (same address as kanari_system)
        let transfer_natives = kanari_system_natives::transfer_natives::all_natives(system_addr);

        // Event natives at 0x2 (provides `event::emit` native)
        let event_natives = kanari_system_natives::event::all_natives(system_addr);

        // TxContext natives at 0x2
        let tx_context_natives = kanari_system_natives::tx_context::all_natives(system_addr);

        // Object natives (save_object) at 0x2
        let object_natives = kanari_system_natives::object::all_natives(system_addr);

        vec![
            std_natives,
            crypto_natives,
            transfer_natives,
            event_natives,
            tx_context_natives,
            object_natives,
        ]
    }

    /// Create a new worker runtime that shares the same state/storage as this runtime
    /// but has its own independent MoveVM instance.
    /// This is crucial for parallel execution to avoid VM lock contention while sharing data.
    pub fn spawn_worker(&self) -> Result<Self> {
        // Create new VM with same natives
        // Note: We assume Kanari natives here. If custom natives were used, this might be incorrect,
        // but for now we only support Kanari natives in the pool.
        let natives = Self::get_kanari_natives_list();
        let all_natives: Vec<_> = natives
            .into_iter()
            .flat_map(|table| table.into_iter())
            .collect();

        let vm = MoveVM::new(all_natives)
            .map_err(|e| anyhow::anyhow!(format!("Worker VM init error: {:?}", e)))?;

        // Clone shared components (Arc-wrapped)
        Ok(MoveRuntime {
            vm: Arc::new(vm),
            resolver: self.resolver.clone(),
            state: self.state.clone(),
            published_modules: self.published_modules.clone(),
            object_storage: self.object_storage.clone(),
        })
    }

    /// Apply a Move VM changeset to our storage, handling module overwrites/upgrades
    /// and persisting modules to the database.
    pub(crate) fn apply_move_changeset(
        &self,
        move_cs: move_core_types::effects::ChangeSet,
    ) -> Result<()> {
        // 1. Manually apply ALL module changes.
        // This ensures upgrades/overwrites work.
        for (addr, account_changes) in move_cs.accounts() {
            for (module_name, op) in account_changes.modules() {
                if let move_core_types::effects::Op::New(bytes)
                | move_core_types::effects::Op::Modify(bytes) = op
                {
                    let module_id = ModuleId::new(
                        *addr,
                        Identifier::new(module_name.as_str())
                            .map_err(|e| anyhow::anyhow!("invalid module name: {:?}", e))?,
                    );

                    // Persist to DB and index
                    self.state.save_module(&module_id, bytes)?;
                    // ดึง Lock มาใช้ ถ้าพังก็บังคับดึงค่าออกมา (into_inner) โหนดจะไม่ดับ
                    let mut modules = self
                        .published_modules
                        .write()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    modules.insert(module_id);
                }
            }
        }

        // 2. Handle resources
        // Persist resources to support standard Move storage (borrow_global).
        for (addr, account_changes) in move_cs.accounts() {
            for (struct_tag, op) in account_changes.resources() {
                match op {
                    move_core_types::effects::Op::New(bytes)
                    | move_core_types::effects::Op::Modify(bytes) => {
                        self.state.save_resource(addr, struct_tag, bytes)?;
                    }
                    move_core_types::effects::Op::Delete => {
                        self.state.delete_resource(addr, struct_tag)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Load pre-compiled Kanari system modules from the framework package
    /// This publishes all 0x2::* modules (transfer, coin, balance, etc.) to storage
    fn load_system_modules(&self) -> Result<()> {
        // First, load move-stdlib modules (0x1::*)
        self.load_move_stdlib()?;

        // Then load Kanari system modules (0x2::*)
        self.load_kanari_system()?;

        Ok(())
    }

    /// Publish a module (bytes) with the given sender address.
    /// Returns ChangeSet containing the module addition and any resource changes from Move VM.
    pub fn publish_module(
        &self,
        module_bytes: Vec<u8>,
        sender: AccountAddress,
        // Optional gas tuple: (gas_limit, gas_price). If `Some`, runtime will
        // include gas accounting (debit sender, credit DAO) in the returned ChangeSet.
        gas_info: Option<(u64, u64)>,
        // Optional timestamp for TxContext (defaults to SystemTime::now() if None)
        _timestamp: Option<u64>,
    ) -> Result<ChangeSet> {
        self.publish_module_with_persistence(module_bytes, sender, gas_info, true)
    }

    pub fn publish_module_with_persistence(
        &self,
        module_bytes: Vec<u8>,
        sender: AccountAddress,
        gas_info: Option<(u64, u64)>,
        persist_runtime_state: bool,
    ) -> Result<ChangeSet> {
        // Deserialize to get module ID early
        let compiled = CompiledModule::deserialize_with_defaults(&module_bytes)
            .map_err(|e| anyhow::anyhow!(format!("deserialize error: {:?}", e)))?;
        let module_id = compiled.self_id();

        let (move_changeset, events) = {
            // Use resolver for session
            let mut session = self.vm.new_session(self.resolver.clone());
            let mut gas = UnmeteredGasMeter;

            session
                .publish_module(module_bytes.clone(), sender, &mut gas)
                .map_err(|e| anyhow::anyhow!(format!("publish error: {:?}", e)))?;

            let (res, _new_storage) = session.finish();
            res.map_err(|e| anyhow::anyhow!(format!("finish error: {:?}", e)))?
        };

        // Diagnostic: summarize Move VM changeset and events for debugging
        {
            let mut acct_count = 0usize;
            let mut res_count = 0usize;
            for acct in move_changeset.accounts().values() {
                acct_count += 1;
                res_count += acct.resources().len();
            }
            debug!(
                "[RUNTIME] execute_entry_function: move_changeset accounts={}, total_resources={} events={}",
                acct_count,
                res_count,
                events.len()
            );

            // Print event type summary
            for event in events.iter() {
                // event is a tuple: (key, sequence_number, type_tag, event_data)
                let (_key, _seq, type_tag, _data) = event;
                debug!("[RUNTIME] event: type={}", type_tag);
            }
        }

        // Apply changeset - if module exists, this will fail with "already exists"
        // In that case, we'll handle it as an upgrade
        if persist_runtime_state {
            self.apply_move_changeset(move_changeset.clone())?;
        }

        // Create ChangeSet from Move VM changeset
        let mut cs = ChangeSet::new();
        cs.publish_module(sender, module_id.name().to_string());

        // Parse Move VM changeset and events
        self.parse_move_changeset(&move_changeset, &mut cs);
        self.parse_move_events(&events, &mut cs);

        // If caller provided gas info, include gas accounting in the ChangeSet.
        if let Some((gas_limit, gas_price)) = gas_info {
            let gas_op = GasOperation::PublishModule {
                module_size: module_bytes.len(),
            };
            let (written, deleted) = self.calculate_storage_impact(&move_changeset, &cs);
            self.apply_gas_info(
                &mut cs,
                Some(sender),
                gas_limit,
                gas_price,
                gas_op,
                written,
                deleted,
            )?;
        }

        Ok(cs)
    }

    fn preprocess_entry_args(args: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
        args.into_iter()
            .map(|arg| {
                // แกะ BCS String (67 bytes) หรือ UTF-8 ปกติ ให้เป็น String ที่อ่านได้
                let parsed_string = bcs::from_bytes::<String>(&arg)
                    .or_else(|_| std::str::from_utf8(&arg).map(|s| s.to_string()));

                if let Ok(s) = parsed_string {
                    let s_trim = s.trim();

                    // เติม 0x ถ้ายังไม่มี
                    let hex_str = if !s_trim.starts_with("0x") {
                        format!("0x{}", s_trim)
                    } else {
                        s_trim.to_string()
                    };

                    // ถ้าสตริงนี้คือ Object ID หรือ Address ให้แปลงเป็นไบต์มาตรฐาน 32 bytes
                    // เพื่อให้ MoveVM มองเห็นและดึงข้อมูลมารันได้อย่างถูกต้อง
                    if let Ok(addr) = AccountAddress::from_hex_literal(&hex_str) {
                        return addr.into_bytes().to_vec();
                    }

                    // ถ้ารูปแบบเป็นตัวเลข (เช่น จำนวนเหรียญ amount)
                    if let Ok(n) = s_trim.parse::<u64>()
                        && let Ok(b) = bcs::to_bytes(&n)
                    {
                        return b;
                    }
                }
                arg
            })
            .collect()
    }

    fn build_tx_context_bytes(
        &self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: &[TypeTag],
        args: &[Vec<u8>],
        sender: Option<AccountAddress>,
        timestamp: Option<u64>,
        tx_hash: Option<&[u8]>,
    ) -> Result<Vec<u8>> {
        let sender_addr = sender.unwrap_or(AccountAddress::ZERO);
        let epoch_timestamp_ms = timestamp.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
        });

        let tx_hash = if let Some(raw_tx_hash) = tx_hash {
            if raw_tx_hash.len() == 32 {
                raw_tx_hash.to_vec()
            } else {
                hash_data_blake3(raw_tx_hash).to_vec()
            }
        } else {
            // Fallback for direct runtime calls (tests/examples without a chain tx):
            // derive a deterministic synthetic digest from call inputs.
            let mut input = Vec::new();
            input.extend_from_slice(b"kanari-txctx-v1");
            input.extend_from_slice(sender_addr.as_ref());
            input.extend_from_slice(&epoch_timestamp_ms.to_le_bytes());
            input.extend_from_slice(module_id.address().as_ref());
            input.extend_from_slice(module_id.name().as_str().as_bytes());
            input.extend_from_slice(function_name.as_bytes());
            input.extend_from_slice(&bcs::to_bytes(type_args)?);
            for arg in args {
                input.extend_from_slice(&(arg.len() as u64).to_le_bytes());
                input.extend_from_slice(arg);
            }
            hash_data_blake3(&input).to_vec()
        };

        let tx_ctx = TxContextRecord::from_address(sender_addr, tx_hash, 0, epoch_timestamp_ms, 0);
        bcs::to_bytes(&tx_ctx).map_err(Into::into)
    }

    fn uid_from_object_id(object_id: &str) -> Option<kanari_types::object::UIDRecord> {
        AccountAddress::from_hex_literal(object_id)
            .ok()
            .map(kanari_types::object::UIDRecord::new)
    }

    fn maybe_add_token_balance(
        &self,
        cs: &mut ChangeSet,
        owner: AccountAddress,
        type_name: &str,
        data: &[u8],
        object_id: &str,
        source: &str,
    ) {
        if let Ok(struct_tag) = type_name.parse::<move_core_types::language_storage::StructTag>()
            && self.is_balance_resource(&struct_tag)
            && let Some(amount) = self.extract_balance_from_bytes(data, &struct_tag)
            && let Some(token_type) = self.token_type_from_struct_tag(&struct_tag)
        {
            cs.add_token_balance_set(owner, token_type.clone(), amount);
            debug!(
                "[RUNTIME] Extracted balance from {} object {}: {} = {}",
                source, object_id, token_type, amount
            );
        }
    }

    fn resolve_saved_owner_and_version(
        &self,
        loaded_mutable_objects: &[LoadedMutableObject],
        object_id: &str,
    ) -> (AccountAddress, u64) {
        if let Some((_, _, owner, _, version)) = loaded_mutable_objects
            .iter()
            .find(|(_, id, _, _, _)| id == object_id)
        {
            return (*owner, *version + 1);
        }

        if let Some(stored) = self.object_storage.get_object(object_id) {
            return (stored.owner, stored.version + 1);
        }

        (AccountAddress::ZERO, 1)
    }

    fn persist_created_objects(&self, cs: &ChangeSet) {
        for (id, created) in &cs.created_objects {
            let stored = StoredObject {
                id: id.clone(),
                owner: created.owner,
                type_name: created.type_.clone(),
                data: created.data.clone(),
                version: created.version,
            };
            if let Err(e) = self.object_storage.store_object(stored) {
                log::warn!(
                    "[RUNTIME] Failed to persist object {} to internal storage: {:?}",
                    id,
                    e
                );
            } else {
                debug!(
                    "[RUNTIME] Persisted object {} (v{}) to internal storage",
                    id, created.version
                );
            }
        }
    }

    fn persist_deleted_objects(&self, cs: &ChangeSet) {
        for obj_id in &cs.deleted_objects {
            if let Err(e) = self.object_storage.delete_object(obj_id) {
                log::warn!(
                    "[RUNTIME] Failed to delete object {} from internal storage: {:?}",
                    obj_id,
                    e
                );
            } else {
                debug!("[RUNTIME] Deleted object {} from internal storage", obj_id);
            }
        }
    }

    /// Preload an object snapshot into runtime object storage.
    /// This is used by the engine to synchronize runtime object reads with
    /// canonical StateManager objects before VM execution.
    pub fn preload_object_snapshot(
        &self,
        object_id: &str,
        owner: AccountAddress,
        type_name: &str,
        data: Vec<u8>,
        version: u64,
    ) -> Result<()> {
        let stored = StoredObject {
            id: object_id.to_string(),
            owner,
            type_name: type_name.to_string(),
            data,
            version,
        };
        self.object_storage
            .store_object(stored)
            .map_err(|e| anyhow::anyhow!(format!("preload object snapshot failed: {:?}", e)))
    }

    /// Execute an entry function. `type_args` are Move `TypeTag`s and `args` are serialized
    /// arguments as Vec<u8> (Move simple-serialized values).
    /// Returns ChangeSet containing all state changes from Move VM execution.
    pub fn execute_entry_function(
        &self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        // Optional sender address. If provided along with `gas_info`, runtime will debit this sender.
        sender: Option<AccountAddress>,
        // Optional gas tuple: (gas_limit, gas_price). If provided, runtime will
        // include gas accounting (debit sender if available, credit DAO) in the returned ChangeSet.
        gas_info: Option<(u64, u64)>,
        // Optional timestamp for TxContext (defaults to SystemTime::now() if None)
        timestamp: Option<u64>,
    ) -> Result<ChangeSet> {
        self.execute_entry_function_internal(
            module_id,
            function_name,
            type_args,
            args,
            sender,
            gas_info,
            timestamp,
            None,
            true,
        )
    }

    /// Execute entry function with optional canonical transaction hash.
    /// When provided, `tx_hash` is used as TxContext digest source to ensure object IDs
    /// are unique per transaction.
    pub fn execute_entry_function_with_tx_hash(
        &self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        sender: Option<AccountAddress>,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
    ) -> Result<ChangeSet> {
        self.execute_entry_function_internal(
            module_id,
            function_name,
            type_args,
            args,
            sender,
            gas_info,
            timestamp,
            tx_hash,
            true,
        )
    }

    pub fn execute_entry_function_with_tx_hash_and_persistence(
        &self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        sender: Option<AccountAddress>,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
        persist_runtime_state: bool,
    ) -> Result<ChangeSet> {
        self.execute_entry_function_internal(
            module_id,
            function_name,
            type_args,
            args,
            sender,
            gas_info,
            timestamp,
            tx_hash,
            persist_runtime_state,
        )
    }

    fn execute_entry_function_internal(
        &self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        sender: Option<AccountAddress>,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
        persist_runtime_state: bool,
    ) -> Result<ChangeSet> {
        // Use resolver for session
        let mut session = self.vm.new_session(self.resolver.clone());
        let mut gas = UnmeteredGasMeter;

        // 🚨 1. ตัวแปรความปลอดภัย ป้องกัน DDoS & Auto-Merge
        let mut auto_merged_coin_ids = Vec::new();
        let mut merged_coin_types = std::collections::HashSet::new();
        let mut total_merge_reads: u64 = 0; // เก็บค่าเหนื่อยในการกวาดเหรียญ
        let mut synthetic_events = Vec::new(); // <--- 🚨 เพิ่มตะกร้าเก็บใบเสร็จ

        // convert type tags to VM runtime types
        let mut ty_args_loaded = vec![];
        for tag in type_args.iter() {
            let ty = session
                .load_type(tag)
                .map_err(|e| anyhow::anyhow!(format!("load type error: {:?}", e)))?;
            ty_args_loaded.push(ty);
        }

        let ident = IdentStr::new(function_name).map_err(|e| anyhow::anyhow!(e.to_string()))?;

        // Auto-inject TxContext if function expects it as last parameter
        let mut final_args = Self::preprocess_entry_args(args);
        let tx_context_bytes = self.build_tx_context_bytes(
            module_id,
            function_name,
            &type_args,
            &final_args,
            sender,
            timestamp,
            tx_hash.as_deref(),
        )?;

        // Track loaded mutable objects for writeback: (arg_index, object_id, owner, type_name, version)
        let mut loaded_mutable_objects: Vec<LoadedMutableObject> = Vec::new();

        // Conditionally add TxContext as last argument if the function expects one.
        // Also perform Object Loading: if a parameter is a Struct and the argument is an ID,
        // load the object from storage.
        if let Ok(func) = session.load_function(module_id, ident, &ty_args_loaded) {
            let type_tag_for_param = |param_type: &RuntimeType| {
                session
                    .get_type_tag(param_type)
                    .ok()
                    .or_else(|| match param_type {
                        RuntimeType::Reference(inner) | RuntimeType::MutableReference(inner) => {
                            session.get_type_tag(inner).ok()
                        }
                        _ => None,
                    })
            };

            // 1. Object Loading
            for (i, param_type) in func.parameters.iter().enumerate() {
                if i >= final_args.len() {
                    break;
                }

                // Check if this parameter expects a Struct (or Reference to Struct)
                // and if the provided argument is a potential Object ID (32 bytes).
                let is_potential_id = final_args[i].len() == 32;

                if is_potential_id
                    && let Some(TypeTag::Struct(struct_tag)) = type_tag_for_param(param_type)
                {
                    // It expects a Struct, and we have 32 bytes. Try to load as Object.
                    let object_id = format!("0x{}", hex::encode(&final_args[i]));

                    // Try to fetch from ObjectStorage
                    if let Some(mut stored_obj) = self.object_storage.get_object(&object_id) {
                        // --- 🚨 OWNERSHIP VERIFICATION GUARD 🚨 ---
                        if let Some(s_addr) = sender {
                            let sys_addr = KanariAddress::kanari_system_account_address();
                            let std_addr = KanariAddress::std_account_address();
                            if stored_obj.owner != s_addr
                                && stored_obj.owner != AccountAddress::ZERO
                                && stored_obj.owner != sys_addr
                                && stored_obj.owner != std_addr
                            {
                                return Err(anyhow::anyhow!("Ownership verification failed"));
                            }
                        }

                        // 🚨 --- [ANTI-DDOS AUTO-MERGE LOGIC] กวาดเหรียญมารวมอัตโนมัติ --- 🚨
                        let is_coin = struct_tag.module.as_str() == "coin"
                            && struct_tag.name.as_str() == "Coin";
                        if is_coin
                            && let Some(s_addr) = sender
                            && let Some(coin_t) = struct_tag.type_params.first()
                        {
                            // ป้องกันการ Merge ซ้ำซ้อนถ้ามีหลาย Parameter
                            let merge_key = format!("{:?}_{}", s_addr, coin_t);
                            if !merged_coin_types.contains(&merge_key) {
                                merged_coin_types.insert(merge_key);

                                // 🚨 แก้ข้อ 3: จำกัดโควต้าดึงเหรียญสูงสุดแค่ 200 ก้อน ป้องกัน Node ค้าง
                                let all_coins: Vec<_> = self
                                    .object_storage
                                    .get_coins_by_type_and_owner(s_addr, coin_t)
                                    .into_iter()
                                    .take(200)
                                    .collect();

                                // 🚨 จดบันทึกว่าโหนดทำงานไปกี่ก้อน เพื่อเอาไปคิดค่า Gas ย้อนหลัง
                                total_merge_reads += all_coins.len() as u64;

                                if all_coins.len() > 1 {
                                    log::debug!(
                                        "[RUNTIME] Auto-merging {} coins for sender",
                                        all_coins.len()
                                    );

                                    let mut total_balance: u64 = 0;
                                    // 🚨 แก้ข้อ 1: สร้างคิวเก็บเฉพาะ ID ที่บวกเงินสำเร็จเท่านั้น
                                    let mut successfully_merged_ids = Vec::new();

                                    for c in &all_coins {
                                        if c.data.len() == 40 {
                                            let mut bal_bytes = [0u8; 8];
                                            bal_bytes.copy_from_slice(&c.data[32..40]);

                                            // 🚨 ใช้ checked_add ป้องกันบั๊กเงินล้น (Overflow)
                                            if let Some(new_total) = total_balance
                                                .checked_add(u64::from_le_bytes(bal_bytes))
                                            {
                                                total_balance = new_total;

                                                if c.id != stored_obj.id {
                                                    successfully_merged_ids.push(c.id.clone());
                                                }
                                            }
                                        } else {
                                            // ถ้าเจอเหรียญพัง ให้ข้ามไปเลย ไม่เอาไปลบ
                                            log::warn!(
                                                "[RUNTIME] Skipped coin with invalid size: {}",
                                                c.id
                                            );
                                        }
                                    }

                                    // บันทึกยอดรวมกลับเข้าไปที่ก้อนหลัก
                                    if stored_obj.data.len() == 40 {
                                        let new_bal_bytes = total_balance.to_le_bytes();
                                        stored_obj.data[32..40].copy_from_slice(&new_bal_bytes);
                                    }

                                    // เอา ID ที่บวกผ่านแล้ว โยนใส่ตัวแปรที่รอคำสั่งลบ
                                    for id in &successfully_merged_ids {
                                        // <--- เติม & ตรงนี้เพื่อขอยืม
                                        auto_merged_coin_ids.push(id.clone()); // <--- เติม .clone() เพื่อก๊อปปี้ค่า
                                    }
                                    // 🚨 2. สร้างใบเสร็จใส่ตะกร้า (Synthetic Event)
                                    let receipt_data = AutoMergeReceiptData {
                                        owner: s_addr,
                                        merged_count: successfully_merged_ids.len() as u64,
                                        total_amount: total_balance,
                                    };

                                    // แปลงใบเสร็จเป็นไบต์ (BCS) และสร้าง TypeTag แจ้ง Explorer
                                    if let Ok(event_bytes) = bcs::to_bytes(&receipt_data) {
                                        let event_type = TypeTag::Struct(Box::new(StructTag {
                                            address: KanariAddress::kanari_system_account_address(), // 0x2
                                            module: Identifier::new("system_events").unwrap(),
                                            name: Identifier::new("AutoMergeReceipt").unwrap(),
                                            // ระบุว่าเหรียญที่รวมคือเหรียญอะไร (เช่น <0x2::james::JAMES>)
                                            type_params: vec![TypeTag::Struct(struct_tag.clone())],
                                        }));

                                        synthetic_events.push((event_type, event_bytes));
                                    }
                                }
                            }
                        }
                        // 🚨 --------------------------------------------------------

                        debug!("[RUNTIME] Loaded object {} for param {}", object_id, i);
                        final_args[i] = stored_obj.data.clone();

                        // 🚨 (ส่วนที่หายไปรอบที่แล้ว ต้องมีตรงนี้นะครับ!)
                        // If this parameter is a mutable reference (&mut T),
                        // modifications made by the entry function need to be persisted.
                        if let RuntimeType::MutableReference(_) = param_type {
                            loaded_mutable_objects.push((
                                i,
                                stored_obj.id.clone(),
                                stored_obj.owner,
                                stored_obj.type_name.clone(),
                                stored_obj.version,
                            ));
                            debug!(
                                "[RUNTIME] Tracking mutable object {} for writeback",
                                object_id
                            );
                        }
                    }
                }
            }

            // 2. TxContext Injection
            let param_count = func.parameters.len();
            if param_count == final_args.len() + 1 {
                // Check if last parameter is TxContext type
                if let Some(last_param_type) = func.parameters.last() {
                    // Use the session loader to convert the runtime `Type` into a `TypeTag` and
                    // check whether the last parameter is the canonical `0x2::tx_context::TxContext`.
                    if let Some(TypeTag::Struct(struct_tag)) = type_tag_for_param(last_param_type) {
                        let system_addr = KanariAddress::kanari_system_account_address();
                        if struct_tag.address == system_addr
                            && struct_tag.module.as_str() == TxContextModule::TX_CONTEXT_MODULE
                            && struct_tag.name.as_str() == TxContextModule::TX_CONTEXT_STRUCT
                        {
                            final_args.push(tx_context_bytes);
                            debug!(
                                "[RUNTIME] Auto-injected TxContext for {}::{}",
                                module_id, function_name
                            );
                        }
                    }
                }
            }
        }

        // Register native-extensions at the session so native functions can
        // record data (transferred objects, emitted events, etc.). The Move
        // VM will pass this extension container to `NativeContext` during
        // native function execution.
        use kanari_system_natives::event::EventsExt;
        use kanari_system_natives::object::DeletedObjectsExt;
        use kanari_system_natives::object::SavedObjectsExt;
        use kanari_system_natives::transfer_natives::TransferredObjectsExt;

        let exts = session.get_native_extensions();
        exts.add(TransferredObjectsExt::default());
        exts.add(EventsExt::default());
        exts.add(SavedObjectsExt::default());
        exts.add(DeletedObjectsExt::default());

        // 🚨 สั่ง Execute และดักจับความล้มเหลวเพื่อหักค่า Gas
        let execution_result =
            session.execute_entry_function(module_id, ident, ty_args_loaded, final_args, &mut gas);

        let mut cs = ChangeSet::new();

        match execution_result {
            Ok(return_values) => {
                // 🟢 กรณีสำเร็จ (Success Path)
                let (transferred, captured_events, saved_objects, deleted_objects) = {
                    let exts_after = session.get_native_extensions();
                    let trans = exts_after.get_mut::<TransferredObjectsExt>().take_all();
                    let evs = exts_after.get_mut::<EventsExt>().take_all();
                    let saved = exts_after.get_mut::<SavedObjectsExt>().take_all();
                    let deleted = exts_after.get_mut::<DeletedObjectsExt>().take_all();
                    (trans, evs, saved, deleted)
                };

                let (res, _new_storage) = session.finish();
                let (move_changeset, events) =
                    res.map_err(|e| anyhow::anyhow!(format!("exec error: {:?}", e)))?;

                if persist_runtime_state {
                    self.apply_move_changeset(move_changeset.clone())?;
                }

                self.parse_move_changeset(&move_changeset, &mut cs);
                self.parse_move_events(&events, &mut cs);

                let mut processed_ids = std::collections::HashSet::new();

                for (idx, data, _layout) in return_values.mutable_reference_outputs {
                    if let Some((_, id, owner, type_name, version)) = loaded_mutable_objects
                        .iter()
                        .find(|(i, _, _, _, _)| *i == idx as usize)
                    {
                        let uid = Self::uid_from_object_id(id);
                        let updated_obj = crate::changeset::CreatedObject {
                            owner: *owner,
                            uid,
                            type_: type_name.clone(),
                            data: data.clone(),
                            version: version + 1,
                        };

                        self.maybe_add_token_balance(
                            &mut cs,
                            *owner,
                            type_name,
                            &data,
                            id,
                            "writeback",
                        );
                        cs.created_objects.push((id.clone(), updated_obj));
                        processed_ids.insert(id.clone());
                    }
                }

                for saved in saved_objects {
                    if processed_ids.contains(&saved.object_id) {
                        continue;
                    }

                    let (owner, version) = self
                        .resolve_saved_owner_and_version(&loaded_mutable_objects, &saved.object_id);

                    let uid = Self::uid_from_object_id(&saved.object_id);
                    let saved_data_clone = saved.data.clone();
                    let saved_type_clone = saved.object_type.clone();

                    let updated_obj = crate::changeset::CreatedObject {
                        owner,
                        uid,
                        type_: saved.object_type,
                        data: saved.data,
                        version,
                    };

                    self.maybe_add_token_balance(
                        &mut cs,
                        owner,
                        &saved_type_clone,
                        &saved_data_clone,
                        &saved.object_id,
                        "saved",
                    );

                    cs.created_objects
                        .push((saved.object_id.clone(), updated_obj));
                    processed_ids.insert(saved.object_id);
                }

                self.add_transferred_objects_to_changeset(&mut cs, transferred);

                for ev in captured_events.into_iter() {
                    let ev_rec = Event {
                        key: ev.key,
                        sequence_number: ev.sequence_number,
                        type_tag: ev.type_tag,
                        event_data: ev.event_data,
                    };
                    cs.add_event(ev_rec);
                }

                for deleted_obj in deleted_objects {
                    cs.add_deleted_object(deleted_obj.object_id);
                }

                for merged_id in auto_merged_coin_ids {
                    cs.add_deleted_object(merged_id);
                }

                // 🚨 3. ยิง Event ใบเสร็จออกไปให้ Block Explorer รับทราบ
                for (evt_type, evt_data) in synthetic_events {
                    let ev_rec = Event {
                        key: Default::default(), // ใช้ Default key (หลอกระบบว่าเป็น System Event)
                        sequence_number: 0,
                        type_tag: evt_type.to_string(),
                        event_data: evt_data,
                    };
                    cs.add_event(ev_rec);
                }

                // 🚨 หักค่า Gas กรณีสำเร็จ (คิดรวมความเหนื่อยในการ Merge เหรียญ)
                if let Some((gas_limit, gas_price)) = gas_info {
                    let complexity = 1 + (total_merge_reads as u32 / 10);
                    let gas_op = GasOperation::ExecuteFunction { complexity };
                    let (written, deleted) = self.calculate_storage_impact(&move_changeset, &cs);
                    self.apply_gas_info(
                        &mut cs, sender, gas_limit, gas_price, gas_op, written, deleted,
                    )?;
                }

                if persist_runtime_state {
                    self.persist_created_objects(&cs);
                    self.persist_deleted_objects(&cs);
                }

                Ok(cs)
            }
            Err(e) => {
                // 🔴 กรณีล้มเหลว (Security Guard Path)
                // ทิ้ง Transaction แต่หักเงินค่า Gas อย่างหนักเพื่อลงโทษแฮ็กเกอร์
                log::error!(
                    "[SECURITY GUARD] Execution aborted! Applying Gas Penalty. Error: {}",
                    e
                );

                if let Some((gas_limit, gas_price)) = gas_info {
                    // ปรับความยากขึ้นเป็นทวีคูณถ้ายิงขยะเข้ามา + คิดค่าอ่าน DB
                    let penalty_complexity = 5 + (total_merge_reads as u32);
                    let gas_op = GasOperation::ExecuteFunction {
                        complexity: penalty_complexity,
                    };

                    if let Err(gas_err) =
                        self.apply_gas_info(&mut cs, sender, gas_limit, gas_price, gas_op, 0, 0)
                    {
                        log::error!("Failed to deduct penalty gas: {}", gas_err);
                    }

                    if persist_runtime_state {
                        self.persist_created_objects(&cs);
                    }
                }

                Err(anyhow::anyhow!(format!("exec error: {:?}", e)))
            }
        }
    }
}
