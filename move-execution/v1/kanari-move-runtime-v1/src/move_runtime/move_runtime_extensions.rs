// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Extension traits for the Move runtime.

// Extended functionality for MoveRuntime
// Includes module verification and advanced session management
// This module provides additional functionality for the MoveRuntime, including module verification before publishing, checking for module availability in storage, retrieving module bytecode, and listing all published modules. It also includes advanced session management features that allow for more complex transaction execution scenarios.
// The functions in this module are used internally by the MoveRuntime to ensure that modules are valid and that the runtime state is consistent during transaction execution. It also provides mechanisms for managing the lifecycle of modules and ensuring that they can be upgraded or published with the appropriate context and persistence.
// The module is designed to work with the Kanari Move runtime and integrates with the ChangeSet and StateOverlay types to provide a comprehensive solution for managing Move modules and their associated state.
use anyhow::Result;
use kanari_types::address::Address as KanariAddress;
use move_binary_format::file_format::CompiledModule;
use move_core_types::language_storage::ModuleId;

use crate::move_runtime::MoveRuntime;

impl MoveRuntime {
    /// Verify a compiled module before publishing
    /// Checks basic invariants and dependencies
    pub(crate) fn verify_module(&self, module: &CompiledModule) -> Result<()> {
        // Basic verification checks

        // 1. Check module has valid self-id
        let module_id = module.self_id();
        if module_id.name().as_str().is_empty() {
            anyhow::bail!("Module has empty name");
        }

        // 2. Check all dependencies are available
        for dep in module.immediate_dependencies() {
            if !self.has_module(&dep)? {
                // Allow dependencies on stdlib (0x1) and system (0x2)
                let addr = dep.address();
                if addr != &KanariAddress::std_account_address()
                    && addr != &KanariAddress::kanari_system_account_address()
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
    fn has_module(&self, module_id: &ModuleId) -> Result<bool> {
        // Check by assuming stdlib/system modules are always available
        let addr = module_id.address();
        if addr == &KanariAddress::std_account_address()
            || addr == &KanariAddress::kanari_system_account_address()
        {
            return Ok(true);
        }

        // Check if module exists in persistent storage
        Ok(self.state.try_get_module(module_id)?.is_some())
    }

    /// Get module bytecode if available
    pub fn get_module_bytes(&self, module_id: &ModuleId) -> Option<Vec<u8>> {
        match self.state.try_get_module(module_id) {
            Ok(bytes) => bytes,
            Err(error) => {
                log::error!("Failed to read module {module_id} from storage: {error:#}");
                None
            }
        }
    }

    /// List all published modules in storage
    pub fn list_modules(&self) -> Vec<ModuleId> {
        // Return modules from our maintained index
        self.published_modules
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .cloned()
            .collect()
    }
}
