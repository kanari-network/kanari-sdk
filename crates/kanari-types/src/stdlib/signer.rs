//! Signer Module - Move stdlib signer implementation
//!
//! The signer module defines operations for the Move `signer` native type.

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

/// Signer representation (holds an address)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SignerRef {
    pub address: String,
}

impl SignerRef {
    /// Create new signer reference
    pub fn new(address: String) -> Self {
        Self { address }
    }

    /// Create from AccountAddress
    pub fn from_address(addr: AccountAddress) -> Self {
        Self::new(format!("{}", addr))
    }

    /// Get address
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Parse to AccountAddress
    pub fn to_account_address(&self) -> Result<AccountAddress> {
        AccountAddress::from_hex_literal(&self.address).context("Invalid address format")
    }
}

/// Signer module constants and utilities
pub struct SignerModule;

impl SignerModule {
    pub const MODULE_NAME: &'static str = "signer";

    /// Get the module ID for std::signer
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::STD_ADDRESS)
            .context("Invalid std address")?;

        let module_name =
            Identifier::new(Self::MODULE_NAME).context("Invalid signer module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names
    pub fn function_names() -> SignerFunctions {
        SignerFunctions {
            address_of: "address_of",
            borrow_address: "borrow_address",
            address_to_u64: "address_to_u64",
            address_to_bytes: "address_to_bytes",
        }
    }
}

/// Signer module function names
pub struct SignerFunctions {
    pub address_of: &'static str,
    pub borrow_address: &'static str,
    pub address_to_u64: &'static str,
    pub address_to_bytes: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signer_creation() {
        let signer = SignerRef::new("0x1".to_string());
        assert_eq!(signer.address(), "0x1");
    }

    #[test]
    fn test_signer_module_id() {
        let module_id = SignerModule::get_module_id();
        assert!(module_id.is_ok());
    }
}
