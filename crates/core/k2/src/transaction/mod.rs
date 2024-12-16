use crate::gas::{
    TRANSACTION_GAS_COST, 
    MOVE_MODULE_DEPLOY_GAS,
    MOVE_FUNCTION_CALL_GAS
};
use serde::{Serialize, Deserialize};
use secp256k1::{Secp256k1, Message, SecretKey};
use sha2::{Sha256, Digest};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TransactionType {
    Transfer,
    MoveModuleDeploy(Vec<u8>),
    MoveFunctionCall {
        module_id: String,
        function: String,
        args: Vec<String>
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub sender: String,
    pub receiver: String, 
    pub amount: u64,
    pub gas_cost: f64,
    pub timestamp: u64,
    pub signature: Option<String>,
    pub tx_type: TransactionType, // Add transaction type
}

impl Transaction {
    pub fn new(sender: String, receiver: String, amount: u64) -> Self {
        Self {
            sender,
            receiver,
            amount,
            gas_cost: TRANSACTION_GAS_COST,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            signature: None,
            tx_type: TransactionType::Transfer,
        }
    }

    pub fn calculate_total_cost(&self) -> u64 {
        self.amount + self.gas_cost as u64
    }

    // Add signing functionality
    pub fn sign(&mut self, secp: &Secp256k1<secp256k1::All>, private_key: &[u8]) -> Result<String, String> {
        let tx_hash = self.hash();
        let message = Message::from_digest_slice(&tx_hash)
            .map_err(|e| format!("Message creation error: {}", e))?;
        
        let secret_key = SecretKey::from_slice(private_key)
            .map_err(|e| format!("Invalid private key: {}", e))?;
            
        let signature = secp.sign_ecdsa(&message, &secret_key);
        let signature_hex = hex::encode(signature.serialize_compact());
        self.signature = Some(signature_hex.clone());
        
        Ok(signature_hex)
    }

    // Add hash calculation
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}{}{}{}", 
            self.sender,
            self.receiver,
            self.amount,
            self.gas_cost,
            self.timestamp
        ));
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    pub fn new_move_deploy(sender: String, module_bytes: Vec<u8>) -> Self {
        Self {
            sender,
            receiver: "system".to_string(),
            amount: 0,
            gas_cost: MOVE_MODULE_DEPLOY_GAS,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            signature: None,
            tx_type: TransactionType::MoveModuleDeploy(module_bytes),
        }
    }

    pub fn new_move_call(
        sender: String,
        module_id: String,
        function: String,
        args: Vec<String>
    ) -> Self {
        Self {
            sender,
            receiver: module_id.clone(),
            amount: 0,
            gas_cost: MOVE_FUNCTION_CALL_GAS,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            signature: None,
            tx_type: TransactionType::MoveFunctionCall {
                module_id,
                function,
                args
            },
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_initialization() {
        let tx = Transaction::new(String::from("Alice"), String::from("Bob"), 10);
        assert_eq!(tx.sender, "Alice");
        assert_eq!(tx.receiver, "Bob");
        assert_eq!(tx.amount, 10);
        assert_eq!(tx.gas_cost, TRANSACTION_GAS_COST);
        assert!(tx.timestamp > 0);
        assert!(tx.signature.is_none());
    }

    #[test]
    fn test_calculate_total_cost() {
        let tx = Transaction::new(String::from("Alice"), String::from("Bob"), 10);
        let expected_total_cost = 10 + TRANSACTION_GAS_COST as u64;
        assert_eq!(tx.calculate_total_cost(), expected_total_cost);
    }

    #[test]
    fn test_transaction_signing() {
        let secp = Secp256k1::new();
        let private_key = [1u8; 32];
        let mut tx = Transaction::new(String::from("Alice"), String::from("Bob"), 10);
        
        let signature = tx.sign(&secp, &private_key);
        assert!(signature.is_ok());
        assert!(tx.signature.is_some());
    }
}