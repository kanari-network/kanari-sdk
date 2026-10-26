// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Extension traits for the Move runtime.

use anyhow::Result;
use kanari_types::address::Address as KanariAddress;
use move_binary_format::file_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use std::collections::HashSet;

use crate::move_runtime::MoveRuntime;

impl MoveRuntime {
    /// Verify a compiled module before publishing
    /// Checks basic invariants and dependencies
    ///
    /// `package_modules` holds the ids of the modules published together with
    /// `module` in the same package. They are not on chain yet — they are
    /// published by the very same transaction — but a dependency on a sibling
    /// is satisfied by the package itself. Without this, a package whose
    /// modules depend on each other could never be published.
    pub(crate) fn verify_module(
        &self,
        module: &CompiledModule,
        package_modules: &HashSet<ModuleId>,
    ) -> Result<()> {
        // Basic verification checks

        // 1. Check module has valid self-id
        let module_id = module.self_id();
        if module_id.name().as_str().is_empty() {
            anyhow::bail!("Module has empty name");
        }

        // 2. Check all dependencies are available
        for dep in module.immediate_dependencies() {
            // A sibling published in the same package is published by this
            // same transaction, so the dependency resolves within the package.
            if package_modules.contains(&dep) {
                continue;
            }
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
