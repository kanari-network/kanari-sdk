use crate::gas::{
    TRANSACTION_GAS_COST, 
    MOVE_MODULE_DEPLOY_GAS,
};
use move_core_types::language_storage::TypeTag;
use serde::{Serialize, Deserialize};
use secp256k1::{Secp256k1, Message, SecretKey};
use sha2::{Sha256, Digest};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum TransactionType {
    Transfer,
    MoveModuleDeploy(Vec<u8>),
    MoveExecute {
        module_name: String,
        function_name: String,
        type_tags: Vec<TypeTag>,
        args: Vec<Vec<u8>>
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MoveModuleTransaction {
    pub sender: String,
    pub module_bytes: Vec<u8>,
    pub gas_cost: f64,
    pub timestamp: u64,
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
}

impl Transaction {
    pub fn new_move_execute(
        sender: String,
        module_name: String, 
        function_name: String,
        type_tags: Vec<TypeTag>,
        args: Vec<Vec<u8>>
    ) -> Self {
        let mut data = Vec::new();
        data.extend_from_slice(module_name.as_bytes());
        data.extend_from_slice(function_name.as_bytes());
        data.extend_from_slice(&bcs::to_bytes(&type_tags).unwrap());
        data.extend_from_slice(&bcs::to_bytes(&args).unwrap());

        Transaction {
            sender,
            receiver: String::new(),
            amount: 0,
            gas_cost: 0.0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            signature: None,
            tx_type: TransactionType::MoveExecute {
                module_name,
                function_name,
                type_tags,
                args
            },
            data,
        }
    }

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
            data: Vec::new(),
        }
    }

    pub fn new_move_deploy(sender: String, module_bytes: Vec<u8>) -> Self {
        Self {
            sender,
            receiver: String::new(),
            amount: 0,
            gas_cost: MOVE_MODULE_DEPLOY_GAS,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            signature: None,
            tx_type: TransactionType::MoveModuleDeploy(module_bytes.clone()),
            data: module_bytes,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_transaction() {
        let tx = Transaction::new(
            "sender123".to_string(),
            "receiver456".to_string(),
            100
        );
        
        assert_eq!(tx.sender, "sender123");
        assert_eq!(tx.receiver, "receiver456");
        assert_eq!(tx.amount, 100);
        assert_eq!(tx.tx_type, TransactionType::Transfer);
        assert!(tx.signature.is_none());
    }

    #[test]
    fn test_move_module_deploy() {
        let module_bytes = vec![1, 2, 3, 4];
        let tx = Transaction::new_move_deploy(
            "deployer123".to_string(),
            module_bytes.clone()
        );

        assert_eq!(tx.sender, "deployer123");
        assert_eq!(tx.amount, 0);
        assert!(matches!(tx.tx_type, TransactionType::MoveModuleDeploy(_)));
        assert_eq!(tx.data, module_bytes);
    }

    #[test]
    fn test_move_execute() {
        let module_name = "Test".to_string();
        let function_name = "run".to_string();
        let type_tags = vec![];
        let args = vec![vec![1, 2, 3]];

        let tx = Transaction::new_move_execute(
            "executor123".to_string(),
            module_name.clone(),
            function_name.clone(),
            type_tags.clone(),
            args.clone()
        );

        if let TransactionType::MoveExecute { 
            module_name: m, 
            function_name: f,
            type_tags: t,
            args: a 
        } = tx.tx_type {
            assert_eq!(m, module_name);
            assert_eq!(f, function_name);
            assert_eq!(t, type_tags);
            assert_eq!(a, args);
        } else {
            panic!("Wrong transaction type");
        }
    }

    #[test]
    fn test_transaction_hash() {
        let tx = Transaction::new(
            "sender123".to_string(),
            "receiver456".to_string(),
            100
        );
        
        let hash = tx.hash();
        assert_eq!(hash.len(), 32);
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn test_transaction_serialization() {
        let tx = Transaction::new(
            "sender123".to_string(),
            "receiver456".to_string(),
            100
        );
        
        let serialized = serde_json::to_string(&tx).unwrap();
        let deserialized: Transaction = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(tx.sender, deserialized.sender);
        assert_eq!(tx.receiver, deserialized.receiver);
        assert_eq!(tx.amount, deserialized.amount);
        assert_eq!(tx.tx_type, deserialized.tx_type);
    }
}