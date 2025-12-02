use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

/// Transfer record structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransferRecord {
    pub from: AccountAddress,
    pub to: AccountAddress,
    pub amount: u64,
}

impl TransferRecord {
    /// Create a new transfer record from Move `AccountAddress`s
    pub fn new(from: AccountAddress, to: AccountAddress, amount: u64) -> Self {
        Self { from, to, amount }
    }

    /// Create transfer record from hex string literals (convenience)
    pub fn from_hex_literals(from_hex: &str, to_hex: &str, amount: u64) -> Result<Self> {
        let from = AccountAddress::from_hex_literal(from_hex).context("Invalid from address")?;
        let to = AccountAddress::from_hex_literal(to_hex).context("Invalid to address")?;
        Ok(Self::new(from, to, amount))
    }
}

/// Transfer validation utilities
pub struct TransferValidator;

impl TransferValidator {
    /// Validate transfer with addresses directly
    pub fn validate_addresses(
        from: &AccountAddress,
        to: &AccountAddress,
        amount: u64,
    ) -> Result<bool> {
        // Reject zero amounts, identical addresses, and the zero address
        let zero =
            AccountAddress::from_hex_literal("0x0").context("failed to build zero address")?;
        let valid = amount > 0 && from != to && *from != zero && *to != zero;
        Ok(valid)
    }
}

/// Transfer module constants and utilities
pub struct TransferModule;

impl TransferModule {
    pub const TRANSFER_MODULE: &'static str = "transfer";

    /// Get the module ID for system::transfer
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name =
            Identifier::new(Self::TRANSFER_MODULE).context("Invalid transfer module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names used in transfer module
    pub fn function_names() -> TransferFunctions {
        TransferFunctions {
            is_valid_amount: "is_valid_amount",
            create_transfer: "create_transfer",
            get_amount: "get_amount",
            get_from: "get_from",
            get_to: "get_to",
            total_amount: "total_amount",
            public_freeze_object: "public_freeze_object",
            public_transfer: "public_transfer",
        }
    }

    /// Convenience: return the function name for `total_amount`
    pub fn total_amount_name() -> &'static str {
        "total_amount"
    }
}

/// Transfer module function names
pub struct TransferFunctions {
    pub is_valid_amount: &'static str,
    pub create_transfer: &'static str,
    pub get_amount: &'static str,
    pub get_from: &'static str,
    pub get_to: &'static str,
    pub total_amount: &'static str,
    pub public_freeze_object: &'static str,
    pub public_transfer: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_record_creation() {
        let record = TransferRecord::from_hex_literals("0x1", "0x2", 1000).unwrap();
        let expected_from = AccountAddress::from_hex_literal("0x1").unwrap();
        let expected_to = AccountAddress::from_hex_literal("0x2").unwrap();
        assert_eq!(record.from, expected_from);
        assert_eq!(record.to, expected_to);
        assert_eq!(record.amount, 1000);
        // timestamp removed in Move transfer; record contains only from/to/amount
    }

    #[test]
    fn test_transfer_validator() {
        let addr1 = AccountAddress::from_hex_literal("0x1").unwrap();
        let addr2 = AccountAddress::from_hex_literal("0x2").unwrap();

        // Valid transfer
        assert!(TransferValidator::validate_addresses(&addr1, &addr2, 500).unwrap());

        // Invalid: zero amount
        assert!(!TransferValidator::validate_addresses(&addr1, &addr2, 0).unwrap());

        // Invalid: same address
        assert!(!TransferValidator::validate_addresses(&addr1, &addr1, 500).unwrap());
    }

    #[test]
    fn test_get_transfer_module_id() {
        let module_id = TransferModule::get_module_id();
        assert!(module_id.is_ok());

        let module_id = module_id.unwrap();
        assert_eq!(module_id.name().as_str(), "transfer");
    }
}
