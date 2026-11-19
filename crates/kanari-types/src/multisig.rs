// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};

/// Multisig module constants and helpers for the `0x2::multisig` package.
pub struct MultisigModule;

impl MultisigModule {
    /// Package (framework) address of the multisig module (`0x2`).
    pub const MULTISIG_PACKAGE: &'static str = Address::KANARI_SYSTEM_ADDRESS;

    /// Move module name.
    pub const MULTISIG_MODULE: &'static str = "multisig";

    /// Fully qualified Move module path for `0x2::multisig`.
    pub fn module_path() -> String {
        format!("{}::{}", Address::KANARI_SYSTEM_ADDRESS, Self::MULTISIG_MODULE)
    }

    /// Get the module ID for `0x2::multisig`.
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;
        let module_name =
            Identifier::new(Self::MULTISIG_MODULE).context("Invalid multisig module name")?;
        Ok(ModuleId::new(address, module_name))
    }

    /// Entry function names.
    pub fn entry_functions() -> MultisigEntryFunctions {
        MultisigEntryFunctions {
            create_wallet_entry: "create_wallet_entry",
            propose_transfer_entry: "propose_transfer_entry",
            approve_entry: "approve_entry",
            execute_entry: "execute_entry",
            cancel_entry: "cancel_entry",
            deposit_entry: "deposit_entry",
        }
    }
}

pub struct MultisigEntryFunctions {
    pub create_wallet_entry: &'static str,
    pub propose_transfer_entry: &'static str,
    pub approve_entry: &'static str,
    pub execute_entry: &'static str,
    pub cancel_entry: &'static str,
    pub deposit_entry: &'static str,
}
