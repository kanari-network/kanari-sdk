use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use std::collections::HashMap;

/// Module registry for all Kanari system modules
pub struct ModuleRegistry;

impl ModuleRegistry {
    /// Module name constants
    pub const KANARI: &'static str = "kanari";
    pub const BALANCE: &'static str = "balance";
    pub const COIN: &'static str = "coin";
    pub const OBJECT: &'static str = "object";
    pub const TRANSFER: &'static str = "transfer";
    pub const TX_CONTEXT: &'static str = "tx_context";

    /// Get all module names
    pub fn all_modules() -> Vec<&'static str> {
        vec![
            Self::KANARI,
            Self::BALANCE,
            Self::COIN,
            Self::OBJECT,
            Self::TRANSFER,
            Self::TX_CONTEXT,
        ]
    }

    /// Get module ID for a given module name
    pub fn get_module_id(module_name: &str) -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let identifier = Identifier::new(module_name)
            .with_context(|| format!("Invalid module name: {}", module_name))?;

        Ok(ModuleId::new(address, identifier))
    }

    /// Get all module IDs
    pub fn all_module_ids() -> Result<Vec<ModuleId>> {
        Self::all_modules()
            .iter()
            .map(|name| Self::get_module_id(name))
            .collect()
    }

    /// Get function names for a specific module
    pub fn get_function_names(module_name: &str) -> Vec<&'static str> {
        match module_name {
            Self::KANARI => vec!["transfer", "burn"],
            Self::BALANCE => vec![
                "zero",
                "create",
                "value",
                "increase",
                "decrease",
                "transfer",
                "has_sufficient",
                "destroy",
                "new_supply",
                "increase_supply",
                "destroy_supply",
                "merge",
                "split",
            ],
            Self::COIN => vec![
                "create_currency",
                "mint",
                "mint_and_transfer",
                "from_balance",
                "burn",
                "total_supply",
                "value",
                "split",
                "join",
                "treasury_into_supply",
                "into_balance",
            ],
            Self::OBJECT => vec!["new", "uid_address", "id_address_as_u64", "id_bytes"],
            Self::TRANSFER => vec![
                "create_transfer",
                "get_amount",
                "get_from",
                "get_to",
                "total_amount",
                "is_valid_amount",
                "public_freeze_object",
                "public_transfer",
            ],
            Self::TX_CONTEXT => vec![
                "sender",
                "digest",
                "epoch",
                "epoch_timestamp_ms",
                "fresh_object_address",
                "derive_id",
            ],
            _ => vec![],
        }
    }

    /// Check if a module exists
    pub fn module_exists(module_name: &str) -> bool {
        Self::all_modules().contains(&module_name)
    }

    /// Get module metadata (name, address, function count)
    pub fn get_module_info(module_name: &str) -> Option<ModuleInfo> {
        if !Self::module_exists(module_name) {
            return None;
        }

        let functions = Self::get_function_names(module_name);
        Some(ModuleInfo {
            name: module_name.to_string(),
            address: Address::KANARI_SYSTEM_ADDRESS.to_string(),
            function_count: functions.len(),
            functions: functions.iter().map(|s| s.to_string()).collect(),
        })
    }

    /// Get all modules information
    pub fn all_modules_info() -> Vec<ModuleInfo> {
        Self::all_modules()
            .iter()
            .filter_map(|name| Self::get_module_info(name))
            .collect()
    }

    /// Create a function map for quick lookup
    pub fn create_function_map() -> HashMap<String, Vec<String>> {
        Self::all_modules()
            .iter()
            .map(|module| {
                let functions = Self::get_function_names(module)
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                (module.to_string(), functions)
            })
            .collect()
    }

    /// Check if a function exists in a module
    pub fn function_exists(module_name: &str, function_name: &str) -> bool {
        Self::get_function_names(module_name).contains(&function_name)
    }

    /// Get fully qualified function identifier
    pub fn get_function_identifier(module_name: &str, function_name: &str) -> Option<String> {
        if Self::function_exists(module_name, function_name) {
            Some(format!(
                "{}::{}::{}",
                Address::KANARI_SYSTEM_ADDRESS,
                module_name,
                function_name
            ))
        } else {
            None
        }
    }
}

/// Module information structure
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub address: String,
    pub function_count: usize,
    pub functions: Vec<String>,
}

impl ModuleInfo {
    /// Display module info as formatted string
    pub fn display(&self) -> String {
        format!(
            "Module: {}::{}\nFunctions ({}):\n  {}",
            self.address,
            self.name,
            self.function_count,
            self.functions.join("\n  ")
        )
    }
}

/// Builder for creating module calls
pub struct ModuleCallBuilder {
    module_name: String,
    function_name: String,
}

impl ModuleCallBuilder {
    /// Create new module call builder
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
            function_name: String::new(),
        }
    }

    /// Set function name
    pub fn function(mut self, function_name: impl Into<String>) -> Self {
        self.function_name = function_name.into();
        self
    }

    /// Validate the call
    pub fn validate(&self) -> Result<()> {
        if !ModuleRegistry::module_exists(&self.module_name) {
            anyhow::bail!("Module '{}' does not exist", self.module_name);
        }

        if !ModuleRegistry::function_exists(&self.module_name, &self.function_name) {
            anyhow::bail!(
                "Function '{}' does not exist in module '{}'",
                self.function_name,
                self.module_name
            );
        }

        Ok(())
    }

    /// Build module ID
    pub fn build_module_id(&self) -> Result<ModuleId> {
        self.validate()?;
        ModuleRegistry::get_module_id(&self.module_name)
    }

    /// Get fully qualified function identifier
    pub fn build_identifier(&self) -> Result<String> {
        self.validate()?;
        ModuleRegistry::get_function_identifier(&self.module_name, &self.function_name)
            .ok_or_else(|| anyhow::anyhow!("Failed to build function identifier"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_modules() {
        let modules = ModuleRegistry::all_modules();
        assert_eq!(modules.len(), 6);
        assert!(modules.contains(&"kanari"));
        assert!(modules.contains(&"coin"));
    }

    #[test]
    fn test_module_exists() {
        assert!(ModuleRegistry::module_exists("kanari"));
        assert!(ModuleRegistry::module_exists("coin"));
        assert!(!ModuleRegistry::module_exists("invalid"));
    }

    #[test]
    fn test_get_module_id() {
        let module_id = ModuleRegistry::get_module_id("kanari").unwrap();
        assert_eq!(module_id.name().to_string(), "kanari");
    }

    #[test]
    fn test_function_names() {
        let functions = ModuleRegistry::get_function_names("kanari");
        assert!(!functions.is_empty());
        assert!(functions.contains(&"transfer"));
    }

    #[test]
    fn test_function_exists() {
        assert!(ModuleRegistry::function_exists("kanari", "transfer"));
        assert!(!ModuleRegistry::function_exists(
            "kanari",
            "invalid_function"
        ));
    }

    #[test]
    fn test_module_info() {
        let info = ModuleRegistry::get_module_info("coin").unwrap();
        assert_eq!(info.name, "coin");
        assert!(info.function_count > 0);
    }

    #[test]
    fn test_function_map() {
        let map = ModuleRegistry::create_function_map();
        assert_eq!(map.len(), 6);
        assert!(map.contains_key("kanari"));
    }

    #[test]
    fn test_module_call_builder() {
        let builder = ModuleCallBuilder::new("kanari").function("transfer");
        assert!(builder.validate().is_ok());

        let module_id = builder.build_module_id().unwrap();
        assert_eq!(module_id.name().to_string(), "kanari");
    }

    #[test]
    fn test_invalid_module_call() {
        let builder = ModuleCallBuilder::new("invalid").function("test");
        assert!(builder.validate().is_err());
    }

    #[test]
    fn test_get_function_identifier() {
        let identifier = ModuleRegistry::get_function_identifier("kanari", "transfer");
        assert!(identifier.is_some());
        assert!(identifier.unwrap().contains("kanari::transfer"));
    }

    #[test]
    fn test_all_modules_info() {
        let infos = ModuleRegistry::all_modules_info();
        assert_eq!(infos.len(), 6);

        for info in infos {
            assert!(!info.name.is_empty());
            assert!(!info.functions.is_empty());
        }
    }
}
