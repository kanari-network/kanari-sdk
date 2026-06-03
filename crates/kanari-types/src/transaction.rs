// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Blockchain data structures and operations
use anyhow::Result;
use kanari_crypto::keys::CurveType;
use kanari_crypto::verify_signature;
use kanari_crypto::{hash_data_blake3, signatures::sign_message};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use tracing::error;

/// Signed transaction wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Vec<u8>,
}

impl SignedTransaction {
    pub fn new(transaction: Transaction) -> Self {
        Self {
            transaction,
            signature: vec![],
        }
    }

    pub fn sign(&mut self, private_key: &str, curve_type: CurveType) -> Result<()> {
        let tx_hash = self.transaction.hash();
        let signature = sign_message(private_key, &tx_hash, curve_type)
            .map_err(|e| anyhow::anyhow!("Failed to sign transaction: {}", e))?;
        self.signature = signature;
        Ok(())
    }

    pub fn verify_signature(&self) -> Result<bool> {
        let tx_hash = self.transaction.hash();
        self.verify_signature_for_hash(&tx_hash)
    }

    pub fn verify_signature_for_hash(&self, tx_hash: &[u8]) -> Result<bool> {
        if self.signature.is_empty() {
            anyhow::bail!("Transaction not signed");
        }

        let signature = &self.signature;
        let sender = self.transaction.sender();

        verify_signature(sender, tx_hash, signature)
            .map_err(|e| anyhow::anyhow!("Signature verification failed: {}", e))
    }

    pub fn hash(&self) -> Vec<u8> {
        let serialized = match bcs::to_bytes(self) {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to serialize SignedTransaction for hashing: {}", e);
                Vec::new()
            }
        };
        hash_data_blake3(&serialized)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeCall {
    TransferAmount { recipient: String, amount: u64 },
    BurnAmount { amount: u64 },
}

impl NativeCall {
    pub fn tx_type_label(&self) -> &'static str {
        match self {
            Self::TransferAmount { .. } => "transfer",
            Self::BurnAmount { .. } => "burn",
        }
    }

    pub fn required_native_amount(&self) -> u64 {
        match self {
            Self::TransferAmount { amount, .. } | Self::BurnAmount { amount } => *amount,
        }
    }
}

/// Transaction types in Kanari blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transaction {
    /// Publish a Move module
    PublishModule {
        sender: String,
        module_bytes: Vec<u8>,
        module_name: String,
        gas_limit: u64,
        gas_price: u64,
        sequence_number: u64,
    },
    /// Execute a Move function
    ExecuteFunction {
        sender: String,
        module: String,
        function: String,
        type_args: Vec<String>,
        args: Vec<Vec<u8>>,
        gas_limit: u64,
        gas_price: u64,
        sequence_number: u64,
    },
}

impl Transaction {
    pub const KANARI_MODULE: &'static str = "0x2::kanari";
    pub const TRANSFER_AMOUNT_FUNCTION: &'static str = "transfer_amount";
    pub const BURN_AMOUNT_FUNCTION: &'static str = "burn_amount";

    pub fn hash(&self) -> Vec<u8> {
        let serialized = match bcs::to_bytes(self) {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to serialize Transaction for hashing: {}", e);
                Vec::new()
            }
        };
        hash_data_blake3(&serialized)
    }

    pub fn sender(&self) -> &str {
        match self {
            Transaction::PublishModule { sender, .. } => sender,
            Transaction::ExecuteFunction { sender, .. } => sender,
        }
    }

    pub fn sender_address(&self) -> &str {
        self.sender()
    }

    pub fn sequence_number(&self) -> u64 {
        match self {
            Transaction::PublishModule {
                sequence_number, ..
            } => *sequence_number,
            Transaction::ExecuteFunction {
                sequence_number, ..
            } => *sequence_number,
        }
    }

    pub fn gas_limit(&self) -> u64 {
        match self {
            Transaction::PublishModule { gas_limit, .. } => *gas_limit,
            Transaction::ExecuteFunction { gas_limit, .. } => *gas_limit,
        }
    }

    pub fn gas_price(&self) -> u64 {
        match self {
            Transaction::PublishModule { gas_price, .. } => *gas_price,
            Transaction::ExecuteFunction { gas_price, .. } => *gas_price,
        }
    }

    /// Get conflict keys for this transaction.
    /// Transactions with overlapping conflict keys must be executed sequentially.
    pub fn get_conflict_keys(&self) -> Vec<String> {
        // 1. Force Normalize Sender (convert to standard Address, lowercase, always with 0x)
        let sender_norm = if let Ok(addr) = AccountAddress::from_hex_literal(self.sender()) {
            addr.to_hex_literal()
        } else {
            let s = self.sender();
            if !s.starts_with("0x") {
                format!("0x{}", s)
            } else {
                s.to_string()
            }
        };

        let mut keys = vec![sender_norm];

        match self {
            Transaction::ExecuteFunction { args, .. } => {
                for arg in args {
                    if arg.len() == 32
                        && let Ok(addr) = AccountAddress::from_bytes(arg)
                    {
                        keys.push(addr.to_hex_literal());
                        continue; // Skip to next item if condition already met
                    }

                    // 2. Extract String from Argument (support both BCS String and Raw UTF-8)
                    let parsed_string = bcs::from_bytes::<String>(arg)
                        .or_else(|_| std::str::from_utf8(arg).map(|s| s.to_string()));

                    if let Ok(s) = parsed_string {
                        let s_trim = s.trim();

                        // Force add 0x prefix if missing
                        let hex_str = if !s_trim.starts_with("0x") {
                            format!("0x{}", s_trim)
                        } else {
                            s_trim.to_string()
                        };

                        // Use from_hex_literal to handle length and convert to Lowercase
                        if let Ok(addr) = AccountAddress::from_hex_literal(&hex_str) {
                            keys.push(addr.to_hex_literal());
                        }
                    }
                }
            }
            Transaction::PublishModule { module_name, .. } => {
                keys.push(module_name.clone());
            }
        }
        keys
    }

    pub fn native_call(&self) -> Option<NativeCall> {
        let Transaction::ExecuteFunction {
            module,
            function,
            args,
            ..
        } = self
        else {
            return None;
        };

        if module != Self::KANARI_MODULE {
            return None;
        }

        match function.as_str() {
            Self::TRANSFER_AMOUNT_FUNCTION if args.len() >= 2 => {
                let amount = bcs::from_bytes::<u64>(&args[0]).ok()?;
                let recipient = bcs::from_bytes::<String>(&args[1]).ok()?;
                Some(NativeCall::TransferAmount { recipient, amount })
            }
            Self::BURN_AMOUNT_FUNCTION if !args.is_empty() => {
                let amount = bcs::from_bytes::<u64>(&args[0]).ok()?;
                Some(NativeCall::BurnAmount { amount })
            }
            _ => None,
        }
    }

    pub fn is_native_balance_call(&self) -> bool {
        self.native_call().is_some()
    }

    pub fn tx_type_label(&self) -> &'static str {
        if let Some(native_call) = self.native_call() {
            return native_call.tx_type_label();
        }

        match self {
            Transaction::PublishModule { .. } => "publish_module",
            Transaction::ExecuteFunction { .. } => "call",
        }
    }

    /// Create a transfer transaction with default gas settings
    pub fn new_transfer(from: String, to: String, amount: u64, sequence_number: u64) -> Self {
        Self::new_transfer_with_gas(from, to, amount, sequence_number, 100_000, 1000)
    }

    pub fn new_transfer_with_gas(
        from: String,
        to: String,
        amount: u64,
        sequence_number: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Self {
        Self::ExecuteFunction {
            sender: from,
            module: Self::KANARI_MODULE.to_string(),
            function: Self::TRANSFER_AMOUNT_FUNCTION.to_string(),
            type_args: vec![],
            args: vec![
                bcs::to_bytes(&amount).unwrap_or_default(),
                bcs::to_bytes(&to).unwrap_or_default(),
            ],
            gas_limit,
            gas_price,
            sequence_number,
        }
    }

    /// Create a burn transaction with default gas settings
    pub fn new_burn(from: String, amount: u64, sequence_number: u64) -> Self {
        Self::new_burn_with_gas(from, amount, sequence_number, 100_000, 1000)
    }

    pub fn new_burn_with_gas(
        from: String,
        amount: u64,
        sequence_number: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Self {
        Self::ExecuteFunction {
            sender: from,
            module: Self::KANARI_MODULE.to_string(),
            function: Self::BURN_AMOUNT_FUNCTION.to_string(),
            type_args: vec![],
            args: vec![bcs::to_bytes(&amount).unwrap_or_default()],
            gas_limit,
            gas_price,
            sequence_number,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_helper_builds_native_execute_function() {
        let tx = Transaction::new_transfer("0x1".to_string(), "0x2".to_string(), 42, 7);

        match &tx {
            Transaction::ExecuteFunction {
                module,
                function,
                sequence_number,
                ..
            } => {
                assert_eq!(module, Transaction::KANARI_MODULE);
                assert_eq!(function, Transaction::TRANSFER_AMOUNT_FUNCTION);
                assert_eq!(*sequence_number, 7);
            }
            Transaction::PublishModule { .. } => panic!("transfer helper must build a call"),
        }

        assert_eq!(
            tx.native_call(),
            Some(NativeCall::TransferAmount {
                recipient: "0x2".to_string(),
                amount: 42,
            })
        );
        assert_eq!(tx.tx_type_label(), "transfer");
    }

    #[test]
    fn burn_helper_builds_native_execute_function() {
        let tx = Transaction::new_burn("0x1".to_string(), 9, 3);

        assert_eq!(tx.native_call(), Some(NativeCall::BurnAmount { amount: 9 }));
        assert_eq!(tx.tx_type_label(), "burn");
    }
}
