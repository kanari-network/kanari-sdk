use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum TransactionType {
    Transfer,
    FileStore,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub sender: String,
    pub receiver: String,
    pub amount: u64,
    pub gas_cost: f64,
    pub timestamp: u64,
    pub signature: Option<String>,
    pub tx_type: TransactionType,
    pub data: Vec<u8>,
    pub coin_type: Option<String>,
    // Add missing fields needed by the VM
    pub hash: String,          // Transaction hash
    pub gas_limit: u64,        // Maximum gas units
    pub gas_price: u64,        // Price per gas unit
    pub nonce: u64,            // Sequence number/nonce
}

impl Transaction {
    // Add a method to serialize the transaction into bytes for VM execution
    pub fn serialize(&self) -> Vec<u8> {
        // Implement proper serialization here
        // This is a simple example - you should use proper serialization
        bincode::serialize(self).unwrap_or_default()
    }
}
