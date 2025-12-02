use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::account_address::AccountAddress;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use serde::{Deserialize, Serialize};

/// UID wrapper used by Move `object::UID` (contains an address)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UIDRecord {
    pub addr: AccountAddress,
}

impl UIDRecord {
    /// Create a new UIDRecord from an AccountAddress
    pub fn new(addr: AccountAddress) -> Self {
        Self { addr }
    }

    /// Return the underlying address
    pub fn address(&self) -> AccountAddress {
        self.addr
    }

    /// Convenience: construct from hex literal string like "0x1"
    pub fn from_hex_literal(hex: &str) -> Result<Self> {
        let addr = AccountAddress::from_hex_literal(hex).context("invalid address")?;
        Ok(Self::new(addr))
    }
}

/// Object module constants and utilities
pub struct ObjectModule;

impl ObjectModule {
    pub const OBJECT_MODULE: &'static str = "object";

    /// Get the module ID for kanari_system::object
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name =
            Identifier::new(Self::OBJECT_MODULE).context("Invalid object module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names used in object module
    pub fn function_names() -> ObjectFunctions {
        ObjectFunctions {
            new: "new",
            uid_address: "uid_address",
            id_address_as_u64: "id_address_as_u64",
            id_bytes: "id_bytes",
        }
    }
}

/// Object module function names
pub struct ObjectFunctions {
    pub new: &'static str,
    pub uid_address: &'static str,
    pub id_address_as_u64: &'static str,
    pub id_bytes: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::account_address::AccountAddress;

    #[test]
    fn test_uid_record_from_hex() {
        let uid = UIDRecord::from_hex_literal("0x1").unwrap();
        let expected = AccountAddress::from_hex_literal("0x1").unwrap();
        assert_eq!(uid.addr, expected);
    }

    #[test]
    fn test_module_id() {
        let module_id = ObjectModule::get_module_id();
        assert!(module_id.is_ok());
    }

    #[test]
    fn test_function_names() {
        let fns = ObjectModule::function_names();
        assert_eq!(fns.new, "new");
        assert_eq!(fns.uid_address, "uid_address");
    }
}
