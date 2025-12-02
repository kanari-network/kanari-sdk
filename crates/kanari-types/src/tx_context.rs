use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

/// Transaction context structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TxContextRecord {
    pub sender: String,
    pub tx_hash: Vec<u8>,
    pub epoch: u64,
    pub epoch_timestamp_ms: u64,
    pub ids_created: u64,
}

impl TxContextRecord {
    /// Create a new transaction context
    pub fn new(
        sender: String,
        tx_hash: Vec<u8>,
        epoch: u64,
        epoch_timestamp_ms: u64,
        ids_created: u64,
    ) -> Self {
        Self {
            sender,
            tx_hash,
            epoch,
            epoch_timestamp_ms,
            ids_created,
        }
    }

    /// Create from AccountAddress
    pub fn from_address(
        sender: AccountAddress,
        tx_hash: Vec<u8>,
        epoch: u64,
        epoch_timestamp_ms: u64,
        ids_created: u64,
    ) -> Self {
        Self::new(
            format!("{}", sender),
            tx_hash,
            epoch,
            epoch_timestamp_ms,
            ids_created,
        )
    }

    /// Get sender address
    pub fn sender(&self) -> &str {
        &self.sender
    }

    /// Get current epoch
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    /// Return tx hash bytes
    pub fn tx_hash(&self) -> &Vec<u8> {
        &self.tx_hash
    }

    /// Return epoch start time in milliseconds
    pub fn epoch_timestamp_ms(&self) -> u64 {
        self.epoch_timestamp_ms
    }

    /// Number of ids created in this transaction
    pub fn ids_created(&self) -> u64 {
        self.ids_created
    }
}

/// TxContext module constants and utilities
pub struct TxContextModule;

impl TxContextModule {
    pub const TX_CONTEXT_MODULE: &'static str = "tx_context";

    /// Get the module ID for kanari_system::tx_context
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name =
            Identifier::new(Self::TX_CONTEXT_MODULE).context("Invalid tx_context module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names used in tx_context module
    pub fn function_names() -> TxContextFunctions {
        TxContextFunctions {
            sender: "sender",
            epoch: "epoch",
            digest: "digest",
            epoch_timestamp_ms: "epoch_timestamp_ms",
            fresh_object_address: "fresh_object_address",
            derive_id: "derive_id",
        }
    }
}

/// TxContext module function names
pub struct TxContextFunctions {
    pub sender: &'static str,
    pub epoch: &'static str,
    pub digest: &'static str,
    pub epoch_timestamp_ms: &'static str,
    pub fresh_object_address: &'static str,
    pub derive_id: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx_context_creation() {
        let tx_hash = vec![1, 2, 3, 4];
        let ctx = TxContextRecord::new("0x1".to_string(), tx_hash.clone(), 5, 1_600_000_000, 2);
        assert_eq!(ctx.sender(), "0x1");
        assert_eq!(ctx.epoch(), 5);
        assert_eq!(ctx.tx_hash(), &tx_hash);
        assert_eq!(ctx.epoch_timestamp_ms(), 1_600_000_000);
        assert_eq!(ctx.ids_created(), 2);
    }

    #[test]
    fn test_module_id() {
        let module_id = TxContextModule::get_module_id();
        assert!(module_id.is_ok());
    }
}
