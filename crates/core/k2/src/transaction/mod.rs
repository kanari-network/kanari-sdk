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
    pub coin_type: Option<String>,
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
            coin_type: None,
        }
    }

    pub fn apply_transfer(&self, balances: &mut std::collections::HashMap<String, u64>) -> Result<(), String> {
        if let TransactionType::Transfer = self.tx_type {
            let sender_balance = balances.get(&self.sender).cloned().unwrap_or(0);
            if sender_balance < self.amount {
                return Err("Insufficient funds".to_string());
            }
            balances.insert(self.sender.clone(), sender_balance - self.amount);

            let receiver_balance = balances.get(&self.receiver).cloned().unwrap_or(0);
            balances.insert(self.receiver.clone(), receiver_balance + self.amount);

            Ok(())
        } else {
            Err("Not a transfer transaction".to_string())
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
            coin_type: None,
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

    // Add verification functionality
    pub fn apply_coin_transfer(&self, balances: &mut std::collections::HashMap<(String, Option<String>), u64>) -> Result<(), String> {
        if let TransactionType::Transfer = self.tx_type {
            let coin = self.coin_type.clone();
            let sender_account = (self.sender.clone(), coin.clone());
            let receiver_account = (self.receiver.clone(), coin.clone());

            let sender_balance = balances.get(&sender_account).cloned().unwrap_or(0);
            if sender_balance < self.amount {
                return Err("Insufficient funds".to_string());
            }
            balances.insert(sender_account, sender_balance - self.amount);

            let receiver_balance = balances.get(&receiver_account).cloned().unwrap_or(0);
            balances.insert(receiver_account, receiver_balance + self.amount);

            Ok(())
        } else {
            Err("Not a transfer transaction".to_string())
        }
    }
}