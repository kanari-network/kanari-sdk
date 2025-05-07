use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use mona_storage::{BlockchainStorage, RocksDBStorage, StorageError};
use sha3::{Digest, Sha3_256};

// Import common crate to get kari directory
use common::get_kari_dir;

const MODULE_PREFIX: &[u8] = b"vm_module_";
const TRANSACTION_PREFIX: &[u8] = b"vm_tx_";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredModule {
    pub module_id: String,
    pub address: String,
    pub name: String,
    pub bytecode: Vec<u8>,
    pub abi: String,
    pub transaction_id: String,           // Original transaction ID field
    pub deploy_tx_id: String,             // Alias for transaction_id
    pub deploy_block_height: u32,
    pub timestamp: u64,                   // Original timestamp field
    pub deploy_time: u64,                 // Alias for timestamp
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredTransaction {
    pub tx_id: String,
    pub module_id: String,
    pub function: String,
    pub args: Vec<Vec<u8>>,
    pub sender: String,
    pub gas_used: u64,
    pub success: bool,
    pub result_data: String,              // Renamed from result to result_data
    pub block_height: u64,
    pub timestamp: u64,
}

// Helper function to get storage instance
fn get_storage() -> Result<RocksDBStorage, StorageError> {
    let kari_dir = get_kari_dir();
    let db_path = kari_dir.join("vm_db");
    debug!("Opening VM database at: {:?}", db_path);
    RocksDBStorage::new(db_path)
}

// List modules with pagination
pub fn list_modules(limit: usize, offset: usize) -> Result<Vec<StoredModule>, StorageError> {
    let storage = get_storage()?;
    let mut result = Vec::new();
    let mut count = 0;
    
    // Iterate through keys with MODULE_PREFIX
    for i in 0..10000 {  // Reasonable upper limit to prevent infinite loops
        let key = [MODULE_PREFIX, format!("{:010}", i).as_bytes()].concat();
        
        match storage.load_data(&key) {
            Ok(Some(data)) => {
                count += 1;
                
                // Skip if before offset
                if count <= offset {
                    continue;
                }
                
                // Deserialize module
                match bincode::deserialize::<StoredModule>(&data) {
                    Ok(module) => {
                        result.push(module);
                        if result.len() >= limit {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to deserialize module: {}", e);
                    }
                }
            }
            Ok(None) => {
                // No more modules
                break;
            }
            Err(e) => {
                error!("Error loading module at index {}: {}", i, e);
                break;
            }
        }
    }
    
    debug!("Listed {} modules", result.len());
    Ok(result)
}

// Get module by ID
pub fn get_module(module_id: &str) -> Result<Option<StoredModule>, StorageError> {
    let storage = get_storage()?;
    
    // Hash the module_id to create a deterministic key
    let key = module_id_to_key(module_id);
    debug!("Looking up module with key: {:?}", hex::encode(&key));
    
    match storage.load_data(&key) {
        Ok(Some(data)) => {
            match bincode::deserialize::<StoredModule>(&data) {
                Ok(module) => {
                    debug!("Found module: {}", module_id);
                    Ok(Some(module))
                }
                Err(e) => {
                    error!("Failed to deserialize module {}: {}", module_id, e);
                    Err(StorageError::SerializationError(e))
                }
            }
        }
        Ok(None) => {
            debug!("Module not found: {}", module_id);
            Ok(None)
        }
        Err(e) => {
            error!("Error loading module {}: {}", module_id, e);
            Err(e)
        }
    }
}

// Store module
pub fn store_module(
    module_id: &str,
    address: &str,
    name: &str,
    bytecode: &[u8],
    abi: &str,
    transaction_id: &str,
    deploy_block_height: u32
) -> Result<(), StorageError> {
    let storage = get_storage()?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let module = StoredModule {
        module_id: module_id.to_string(),
        address: address.to_string(),
        name: name.to_string(),
        bytecode: bytecode.to_vec(),
        abi: abi.to_string(),
        transaction_id: transaction_id.to_string(),
        deploy_tx_id: transaction_id.to_string(),  // Alias field
        deploy_block_height,
        timestamp,
        deploy_time: timestamp,  // Alias field
    };
    
    // Generate a unique key for this module
    let key = module_id_to_key(module_id);
    
    // Serialize and store module
    let data = bincode::serialize(&module)?;
    storage.save_data(&key, &data)?;
    
    // Also store under a sequential key for listing
    let modules = list_modules(1, 0)?;
    let index = modules.len();
    let seq_key = [MODULE_PREFIX, format!("{:010}", index).as_bytes()].concat();
    storage.save_data(&seq_key, &data)?;
    
    debug!("Stored module {} successfully", module_id);
    storage.flush()?;
    
    Ok(())
}

// Store transaction
pub fn store_transaction(
    transaction_id: &str,
    module_id: &str,
    function: &str,
    args: &[Vec<u8>],
    sender: &str,
    gas_used: u64,
    success: bool,
    result: &str,
    block_height: u64
) -> Result<(), StorageError> {
    let storage = get_storage()?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let transaction = StoredTransaction {
        tx_id: transaction_id.to_string(),
        module_id: module_id.to_string(),
        function: function.to_string(),
        args: args.to_vec(),
        sender: sender.to_string(),
        gas_used,
        success,
        result_data: result.to_string(),
        block_height,
        timestamp,
    };
    
    // Generate a key for this transaction
    let key = [TRANSACTION_PREFIX, transaction_id.as_bytes()].concat();
    
    // Serialize and store transaction
    let data = bincode::serialize(&transaction)?;
    storage.save_data(&key, &data)?;
    
    debug!("Stored transaction {} successfully", transaction_id);
    storage.flush()?;
    
    Ok(())
}

// NEW: Get transaction by ID
pub fn get_transaction(tx_id: &str) -> Result<Option<StoredTransaction>, StorageError> {
    let storage = get_storage()?;
    
    // Generate transaction key
    let key = [TRANSACTION_PREFIX, tx_id.as_bytes()].concat();
    debug!("Looking up transaction with key: {:?}", hex::encode(&key));
    
    match storage.load_data(&key) {
        Ok(Some(data)) => {
            match bincode::deserialize::<StoredTransaction>(&data) {
                Ok(transaction) => {
                    debug!("Found transaction: {}", tx_id);
                    Ok(Some(transaction))
                }
                Err(e) => {
                    error!("Failed to deserialize transaction {}: {}", tx_id, e);
                    Err(StorageError::SerializationError(e))
                }
            }
        }
        Ok(None) => {
            debug!("Transaction not found: {}", tx_id);
            Ok(None)
        }
        Err(e) => {
            error!("Error loading transaction {}: {}", tx_id, e);
            Err(e)
        }
    }
}

// NEW: Get transactions related to a module
pub fn get_module_transactions(module_id: &str, limit: usize, offset: usize) -> Result<Vec<StoredTransaction>, StorageError> {
    let storage = get_storage()?;
    let mut transactions = Vec::new();
    let mut count = 0;
    
    // This is a simple implementation that scans all transactions
    // A production system would use a more efficient indexing approach
    let all_keys = storage.list_keys_with_prefix(TRANSACTION_PREFIX)?;
    
    for key in all_keys {
        match storage.load_data(&key) {
            Ok(Some(data)) => {
                match bincode::deserialize::<StoredTransaction>(&data) {
                    Ok(tx) => {
                        if tx.module_id == module_id {
                            count += 1;
                            if count > offset && transactions.len() < limit {
                                transactions.push(tx);
                            }
                        }
                    },
                    Err(e) => {
                        error!("Failed to deserialize transaction: {}", e);
                    }
                }
            },
            Ok(None) => (),
            Err(e) => {
                error!("Error loading transaction: {}", e);
            }
        }
    }
    
    // Sort by timestamp, newest first
    transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    debug!("Found {} transactions for module {}", transactions.len(), module_id);
    
    Ok(transactions)
}

// Helper function to generate a deterministic key from module_id
fn module_id_to_key(module_id: &str) -> Vec<u8> {
    // Create a hash of the module_id for consistent lookup
    let mut hasher = Sha3_256::new();
    hasher.update(module_id.as_bytes());
    let hash = hasher.finalize();
    
    // Combine prefix and hash
    let mut key = Vec::with_capacity(MODULE_PREFIX.len() + hash.len());
    key.extend_from_slice(MODULE_PREFIX);
    key.extend_from_slice(&hash);
    key
}
