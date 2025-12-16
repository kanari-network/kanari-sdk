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

/// DenyList module constants and utilities
pub struct DenyListModule;

impl DenyListModule {
    pub const DENYLIST_MODULE: &'static str = "deny_list";

    /// Get the module ID for kanari_system::deny_list
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name =
            Identifier::new(Self::DENYLIST_MODULE).context("Invalid deny_list module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names used in deny_list module
    pub fn function_names() -> DenyListFunctions {
        DenyListFunctions {
            new_denylist: "new_denylist",
            new_denycap: "new_denycap",
            deny_list_add: "deny_list_add",
            deny_list_remove: "deny_list_remove",
        }
    }
}

/// DenyList module function names
pub struct DenyListFunctions {
    pub new_denylist: &'static str,
    pub new_denycap: &'static str,
    pub deny_list_add: &'static str,
    pub deny_list_remove: &'static str,
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
        assert_eq!(fns.new_denylist, "new_denylist");
        assert_eq!(fns.deny_list_add, "deny_list_add");
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
