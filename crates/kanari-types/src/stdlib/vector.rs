//! Vector Module - Move stdlib vector implementation
//!
//! A variable-sized container that can hold any type.

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};

/// Vector module constants and utilities
pub struct VectorModule;

impl VectorModule {
    pub const MODULE_NAME: &'static str = "vector";

    /// Error: index out of bounds
    pub const EINDEX_OUT_OF_BOUNDS: u64 = 0x20000;

    /// Get the module ID for std::vector
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::STD_ADDRESS)
            .context("Invalid std address")?;

        let module_name =
            Identifier::new(Self::MODULE_NAME).context("Invalid vector module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names
    pub fn function_names() -> VectorFunctions {
        VectorFunctions {
            empty: "empty",
            length: "length",
            borrow: "borrow",
            push_back: "push_back",
            borrow_mut: "borrow_mut",
            pop_back: "pop_back",
            destroy_empty: "destroy_empty",
            swap: "swap",
            singleton: "singleton",
            reverse: "reverse",
            append: "append",
            is_empty: "is_empty",
            contains: "contains",
            index_of: "index_of",
            remove: "remove",
            swap_remove: "swap_remove",
        }
    }
}

/// Vector module function names
pub struct VectorFunctions {
    pub empty: &'static str,
    pub length: &'static str,
    pub borrow: &'static str,
    pub push_back: &'static str,
    pub borrow_mut: &'static str,
    pub pop_back: &'static str,
    pub destroy_empty: &'static str,
    pub swap: &'static str,
    pub singleton: &'static str,
    pub reverse: &'static str,
    pub append: &'static str,
    pub is_empty: &'static str,
    pub contains: &'static str,
    pub index_of: &'static str,
    pub remove: &'static str,
    pub swap_remove: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_module_id() {
        let module_id = VectorModule::get_module_id();
        assert!(module_id.is_ok());
    }

    #[test]
    fn test_vector_functions() {
        let funcs = VectorModule::function_names();
        assert_eq!(funcs.empty, "empty");
        assert_eq!(funcs.length, "length");
        assert_eq!(funcs.push_back, "push_back");
    }
}
