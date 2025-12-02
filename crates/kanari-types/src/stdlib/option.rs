//! Option Module - Move stdlib option implementation
//!
//! This module defines the Option type to represent and handle optional values.

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

/// Option value representation
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum OptionValue<T> {
    Some(T),
    None,
}

impl<T> OptionValue<T> {
    /// Create a Some value
    pub fn some(value: T) -> Self {
        OptionValue::Some(value)
    }

    /// Create a None value
    pub fn none() -> Self {
        OptionValue::None
    }

    /// Check if is Some
    pub fn is_some(&self) -> bool {
        matches!(self, OptionValue::Some(_))
    }

    /// Check if is None
    pub fn is_none(&self) -> bool {
        matches!(self, OptionValue::None)
    }

    /// Get the value, panics if None
    pub fn unwrap(self) -> T {
        match self {
            OptionValue::Some(v) => v,
            OptionValue::None => panic!("Called unwrap on None"),
        }
    }

    /// Get reference to value if Some
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            OptionValue::Some(v) => Some(v),
            OptionValue::None => None,
        }
    }

    /// Take the value, leaving None
    pub fn take(&mut self) -> Self {
        std::mem::replace(self, OptionValue::None)
    }
}

impl<T> From<Option<T>> for OptionValue<T> {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => OptionValue::Some(v),
            None => OptionValue::None,
        }
    }
}

impl<T> From<OptionValue<T>> for Option<T> {
    fn from(opt: OptionValue<T>) -> Self {
        match opt {
            OptionValue::Some(v) => Some(v),
            OptionValue::None => None,
        }
    }
}

/// Option module constants and utilities
pub struct OptionModule;

impl OptionModule {
    pub const MODULE_NAME: &'static str = "option";

    /// Error: Option is set when it should be None
    pub const EOPTION_IS_SET: u64 = 0x40000;
    /// Error: Option is not set when it should be Some
    pub const EOPTION_NOT_SET: u64 = 0x40001;

    /// Get the module ID for std::option
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::STD_ADDRESS)
            .context("Invalid std address")?;

        let module_name =
            Identifier::new(Self::MODULE_NAME).context("Invalid option module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names
    pub fn function_names() -> OptionFunctions {
        OptionFunctions {
            none: "none",
            some: "some",
            is_none: "is_none",
            is_some: "is_some",
            contains: "contains",
            borrow: "borrow",
            borrow_with_default: "borrow_with_default",
            get_with_default: "get_with_default",
            extract: "extract",
            destroy_with_default: "destroy_with_default",
        }
    }
}

/// Option module function names
pub struct OptionFunctions {
    pub none: &'static str,
    pub some: &'static str,
    pub is_none: &'static str,
    pub is_some: &'static str,
    pub contains: &'static str,
    pub borrow: &'static str,
    pub borrow_with_default: &'static str,
    pub get_with_default: &'static str,
    pub extract: &'static str,
    pub destroy_with_default: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_some() {
        let opt = OptionValue::some(42);
        assert!(opt.is_some());
        assert!(!opt.is_none());
        assert_eq!(opt.unwrap(), 42);
    }

    #[test]
    fn test_option_none() {
        let opt: OptionValue<i32> = OptionValue::none();
        assert!(opt.is_none());
        assert!(!opt.is_some());
    }

    #[test]
    fn test_option_conversion() {
        let rust_opt = Some(100);
        let move_opt: OptionValue<i32> = rust_opt.into();
        assert!(move_opt.is_some());

        let back: Option<i32> = move_opt.into();
        assert_eq!(back, Some(100));
    }
}
