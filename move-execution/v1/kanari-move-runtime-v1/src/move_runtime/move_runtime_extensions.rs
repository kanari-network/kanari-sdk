// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Extension traits for the Move runtime.

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

        // 4. Object-model rule (Sui parity): every `key` struct must declare
        // `id: 0x2::object::UID` as its first field. Transfer tracking reads
        // the UID from the first 32 bytes, so anything else silently breaks
        // object identity.
        Self::verify_key_structs_carry_uid(module)?;

        Ok(())
    }

    fn verify_key_structs_carry_uid(module: &CompiledModule) -> Result<()> {
        use move_binary_format::file_format::{SignatureToken, StructFieldInformation};

        let system_addr = KanariAddress::kanari_system_account_address();
        for def in module.struct_defs() {
            let handle = module.struct_handle_at(def.struct_handle);
            if !handle.abilities.has_key() {
                continue;
            }
            let struct_name = module.identifier_at(handle.name).to_string();
            let fields = match &def.field_information {
                StructFieldInformation::Declared(fields) => fields,
                StructFieldInformation::Native => {
                    anyhow::bail!(
                        "key struct {struct_name} has no declared fields; \
                         key structs must declare `id: 0x2::object::UID` first"
                    );
                }
            };
            let Some(first) = fields.first() else {
                anyhow::bail!(
                    "key struct {struct_name} has no fields; \
                     key structs must declare `id: 0x2::object::UID` first"
                );
            };
            let is_uid = match &first.signature.0 {
                SignatureToken::Struct(field_handle) => {
                    let field_struct = module.struct_handle_at(*field_handle);
                    let field_module = module.module_handle_at(field_struct.module);
                    module.address_identifier_at(field_module.address) == &system_addr
                        && module.identifier_at(field_module.name).as_str() == "object"
                        && module.identifier_at(field_struct.name).as_str() == "UID"
                }
                _ => false,
            };
            if !is_uid {
                anyhow::bail!(
                    "key struct {struct_name} must declare `id: 0x2::object::UID` \
                     as its first field"
                );
            }
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
