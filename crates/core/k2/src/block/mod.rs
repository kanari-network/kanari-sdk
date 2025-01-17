use crate::{chain_id::CHAIN_ID, transaction::Transaction};
use bcs::to_bytes;
use consensus_pos::HashAlgorithm;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use move_core_types::{
    language_storage::{ModuleId, TypeTag},
    account_address::AccountAddress,
    identifier::Identifier,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MoveModulePublish {
    pub module_id: ModuleId,
    pub module_bytes: Vec<u8>,
    pub publisher: AccountAddress,
}

impl MoveModulePublish {
    pub fn new(
        publisher: AccountAddress,
        module_name: &str,
        module_bytes: Vec<u8>
    ) -> Result<Self, String> {
        let identifier = Identifier::new(module_name)
            .map_err(|e| format!("Invalid module name: {}", e))?;
        
        let module_id = ModuleId::new(publisher, identifier);
        
        Ok(Self {
            module_id,
            module_bytes,
            publisher,
        })
    }

    pub fn to_vm_state(&self) -> MoveVMState {
        let mut state = HashMap::new();
        let mut modules = HashMap::new();
        
        // Serialize ModuleId using BCS
        let module_key = to_bytes(&self.module_id)
            .expect("Failed to serialize ModuleId");
            
        modules.insert(
            module_key,
            self.module_bytes.clone()
        );
        
        state.insert(self.publisher, modules);

        MoveVMState {
            state_updates: state,
            events: vec![],
            module_bytes: Some(self.module_bytes.clone()),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.module_bytes.is_empty() {
            return Err("Module bytecode cannot be empty".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_publish() {
        let publisher = AccountAddress::random();
        let bytes = vec![1, 2, 3, 4];
        
        let publish = MoveModulePublish::new(
            publisher,
            "TestModule",
            bytes.clone()
        ).unwrap();
        
        assert!(publish.validate().is_ok());
        let state = publish.to_vm_state();
        assert!(state.module_bytes.is_some());
    }
}



#[derive(Serialize, Deserialize, Clone)]
pub struct MoveVMState {
    pub state_updates: HashMap<AccountAddress, HashMap<Vec<u8>, Vec<u8>>>,
    pub events: Vec<Vec<u8>>,
    pub module_bytes: Option<Vec<u8>>,
}

// Define the Block struct
#[derive(Serialize, Deserialize, Clone)]
pub struct Block<T: HashAlgorithm> {
    pub chain_id: String,
    pub index: u32,
    pub timestamp: u64,
    pub data: Vec<u8>,
    pub hash: String,
    pub prev_hash: String,
    pub tokens: u64,
    pub token_name: String,
    pub transactions: Vec<Transaction>,
    pub address: String,
    pub hasher: T,
    pub move_state: MoveVMState, // New field
}

// Implement the Block struct
impl<T: HashAlgorithm> Block<T> {
    // Add Move module deployment
    pub fn deploy_move_module(&mut self, module_bytes: Vec<u8>, sender: String) {
        self.data = module_bytes.clone();
        self.transactions.push(Transaction::new_move_deploy(
            sender,
            module_bytes
        ));
        self.hash = self.calculate_hash();
    }

    // Add Move module execution
    pub fn execute_move_module(
        &mut self,
        sender: String,
        module_name: String,
        function_name: String,
        type_tags: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) {
        let transaction = Transaction::new_move_execute(
            sender,
            module_name,
            function_name, 
            type_tags,
            args
        );
        
        self.data = transaction.data.clone();
        self.transactions.push(transaction);
        self.hash = self.calculate_hash();
    }

    pub fn get_move_state(&self) -> &MoveVMState {
        &self.move_state
    }

    pub fn new(
        index: u32,
        data: Vec<u8>,
        prev_hash: String,
        tokens: u64,
        transactions: Vec<Transaction>,
        address: String,
        hasher: T,
    ) -> Block<T> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut block = Block {
            chain_id: CHAIN_ID.to_string(),
            index,
            timestamp,
            data,
            hash: String::new(),
            prev_hash,
            tokens,
            token_name: String::from("Kanari"),
            transactions,
            address,
            hasher,
            move_state: MoveVMState {
                state_updates: HashMap::new(),
                events: Vec::new(),
                module_bytes: None,
            },
        };
        block.hash = block.calculate_hash();
        block
    }

    // Add a method to calculate the hash of the block
    pub fn calculate_hash(&self) -> String {
        let mut input = Vec::new();
        input.extend_from_slice(self.chain_id.as_bytes());
        input.extend_from_slice(&self.index.to_le_bytes());
        input.extend_from_slice(&self.timestamp.to_le_bytes());
        input.extend_from_slice(&self.data);
        input.extend_from_slice(self.prev_hash.as_bytes());
        input.extend_from_slice(&self.tokens.to_le_bytes());
        input.extend_from_slice(self.token_name.as_bytes());

        // Serialize transactions
        let transactions_serialized = serde_json::to_string(&self.transactions).unwrap();
        input.extend_from_slice(transactions_serialized.as_bytes());

        // self.hasher.log_input(&input);
        self.hasher.hash(&input)
    }

    // Add a method to verify the block
    pub fn verify(&self, prev_block: &Block<T>) -> bool {
        if self.index != prev_block.index + 1 {
            return false;
        }
        if self.prev_hash != prev_block.hash {
            return false;
        }

        let calculated_hash = self.calculate_hash();
        if self.hash != calculated_hash {
            return false;
        }
        true
    }
}

