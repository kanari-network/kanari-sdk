// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// This file contains the MoveRuntime wrapper implementation.
// It utilizes MoveVM and InMemoryStorage for executing functions and publishing modules.
// Enhanced with native function support, gas metering, and session management.
use anyhow::Result;
use log::debug;
use move_binary_format::file_format::CompiledModule;
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::IdentStr;
use move_core_types::language_storage::{ModuleId, TypeTag};
use move_vm_runtime::move_vm::MoveVM;
use move_vm_runtime::native_functions::NativeFunctionTable;
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;
mod gas_ops;
mod helpers;
mod load_system_modules;
mod object_ops;
mod parsers;
use crate::gas::GasOperation;
use kanari_types::address::Address as KanariAddress;
pub mod move_runtime_extensions;
use crate::changeset::ChangeSet;
use crate::storage::move_vm_state::MoveVMState;
use crate::storage::object_storage::{ObjectStorage, ObjectStore};
use kanari_types::tx_context::TxContextRecord;

use std::collections::HashSet;

/// Enhanced runtime wrapper around `move-vm` for executing functions, publishing modules,
/// and running scripts. Supports custom native functions, gas metering, and atomic sessions.
pub struct MoveRuntime {
    pub(crate) vm: MoveVM,
    pub(crate) storage: InMemoryStorage,
    pub(crate) state: MoveVMState,
    /// Whether to use gas metering in VM sessions
    pub(crate) enable_gas_metering: bool,
    /// Index of published modules for faster listing
    pub(crate) published_modules: HashSet<ModuleId>,
    /// Persistent object storage for transferred objects
    pub(crate) object_storage: Box<dyn ObjectStore>,
}

impl MoveRuntime {
    /// Open the runtime using the default persistent DB path (see README).
    /// Initializes VM without custom natives (for basic usage).
    pub fn new() -> Result<Self> {
        Self::new_with_natives(vec![], false)
    }

    /// Create a new MoveRuntime with custom native functions.
    ///
    /// # Arguments
    /// * `natives` - Native function tables for custom functions (e.g., crypto operations)
    /// * `enable_gas_metering` - Whether to enable gas metering in VM sessions
    ///
    /// # Example
    /// ```ignore
    /// use kanari_crypto::move_natives;
    /// use move_core_types::account_address::AccountAddress;
    ///
    /// let system_addr = AccountAddress::from_hex_literal("0x2").unwrap();
    /// let natives = move_natives::all_natives(system_addr);
    /// let runtime = MoveRuntime::new_with_natives(vec![natives], true)?;
    /// ```
    pub fn new_with_natives(
        natives: Vec<NativeFunctionTable>,
        enable_gas_metering: bool,
    ) -> Result<Self> {
        let state = MoveVMState::open_default()?;
        let mut storage = InMemoryStorage::new();
        state.load_into_storage(&mut storage)?;

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

        Ok(MoveRuntime {
            vm,
            storage,
            state,
            enable_gas_metering,
            published_modules: HashSet::new(),
            object_storage: ObjectStorage::boxed_inmemory(),
        })
    }

    /// Create runtime with Kanari system natives (crypto + stdlib + object + tx_context)
    /// Also loads pre-compiled Kanari system modules (0x2::*)
    pub fn new_with_kanari_natives() -> Result<Self> {
        // Standard library natives at 0x1
        let std_addr = KanariAddress::std_account_address();
        let std_natives =
            move_stdlib_natives::all_natives(std_addr, move_stdlib_natives::GasParameters::zeros());

        // Kanari crypto natives at 0x2
        let system_addr = KanariAddress::kanari_system_account_address();
        let crypto_natives = kanari_system_natives::crypto::all_natives(system_addr);

        // Transfer natives at 0x2 (same address as kanari_system)
        let transfer_natives = kanari_system_natives::transfer_natives::all_natives(system_addr);

        // Create runtime with natives
        let mut runtime =
            Self::new_with_natives(vec![std_natives, crypto_natives, transfer_natives], true)?;

        // Load pre-compiled Kanari system modules
        runtime.load_system_modules()?;

        Ok(runtime)
    }

    /// Load pre-compiled Kanari system modules from the framework package
    /// This publishes all 0x2::* modules (transfer, coin, balance, etc.) to storage
    fn load_system_modules(&mut self) -> Result<()> {
        // First, load move-stdlib modules (0x1::*)
        self.load_move_stdlib()?;

        // Then load Kanari system modules (0x2::*)
        self.load_kanari_system()?;

        Ok(())
    }

    /// Publish a module (bytes) with the given sender address.
    /// Returns ChangeSet containing the module addition and any resource changes from Move VM.
    pub fn publish_module(
        &mut self,
        module_bytes: Vec<u8>,
        sender: AccountAddress,
        // Optional gas tuple: (gas_limit, gas_price). If `Some`, runtime will
        // include gas accounting (debit sender, credit DAO) in the returned ChangeSet.
        gas_info: Option<(u64, u64)>,
    ) -> Result<ChangeSet> {
        // Deserialize to get module ID early
        let compiled = CompiledModule::deserialize_with_defaults(&module_bytes)
            .map_err(|e| anyhow::anyhow!(format!("deserialize error: {:?}", e)))?;
        let module_id = compiled.self_id();

        let storage_clone = self.storage.clone();
        let mut session = self.vm.new_session(storage_clone);
        let mut gas = UnmeteredGasMeter;

        session
            .publish_module(module_bytes.clone(), sender, &mut gas)
            .map_err(|e| anyhow::anyhow!(format!("publish error: {:?}", e)))?;

        let (res, new_storage) = session.finish();
        let (move_changeset, events) =
            res.map_err(|e| anyhow::anyhow!(format!("finish error: {:?}", e)))?;

        // Diagnostic: summarize Move VM changeset and events for debugging
        {
            let mut acct_count = 0usize;
            let mut res_count = 0usize;
            for (_addr, acct) in move_changeset.accounts() {
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
        let mut storage = new_storage;
        let apply_result = storage.apply(move_changeset.clone());

        match apply_result {
            Ok(_) => {
                // New module published successfully
                self.storage = storage.clone();
                self.published_modules.insert(module_id.clone());
            }
            Err(e) => {
                let err_msg = format!("{:?}", e);
                if err_msg.contains("already exists") {
                    // Module upgrade - use publish_or_overwrite instead of apply
                    self.storage
                        .publish_or_overwrite_module(module_id.clone(), module_bytes.clone());
                    self.published_modules.insert(module_id.clone());
                } else {
                    return Err(anyhow::anyhow!(format!("apply error: {:?}", e)));
                }
            }
        }

        // persist module bytes to DB (overwrite if exists for upgrades)
        self.state.save_module(&module_id, &module_bytes)?;

        // Create ChangeSet from Move VM changeset
        let mut cs = ChangeSet::new();
        cs.publish_module(sender, module_id.name().to_string());

        // If caller provided gas info, include gas accounting in the ChangeSet.
        if let Some((gas_limit, gas_price)) = gas_info {
            let gas_op = GasOperation::PublishModule {
                module_size: module_bytes.len(),
            };
            self.apply_gas_info(&mut cs, Some(sender), gas_limit, gas_price, gas_op)?;
        }

        // Parse Move VM changeset and events
        self.parse_move_changeset(&move_changeset, &mut cs);
        self.parse_move_events(&events, &mut cs);

        Ok(cs)
    }

    /// Execute an entry function. `type_args` are Move `TypeTag`s and `args` are serialized
    /// arguments as Vec<u8> (Move simple-serialized values).
    /// Returns ChangeSet containing all state changes from Move VM execution.
    pub fn execute_entry_function(
        &mut self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        // Optional sender address. If provided along with `gas_info`, runtime will debit this sender.
        sender: Option<AccountAddress>,
        // Optional gas tuple: (gas_limit, gas_price). If provided, runtime will
        // include gas accounting (debit sender if available, credit DAO) in the returned ChangeSet.
        gas_info: Option<(u64, u64)>,
    ) -> Result<ChangeSet> {
        let storage_clone = self.storage.clone();
        let mut session = self.vm.new_session(storage_clone);
        let mut gas = UnmeteredGasMeter;

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
        let mut final_args = args.clone();

        // Create TxContext struct using canonical Kanari type and serialize with BCS
        let sender_addr = sender.unwrap_or(AccountAddress::ZERO);
        let tx_hash = vec![0u8; 32]; // Placeholder
        let epoch = 0u64;
        let epoch_timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let ids_created = 0u64;

        let tx_ctx = TxContextRecord::from_address(
            sender_addr,
            tx_hash.clone(),
            epoch,
            epoch_timestamp_ms,
            ids_created,
        );

        let tx_context_bytes = bcs::to_bytes(&tx_ctx)?;

        // Conditionally add TxContext as last argument if the function expects one.
        // We load the function signature and verify:
        // 1. Parameter count matches args + 1
        // 2. Last parameter type is EXACTLY TxContext (0x2::tx_context::TxContext)
        if let Ok(func) = session.load_function(module_id, ident, &ty_args_loaded) {
            let param_count = func.parameters.len();
            if param_count == final_args.len() + 1 {
                // Check if last parameter is TxContext type
                if let Some(last_param_type) = func.parameters.last() {
                    let type_str = format!("{:?}", last_param_type);
                    // Strict matching: must contain both module path and struct name
                    // Format is typically: Struct(StructTag { address: 0x2, module: tx_context, name: TxContext, ... })
                    let is_tx_context = type_str.contains("0x2")
                        && type_str.contains("module: tx_context")
                        && type_str.contains("name: TxContext");

                    if is_tx_context {
                        final_args.push(tx_context_bytes);
                        debug!(
                            "[RUNTIME] Auto-injected TxContext for {}::{}",
                            module_id, function_name
                        );
                    } else {
                        debug!(
                            "[RUNTIME] Skipped TxContext injection for {}::{} - last param type: {}",
                            module_id, function_name, type_str
                        );
                    }
                }
            }
        }

        // Register the transferred-object extension at the session's
        // native-extensions container so native functions can record into it.
        // The Move VM will pass this extension to NativeContext.extensions_mut()
        // during native function execution.
        use kanari_system_natives::transfer_natives::TransferredObjectsExt;
        let exts = session.get_native_extensions();
        exts.add(TransferredObjectsExt::default());

        session
            .execute_entry_function(module_id, ident, ty_args_loaded, final_args, &mut gas)
            .map_err(|e| anyhow::anyhow!(format!("exec error: {:?}", e)))?;

        // After execution, collect transferred objects from the native-extensions
        // container before consuming the session with `finish()`.
        let transferred = {
            let exts_after = session.get_native_extensions();
            exts_after.get_mut::<TransferredObjectsExt>().take_all()
        };

        let (res, new_storage) = session.finish();
        let (move_changeset, events) =
            res.map_err(|e| anyhow::anyhow!(format!("finish error: {:?}", e)))?;

        let mut storage = new_storage;
        storage
            .apply(move_changeset.clone())
            .map_err(|e| anyhow::anyhow!(format!("apply error: {:?}", e)))?;

        self.storage = storage;

        // Create ChangeSet from Move VM execution
        let mut cs = ChangeSet::new();

        // Parse Move VM changeset and events
        self.parse_move_changeset(&move_changeset, &mut cs);
        self.parse_move_events(&events, &mut cs);

        // Add transferred objects collected from the native extension
        self.add_transferred_objects_to_changeset(&mut cs, transferred);

        // If gas accounting requested, include gas debit/credit in ChangeSet.
        if let Some((gas_limit, gas_price)) = gas_info {
            let gas_op = GasOperation::ExecuteFunction { complexity: 1 };
            self.apply_gas_info(&mut cs, sender, gas_limit, gas_price, gas_op)?;
        }

        Ok(cs)
    }
}
