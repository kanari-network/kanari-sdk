// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

/// DenyList record structure (mirrors `DenyList` in Move)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DenyListRecord {
    pub addresses: Vec<Address>,
}

impl DenyListRecord {
    /// Create a new empty deny list
    pub fn new() -> Self {
        Self {
            addresses: Vec::new(),
        }
    }

    /// Create from a vector of addresses
    pub fn from_addresses(addresses: Vec<Address>) -> Self {
        Self { addresses }
    }

    /// Return addresses reference
    pub fn addresses(&self) -> &Vec<Address> {
        &self.addresses
    }

    /// Add an address (no de-duplication)
    pub fn add(&mut self, addr: Address) {
        self.addresses.push(addr);
    }

    /// Remove the first matching address, returns true if removed
    pub fn remove(&mut self, addr: Address) -> bool {
        if let Some(pos) = self.addresses.iter().position(|a| a == &addr) {
            self.addresses.remove(pos);
            true
        } else {
            false
        }
    }
}

impl Default for DenyListRecord {
    fn default() -> Self {
        Self::new()
    }
}

/// DenyCap wrapper (mirrors `DenyCap<T>` which contains an `object::UID`)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DenyCapRecord {
    pub id: crate::object::UIDRecord,
}

impl DenyCapRecord {
    pub fn new(id: crate::object::UIDRecord) -> Self {
        Self { id }
    }
}

/// DenyList module constants and utilities, mirroring
/// `kanari-system/sources/deny_list.move`.
pub struct DenyListModule;

impl DenyListModule {
    pub const DENYLIST_MODULE: &'static str = "deny_list";
    /// Name of the DenyList struct in Move
    pub const DENYLIST_STRUCT: &'static str = "DenyList";

    /// Well-known object id of the shared deny-list singleton, mirroring
    /// `kanari_system::deny_list::deny_list_object_id()` (`@0x403`, the same
    /// address Sui uses).
    ///
    /// Genesis, the runtime singleton preload and protocol-level coin input
    /// policy all read this one definition, so they cannot disagree about where
    /// the shared `DenyList` lives.
    pub const DENY_LIST_OBJECT_ID: &'static str = "0x403";

    /// Bag index reserved for coin deny entries, mirroring
    /// `kanari_system::deny_list::coin_index()`.
    pub const COIN_INDEX: u64 = 0;

    /// Get the module ID for kanari_system::deny_list
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name =
            Identifier::new(Self::DENYLIST_MODULE).context("Invalid deny_list module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Fully qualified `DenyList` object type (`0x2::deny_list::DenyList`).
    pub fn type_path() -> String {
        format!(
            "{}::{}::{}",
            Address::KANARI_SYSTEM_ADDRESS,
            Self::DENYLIST_MODULE,
            Self::DENYLIST_STRUCT
        )
    }

    /// Well-known object id of the shared `DenyList` singleton.
    pub fn deny_list_object_id() -> Result<AccountAddress> {
        AccountAddress::from_hex_literal(Self::DENY_LIST_OBJECT_ID)
            .context("Invalid deny-list singleton object id")
    }

    /// Get function names used in deny_list module
    pub fn function_names() -> DenyListFunctions {
        DenyListFunctions {
            deny_list_object_id: "deny_list_object_id",
            borrow_singleton: "borrow_singleton",
            coin_index: "coin_index",
            create_for_test: "create_for_test",
            new_for_testing: "new_for_testing",
            new_denycap: "new_denycap",
            new_denycap_for_testing: "new_denycap_for_testing",
            v1_add: "v1_add",
            v1_remove: "v1_remove",
            v1_contains: "v1_contains",
            v2_enable_global_pause: "v2_enable_global_pause",
            v2_disable_global_pause: "v2_disable_global_pause",
            v2_is_global_pause_enabled: "v2_is_global_pause_enabled",
        }
    }
}

/// DenyList module function names, mirroring the public API of `deny_list.move`
/// in source order.
pub struct DenyListFunctions {
    pub deny_list_object_id: &'static str,
    pub borrow_singleton: &'static str,
    pub coin_index: &'static str,
    pub create_for_test: &'static str,
    pub new_for_testing: &'static str,
    pub new_denycap: &'static str,
    pub new_denycap_for_testing: &'static str,
    pub v1_add: &'static str,
    pub v1_remove: &'static str,
    pub v1_contains: &'static str,
    pub v2_enable_global_pause: &'static str,
    pub v2_disable_global_pause: &'static str,
    pub v2_is_global_pause_enabled: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::Address as KanariAddress;
    use move_core_types::account_address::AccountAddress;

    #[test]
    fn test_module_id() {
        let module_id = DenyListModule::get_module_id();
        assert!(module_id.is_ok());
    }

    #[test]
    fn test_functions() {
        let fns = DenyListModule::function_names();
        assert_eq!(fns.deny_list_object_id, "deny_list_object_id");
        assert_eq!(fns.borrow_singleton, "borrow_singleton");
        assert_eq!(fns.v1_add, "v1_add");
        assert_eq!(fns.v2_is_global_pause_enabled, "v2_is_global_pause_enabled");
    }

    #[test]
    fn test_singleton_object_id() {
        let id = DenyListModule::deny_list_object_id().expect("0x403 parses as an address");
        assert_eq!(id.to_hex_literal(), DenyListModule::DENY_LIST_OBJECT_ID);
        assert_eq!(DenyListModule::COIN_INDEX, 0);
    }

    #[test]
    fn test_record_ops() {
        let a1 = AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap();
        let a2 = AccountAddress::from_hex_literal(KanariAddress::KANARI_SYSTEM_ADDRESS).unwrap();
        let addr1: Address = a1.into();
        let addr2: Address = a2.into();

        let mut dl = DenyListRecord::new();
        assert!(dl.addresses().is_empty());
        dl.add(addr1);
        dl.add(addr2);
        assert_eq!(dl.addresses().len(), 2);
        let removed = dl.remove(addr1);
        assert!(removed);
        assert_eq!(dl.addresses().len(), 1);
    }
}
