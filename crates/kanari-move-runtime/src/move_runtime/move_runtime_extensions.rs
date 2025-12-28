// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Extended functionality for MoveRuntime
// Includes module verification and advanced session management
use anyhow::Result;
use kanari_types::address::Address as KanariAddress;
use move_binary_format::file_format::CompiledModule;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::ModuleId;
use move_vm_test_utils::InMemoryStorage;

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
}

/// Runtime statistics and configuration
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    pub gas_metering_enabled: bool,
}
