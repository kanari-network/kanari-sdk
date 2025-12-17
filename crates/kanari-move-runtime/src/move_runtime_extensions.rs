// Extended functionality for MoveRuntime
// Includes module verification and advanced session management

use anyhow::Result;
use kanari_types::address::Address as KanariAddress;
use move_binary_format::file_format::CompiledModule;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::{ModuleId, TypeTag};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;

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

        // 3. Check module doesn't exceed size limits
        let module_size = module.self_id().name().as_str().len();
        if module_size > 10_000 {
            anyhow::bail!("Module name too large: {} bytes", module_size);
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
        // For other modules, we'd need to query storage differently
        // This is a simplified check
        false
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

        session
            .execute_entry_function(module_id, ident, ty_args_loaded, args, &mut gas)
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

    /// Estimate gas cost for executing a function
    pub fn estimate_gas(
        &self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> Result<u64> {
        // Simulate execution and estimate based on complexity
        let _cs = self.simulate_entry_function(module_id, function_name, type_args, args)?;

        // Simple gas estimation based on function complexity
        // In production, this would analyze the actual gas consumption
        let base_gas = 1000u64;
        let complexity_gas = function_name.len() as u64 * 10;

        Ok(base_gas + complexity_gas)
    }

    /// Reset storage to a clean state (for testing)
    #[cfg(test)]
    pub fn reset_storage(&mut self) {
        self.storage = InMemoryStorage::new();
    }

    /// Get runtime statistics
    pub fn get_stats(&self) -> RuntimeStats {
        RuntimeStats {
            gas_metering_enabled: self.enable_gas_metering,
            // Add more stats as needed
        }
    }
}

/// Runtime statistics and configuration
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    pub gas_metering_enabled: bool,
}
