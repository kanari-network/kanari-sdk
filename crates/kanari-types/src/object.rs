// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::account_address::AccountAddress;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use serde::{Deserialize, Serialize};

/// UID wrapper used by Move `object::UID` (contains an address)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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

/// ID wrapper used by Move `object::ID` (contains an address)
/// Added to support DEX/DeFi features where copyable IDs are needed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct IDRecord {
    pub bytes: AccountAddress,
}

impl IDRecord {
    /// Create a new IDRecord from an AccountAddress
    pub fn new(bytes: AccountAddress) -> Self {
        Self { bytes }
    }

    /// Return the underlying address
    pub fn address(&self) -> AccountAddress {
        self.bytes
    }

    /// Convenience: construct from hex literal string like "0x1"
    pub fn from_hex_literal(hex: &str) -> Result<Self> {
        let bytes = AccountAddress::from_hex_literal(hex).context("invalid address")?;
        Ok(Self::new(bytes))
    }
}

/// Object module constants and utilities
pub struct ObjectModule;

impl ObjectModule {
    pub const OBJECT_MODULE: &'static str = "object";

    /// Name of the UID struct in Move
    pub const UID_STRUCT: &'static str = "UID";

    /// Name of the ID struct in Move
    pub const ID_STRUCT: &'static str = "ID";

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
            uid_to_inner: "uid_to_inner",
            id_from_address: "id_from_address",
            id_to_address: "id_to_address",
            id_to_bytes: "id_to_bytes",
            uid_address: "uid_address",
            uid_to_u64: "uid_to_u64",
            uid_to_bytes: "uid_to_bytes",
            id_bytes: "id_bytes",
            save_object: "save_object",
            delete: "delete",
        }
    }
}

/// Object module function names
pub struct ObjectFunctions {
    pub new: &'static str,
    pub uid_to_inner: &'static str,
    pub id_from_address: &'static str,
    pub id_to_address: &'static str,
    pub id_to_bytes: &'static str,
    pub uid_address: &'static str,
    pub uid_to_u64: &'static str,
    pub uid_to_bytes: &'static str,
    pub id_bytes: &'static str,
    pub save_object: &'static str,
    pub delete: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::account_address::AccountAddress;

    #[test]
    fn test_uid_record_from_hex() {
        use crate::address::Address as KanariAddress;

        let uid = UIDRecord::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap();
        let expected = AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap();
        assert_eq!(uid.addr, expected);
    }

    #[test]
    fn test_id_record_from_hex() {
        use crate::address::Address as KanariAddress;

        let id = IDRecord::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap();
        let expected = AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap();
        assert_eq!(id.bytes, expected);
    }
}
