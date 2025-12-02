//! Error Module - Move stdlib error implementation
//!
//! This module defines error codes and categories for Move.

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};

/// Error module constants and utilities
pub struct ErrorModule;

impl ErrorModule {
    pub const MODULE_NAME: &'static str = "error";

    // Error categories
    pub const INVALID_ARGUMENT: u64 = 0x1;
    pub const OUT_OF_RANGE: u64 = 0x2;
    pub const INVALID_STATE: u64 = 0x3;
    pub const UNAUTHENTICATED: u64 = 0x4;
    pub const PERMISSION_DENIED: u64 = 0x5;
    pub const NOT_FOUND: u64 = 0x6;
    pub const ABORTED: u64 = 0x7;
    pub const ALREADY_EXISTS: u64 = 0x8;
    pub const RESOURCE_EXHAUSTED: u64 = 0x9;
    pub const CANCELLED: u64 = 0xA;
    pub const INTERNAL: u64 = 0xB;
    pub const NOT_IMPLEMENTED: u64 = 0xC;
    pub const UNAVAILABLE: u64 = 0xD;

    /// Get the module ID for std::error
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::STD_ADDRESS)
            .context("Invalid std address")?;

        let module_name =
            Identifier::new(Self::MODULE_NAME).context("Invalid error module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names
    pub fn function_names() -> ErrorFunctions {
        ErrorFunctions {
            invalid_argument: "invalid_argument",
            out_of_range: "out_of_range",
            invalid_state: "invalid_state",
            unauthenticated: "unauthenticated",
            permission_denied: "permission_denied",
            not_found: "not_found",
            aborted: "aborted",
            already_exists: "already_exists",
            resource_exhausted: "resource_exhausted",
            cancelled: "cancelled",
            internal: "internal",
            not_implemented: "not_implemented",
            unavailable: "unavailable",
        }
    }

    /// Get error category name
    pub fn category_name(code: u64) -> &'static str {
        match code {
            Self::INVALID_ARGUMENT => "INVALID_ARGUMENT",
            Self::OUT_OF_RANGE => "OUT_OF_RANGE",
            Self::INVALID_STATE => "INVALID_STATE",
            Self::UNAUTHENTICATED => "UNAUTHENTICATED",
            Self::PERMISSION_DENIED => "PERMISSION_DENIED",
            Self::NOT_FOUND => "NOT_FOUND",
            Self::ABORTED => "ABORTED",
            Self::ALREADY_EXISTS => "ALREADY_EXISTS",
            Self::RESOURCE_EXHAUSTED => "RESOURCE_EXHAUSTED",
            Self::CANCELLED => "CANCELLED",
            Self::INTERNAL => "INTERNAL",
            Self::NOT_IMPLEMENTED => "NOT_IMPLEMENTED",
            Self::UNAVAILABLE => "UNAVAILABLE",
            _ => "UNKNOWN",
        }
    }
}

/// Error module function names
pub struct ErrorFunctions {
    pub invalid_argument: &'static str,
    pub out_of_range: &'static str,
    pub invalid_state: &'static str,
    pub unauthenticated: &'static str,
    pub permission_denied: &'static str,
    pub not_found: &'static str,
    pub aborted: &'static str,
    pub already_exists: &'static str,
    pub resource_exhausted: &'static str,
    pub cancelled: &'static str,
    pub internal: &'static str,
    pub not_implemented: &'static str,
    pub unavailable: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        assert_eq!(
            ErrorModule::category_name(ErrorModule::INVALID_ARGUMENT),
            "INVALID_ARGUMENT"
        );
        assert_eq!(
            ErrorModule::category_name(ErrorModule::NOT_FOUND),
            "NOT_FOUND"
        );
        assert_eq!(
            ErrorModule::category_name(ErrorModule::INTERNAL),
            "INTERNAL"
        );
    }

    #[test]
    fn test_error_module_id() {
        let module_id = ErrorModule::get_module_id();
        assert!(module_id.is_ok());
    }
}
