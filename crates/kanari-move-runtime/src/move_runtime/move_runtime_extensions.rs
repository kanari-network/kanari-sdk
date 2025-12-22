// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Extended functionality for MoveRuntime
// Includes module verification and advanced session management
use anyhow::Result;
use bcs;
use kanari_types::address::Address as KanariAddress;
use kanari_types::tx_context::TxContextRecord;
use move_binary_format::file_format::CompiledModule;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::{ModuleId, TypeTag};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;
use std::time::SystemTime;

use crate::changeset::ChangeSet;
use crate::move_runtime::MoveRuntime;

impl MoveRuntime {
    /// Verify a compiled module before publishing
    /// Checks basic invariants and dependencies
    pub fn verify_module(&self, module: &CompiledModule) -> Result<()> {
        // Basic verification checks

        // 1. Check module has valid self-id
        let module_id = module.self_id();
        if module_id.name().as_str().is_empty() {
            anyhow::bail!("Module has empty name");
        }

        // 2. Check all dependencies are available
        for dep in module.immediate_dependencies() {
            if !self.has_module(&dep) {
                // Allow dependencies on stdlib (0x1) and system (0x2)
                let addr = dep.address();
                if addr != &AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap()
                    && addr
                        != &AccountAddress::from_hex_literal(KanariAddress::KANARI_SYSTEM_ADDRESS)
                            .unwrap()
                {
                    anyhow::bail!(
                        "Missing dependency: {}::{}",
                        addr.short_str_lossless(),
                        dep.name()
                    );
                }
            }
        }

        // 3. Check module doesn't exceed size limits (use serialized bytecode size)
        let mut module_bytes: Vec<u8> = vec![];
        move_binary_format::file_format::CompiledModule::serialize(module, &mut module_bytes)?;
        if module_bytes.len() > 1_000_000 {
            anyhow::bail!("Module too large: {} bytes", module_bytes.len());
        }

        Ok(())
    }

    /// Check if a module is available in storage
    pub fn has_module(&self, module_id: &ModuleId) -> bool {
        // InMemoryStorage doesn't expose get_module directly
        // We check by assuming stdlib/system modules are always available
        let addr = module_id.address();
        if addr == &AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap()
            || addr
                == &AccountAddress::from_hex_literal(KanariAddress::KANARI_SYSTEM_ADDRESS).unwrap()
        {
            return true;
        }
        // For other modules, check the runtime's published_modules index
        self.published_modules.contains(module_id)
    }

    /// Get the current storage state (for debugging/inspection)
    pub fn get_storage(&self) -> &InMemoryStorage {
        &self.storage
    }

    /// Get module bytecode if available
    pub fn get_module_bytes(&self, module_id: &ModuleId) -> Option<Vec<u8>> {
        // Query from persistent storage
        self.state.get_module(module_id)
    }

    /// List all published modules in storage
    pub fn list_modules(&self) -> Vec<ModuleId> {
        // Return modules from our maintained index
        self.published_modules.iter().cloned().collect()
    }

    /// Get a reference to the storage for direct queries
    /// This allows advanced users to query the storage directly
    pub fn storage_ref(&self) -> &InMemoryStorage {
        &self.storage
    }

    /// Execute a function in a sandboxed session and return result without committing
    /// Useful for simulation and gas estimation
    pub fn simulate_entry_function(
        &self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> Result<ChangeSet> {
        let storage_clone = self.storage.clone();
        let mut session = self.vm.new_session(storage_clone);
        let mut gas = UnmeteredGasMeter;

        let mut ty_args_loaded = vec![];
        for tag in type_args.iter() {
            let ty = session
                .load_type(tag)
                .map_err(|e| anyhow::anyhow!(format!("load type error: {:?}", e)))?;
            ty_args_loaded.push(ty);
        }

        let ident = move_core_types::identifier::IdentStr::new(function_name)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        // Inject TxContext for simulation when the target function expects it
        let mut final_args = args.clone();

        let sender_addr = AccountAddress::ZERO; // use ZERO for simulation
        let tx_hash = vec![0u8; 32];
        let epoch = 0u64;
        let epoch_timestamp_ms = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let ids_created = 0u64;

        let tx_ctx = TxContextRecord::from_address(
            sender_addr,
            tx_hash,
            epoch,
            epoch_timestamp_ms,
            ids_created,
        );
        let tx_context_bytes = bcs::to_bytes(&tx_ctx)?;

        if let Ok(func) = session.load_function(module_id, ident, &ty_args_loaded) {
            let param_count = func.parameters.len();
            if param_count == final_args.len() + 1 {
                final_args.push(tx_context_bytes);
            }
        }

        session
            .execute_entry_function(module_id, ident, ty_args_loaded, final_args, &mut gas)
            .map_err(|e| anyhow::anyhow!(format!("exec error: {:?}", e)))?;

        let (res, _new_storage) = session.finish();
        let (move_changeset, events) =
            res.map_err(|e| anyhow::anyhow!(format!("finish error: {:?}", e)))?;

        // Note: We don't apply changes - this is simulation only
        let mut cs = ChangeSet::new();
        self.parse_move_changeset(&move_changeset, &mut cs);
        self.parse_move_events(&events, &mut cs);

        Ok(cs)
    }
}

/// Runtime statistics and configuration
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    pub gas_metering_enabled: bool,
}
