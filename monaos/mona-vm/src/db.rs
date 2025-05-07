use rocksdb::{DB, Options, ColumnFamilyDescriptor};
use log::{info, error, debug};

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;
use std::sync::{Arc, Mutex, Once};
use bincode;
use serde::{Serialize, Deserialize};


static INIT_DB: Once = Once::new();
lazy_static::lazy_static! {
    static ref DB_CONNECTION: Arc<Mutex<DB>> = Arc::new(Mutex::new(create_connection().expect("Failed to create database connection")));
}

#[derive(Debug)]
pub enum DbError {
    ConnectionFailed(String),
    QueryFailed(String),
    TransactionFailed(String),
    SerializationError(String),
}

// Struct representing a stored module
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredModule {
    pub module_id: String,
    pub address: String,
    pub name: String,
    pub bytecode: Vec<u8>,
    pub abi: String,
    pub deploy_time: u64,
    pub deploy_tx_id: String,
    pub deploy_block_height: u32,
}

// Struct representing a stored transaction
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredTransaction {
    pub tx_id: String,
    pub module_id: String,
    pub function: String,
    pub args: Vec<Vec<u8>>,
    pub sender: String,
    pub timestamp: u64,
    pub gas_used: u64,
    pub success: bool,
    pub result_data: String,
    pub block_height: u64,
}

// Create database connection and initialize
fn create_connection() -> Result<DB, DbError> {
    // Get database directory from common config
    let db_dir = get_db_directory();
    
    // Create directory if it doesn't exist
    if !db_dir.exists() {
        if let Err(e) = fs::create_dir_all(&db_dir) {
            return Err(DbError::ConnectionFailed(format!(
                "Failed to create database directory: {}", e
            )));
        }
    }
    
    let db_path = db_dir.join("db_mvsm_rocks");
    debug!("Opening RocksDB database at: {}", db_path.display());
    
    // Configure RocksDB options
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    
    // Define column families (replacing tables in SQLite)
    let cf_modules = ColumnFamilyDescriptor::new("modules", Options::default());
    let cf_transactions = ColumnFamilyDescriptor::new("transactions", Options::default());
    let cf_indexes = ColumnFamilyDescriptor::new("indexes", Options::default());
    let cf_descriptors = vec![cf_modules, cf_transactions, cf_indexes];
    
    // Open RocksDB database
    match DB::open_cf_descriptors(&opts, &db_path, cf_descriptors) {
        Ok(db) => {
            INIT_DB.call_once(|| {
                info!("RocksDB database initialized successfully");
            });
            Ok(db)
        },
        Err(e) => Err(DbError::ConnectionFailed(format!(
            "Failed to open RocksDB database: {}", e
        ))),
    }
}

// Get database directory
fn get_db_directory() -> PathBuf {
    let kari_dir = common::get_kari_dir();
    kari_dir.join("db")
}

// Store deployed module in database
pub fn store_module(
    module_id: &str,
    address: &str,
    name: &str,
    bytecode: &[u8],
    abi: &str,
    deploy_tx_id: &str,
    deploy_block_height: u32,
) -> Result<(), DbError> {
    let conn = match DB_CONNECTION.lock() {
        Ok(conn) => conn,
        Err(e) => return Err(DbError::ConnectionFailed(format!(
            "Failed to get database connection lock: {}", e
        ))),
    };
    
    let deploy_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // Create module record
    let module = StoredModule {
        module_id: module_id.to_string(),
        address: address.to_string(),
        name: name.to_string(),
        bytecode: bytecode.to_vec(),
        abi: abi.to_string(),
        deploy_time,
        deploy_tx_id: deploy_tx_id.to_string(),
        deploy_block_height,
    };
    
    // Serialize module
    let serialized = bincode::serialize(&module)
        .map_err(|e| DbError::SerializationError(format!(
            "Failed to serialize module {}: {}", module_id, e
        )))?;
    
    // Get column family handle
    let cf_modules = conn.cf_handle("modules")
        .ok_or_else(|| DbError::QueryFailed("Modules column family not found".to_string()))?;
    
    // Store in RocksDB
    conn.put_cf(&cf_modules, module_id.as_bytes(), serialized)
        .map_err(|e| DbError::QueryFailed(format!(
            "Failed to store module {}: {}", module_id, e
        )))?;
    
    // Create module by address index
    let address_key = format!("address:{}", address);
    let cf_indexes = conn.cf_handle("indexes")
        .ok_or_else(|| DbError::QueryFailed("Indexes column family not found".to_string()))?;
    
    // Append to list of modules by address
    let modules_by_address = match conn.get_cf(&cf_indexes, address_key.as_bytes()) {
        Ok(Some(data)) => {
            let mut modules: Vec<String> = bincode::deserialize(&data)
                .map_err(|e| DbError::SerializationError(format!(
                    "Failed to deserialize modules by address index: {}", e
                )))?;
            if !modules.contains(&module_id.to_string()) {
                modules.push(module_id.to_string());
            }
            modules
        },
        _ => vec![module_id.to_string()],
    };
    
    let serialized_modules = bincode::serialize(&modules_by_address)
        .map_err(|e| DbError::SerializationError(format!(
            "Failed to serialize modules by address index: {}", e
        )))?;
    
    conn.put_cf(&cf_indexes, address_key.as_bytes(), serialized_modules)
        .map_err(|e| DbError::QueryFailed(format!(
            "Failed to update address index for module {}: {}", module_id, e
        )))?;
    
    info!("Module {} stored in database", module_id);
    Ok(())
}

// Store transaction in database
pub fn store_transaction(
    tx_id: &str,
    module_id: &str,
    function: &str,
    args: &[Vec<u8>],
    sender: &str,
    gas_used: u64,
    success: bool,
    result_data: &str,
    block_height: u64,
) -> Result<(), DbError> {
    let conn = match DB_CONNECTION.lock() {
        Ok(conn) => conn,
        Err(e) => return Err(DbError::ConnectionFailed(format!(
            "Failed to get database connection lock: {}", e
        ))),
    };
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // Create transaction record
    let transaction = StoredTransaction {
        tx_id: tx_id.to_string(),
        module_id: module_id.to_string(),
        function: function.to_string(),
        args: args.to_vec(),
        sender: sender.to_string(),
        timestamp,
        gas_used,
        success,
        result_data: result_data.to_string(),
        block_height,
    };
    
    // Serialize transaction
    let serialized = bincode::serialize(&transaction)
        .map_err(|e| DbError::SerializationError(format!(
            "Failed to serialize transaction {}: {}", tx_id, e
        )))?;
    
    // Get column family handle
    let cf_transactions = conn.cf_handle("transactions")
        .ok_or_else(|| DbError::QueryFailed("Transactions column family not found".to_string()))?;
    
    // Store in RocksDB
    conn.put_cf(&cf_transactions, tx_id.as_bytes(), serialized)
        .map_err(|e| DbError::QueryFailed(format!(
            "Failed to store transaction {}: {}", tx_id, e
        )))?;
    
    // Index transactions by module
    update_module_transaction_index(&conn, module_id, tx_id, timestamp)?;
    
    // Index transactions by sender
    update_sender_transaction_index(&conn, sender, tx_id, timestamp)?;
    
    debug!("Transaction {} ({}.{}) stored in database", tx_id, module_id, function);
    Ok(())
}

// Helper function to update module transaction index
fn update_module_transaction_index(
    conn: &DB,
    module_id: &str, 
    tx_id: &str, 
    timestamp: u64
) -> Result<(), DbError> {
    let cf_indexes = conn.cf_handle("indexes")
        .ok_or_else(|| DbError::QueryFailed("Indexes column family not found".to_string()))?;
    
    let module_tx_key = format!("module_txs:{}", module_id);
    
    // Get existing transactions or create new list
    let tx_list = match conn.get_cf(&cf_indexes, module_tx_key.as_bytes()) {
        Ok(Some(data)) => {
            let mut txs: Vec<(String, u64)> = bincode::deserialize(&data)
                .map_err(|e| DbError::SerializationError(format!(
                    "Failed to deserialize module transactions index: {}", e
                )))?;
            txs.push((tx_id.to_string(), timestamp));
            // Sort by timestamp descending for easier pagination later
            txs.sort_by(|a, b| b.1.cmp(&a.1));
            txs
        },
        _ => vec![(tx_id.to_string(), timestamp)],
    };
    
    let serialized_txs = bincode::serialize(&tx_list)
        .map_err(|e| DbError::SerializationError(format!(
            "Failed to serialize module transactions index: {}", e
        )))?;
    
    conn.put_cf(&cf_indexes, module_tx_key.as_bytes(), serialized_txs)
        .map_err(|e| DbError::QueryFailed(format!(
            "Failed to update module transactions index: {}", e
        )))?;
    
    Ok(())
}

// Helper function to update sender transaction index
fn update_sender_transaction_index(
    conn: &DB,
    sender: &str, 
    tx_id: &str, 
    timestamp: u64
) -> Result<(), DbError> {
    let cf_indexes = conn.cf_handle("indexes")
        .ok_or_else(|| DbError::QueryFailed("Indexes column family not found".to_string()))?;
    
    let sender_tx_key = format!("sender_txs:{}", sender);
    
    // Get existing transactions or create new list
    let tx_list = match conn.get_cf(&cf_indexes, sender_tx_key.as_bytes()) {
        Ok(Some(data)) => {
            let mut txs: Vec<(String, u64)> = bincode::deserialize(&data)
                .map_err(|e| DbError::SerializationError(format!(
                    "Failed to deserialize sender transactions index: {}", e
                )))?;
            txs.push((tx_id.to_string(), timestamp));
            // Sort by timestamp descending for easier pagination later
            txs.sort_by(|a, b| b.1.cmp(&a.1));
            txs
        },
        _ => vec![(tx_id.to_string(), timestamp)],
    };
    
    let serialized_txs = bincode::serialize(&tx_list)
        .map_err(|e| DbError::SerializationError(format!(
            "Failed to serialize sender transactions index: {}", e
        )))?;
    
    conn.put_cf(&cf_indexes, sender_tx_key.as_bytes(), serialized_txs)
        .map_err(|e| DbError::QueryFailed(format!(
            "Failed to update sender transactions index: {}", e
        )))?;
    
    Ok(())
}

// Get module by ID
pub fn get_module(module_id: &str) -> Result<Option<StoredModule>, DbError> {
    let conn = match DB_CONNECTION.lock() {
        Ok(conn) => conn,
        Err(e) => return Err(DbError::ConnectionFailed(format!(
            "Failed to get database connection lock: {}", e
        ))),
    };
    
    let cf_modules = conn.cf_handle("modules")
        .ok_or_else(|| DbError::QueryFailed("Modules column family not found".to_string()))?;
    
    match conn.get_cf(&cf_modules, module_id.as_bytes()) {
        Ok(Some(data)) => {
            let module: StoredModule = bincode::deserialize(&data)
                .map_err(|e| DbError::SerializationError(format!(
                    "Failed to deserialize module {}: {}", module_id, e
                )))?;
            Ok(Some(module))
        },
        Ok(None) => Ok(None),
        Err(e) => Err(DbError::QueryFailed(format!(
            "Failed to query module {}: {}", module_id, e
        ))),
    }
}

// Get transactions for a module with pagination
pub fn get_module_transactions(
    module_id: &str,
    limit: usize,
    offset: usize
) -> Result<Vec<StoredTransaction>, DbError> {
    let conn = match DB_CONNECTION.lock() {
        Ok(conn) => conn,
        Err(e) => return Err(DbError::ConnectionFailed(format!(
            "Failed to get database connection lock: {}", e
        ))),
    };
    
    let cf_indexes = conn.cf_handle("indexes")
        .ok_or_else(|| DbError::QueryFailed("Indexes column family not found".to_string()))?;
    
    let module_tx_key = format!("module_txs:{}", module_id);
    
    // Get transaction IDs for this module
    let tx_ids = match conn.get_cf(&cf_indexes, module_tx_key.as_bytes()) {
        Ok(Some(data)) => {
            let all_txs: Vec<(String, u64)> = bincode::deserialize(&data)
                .map_err(|e| DbError::SerializationError(format!(
                    "Failed to deserialize module transactions index: {}", e
                )))?;
            
            // Apply pagination
            all_txs.iter()
                .skip(offset)
                .take(limit)
                .map(|(id, _)| id.clone())
                .collect::<Vec<String>>()
        },
        Ok(None) => return Ok(Vec::new()),
        Err(e) => return Err(DbError::QueryFailed(format!(
            "Failed to query module transactions index: {}", e
        ))),
    };
    
    // Load transactions by ID
    let mut transactions = Vec::new();
    let cf_transactions = conn.cf_handle("transactions")
        .ok_or_else(|| DbError::QueryFailed("Transactions column family not found".to_string()))?;
    
    for tx_id in tx_ids {
        match conn.get_cf(&cf_transactions, tx_id.as_bytes()) {
            Ok(Some(data)) => {
                match bincode::deserialize(&data) {
                    Ok(tx) => transactions.push(tx),
                    Err(e) => error!("Error deserializing transaction {}: {}", tx_id, e),
                }
            },
            Ok(None) => debug!("Transaction {} not found", tx_id),
            Err(e) => error!("Error retrieving transaction {}: {}", tx_id, e),
        }
    }
    
    Ok(transactions)
}

// Get transactions by sender with pagination
pub fn get_transactions_by_sender(
    sender: &str,
    limit: usize, 
    offset: usize
) -> Result<Vec<StoredTransaction>, DbError> {
    let conn = match DB_CONNECTION.lock() {
        Ok(conn) => conn,
        Err(e) => return Err(DbError::ConnectionFailed(format!(
            "Failed to get database connection lock: {}", e
        ))),
    };
    
    let cf_indexes = conn.cf_handle("indexes")
        .ok_or_else(|| DbError::QueryFailed("Indexes column family not found".to_string()))?;
    
    let sender_tx_key = format!("sender_txs:{}", sender);
    
    // Get transaction IDs for this sender
    let tx_ids = match conn.get_cf(&cf_indexes, sender_tx_key.as_bytes()) {
        Ok(Some(data)) => {
            let all_txs: Vec<(String, u64)> = bincode::deserialize(&data)
                .map_err(|e| DbError::SerializationError(format!(
                    "Failed to deserialize sender transactions index: {}", e
                )))?;
            
            // Apply pagination
            all_txs.iter()
                .skip(offset)
                .take(limit)
                .map(|(id, _)| id.clone())
                .collect::<Vec<String>>()
        },
        Ok(None) => return Ok(Vec::new()),
        Err(e) => return Err(DbError::QueryFailed(format!(
            "Failed to query sender transactions index: {}", e
        ))),
    };
    
    // Load transactions by ID
    let mut transactions = Vec::new();
    let cf_transactions = conn.cf_handle("transactions")
        .ok_or_else(|| DbError::QueryFailed("Transactions column family not found".to_string()))?;
    
    for tx_id in tx_ids {
        match conn.get_cf(&cf_transactions, tx_id.as_bytes()) {
            Ok(Some(data)) => {
                match bincode::deserialize(&data) {
                    Ok(tx) => transactions.push(tx),
                    Err(e) => error!("Error deserializing transaction {}: {}", tx_id, e),
                }
            },
            Ok(None) => debug!("Transaction {} not found", tx_id),
            Err(e) => error!("Error retrieving transaction {}: {}", tx_id, e),
        }
    }
    
    Ok(transactions)
}

// Get transaction by ID
pub fn get_transaction(tx_id: &str) -> Result<Option<StoredTransaction>, DbError> {
    let conn = match DB_CONNECTION.lock() {
        Ok(conn) => conn,
        Err(e) => return Err(DbError::ConnectionFailed(format!(
            "Failed to get database connection lock: {}", e
        ))),
    };
    
    let cf_transactions = conn.cf_handle("transactions")
        .ok_or_else(|| DbError::QueryFailed("Transactions column family not found".to_string()))?;
    
    match conn.get_cf(&cf_transactions, tx_id.as_bytes()) {
        Ok(Some(data)) => {
            let transaction: StoredTransaction = bincode::deserialize(&data)
                .map_err(|e| DbError::SerializationError(format!(
                    "Failed to deserialize transaction {}: {}", tx_id, e
                )))?;
            Ok(Some(transaction))
        },
        Ok(None) => Ok(None),
        Err(e) => Err(DbError::QueryFailed(format!(
            "Failed to query transaction {}: {}", tx_id, e
        ))),
    }
}

// Get module count
pub fn get_module_count() -> Result<usize, DbError> {
    let conn = match DB_CONNECTION.lock() {
        Ok(conn) => conn,
        Err(e) => return Err(DbError::ConnectionFailed(format!(
            "Failed to get database connection lock: {}", e
        ))),
    };
    
    let cf_modules = conn.cf_handle("modules")
        .ok_or_else(|| DbError::QueryFailed("Modules column family not found".to_string()))?;
    
    // For RocksDB, we need to iterate through all keys
    let mut count = 0;
    
    // Create an iterator over the modules column family
    let iter = conn.iterator_cf(&cf_modules, rocksdb::IteratorMode::Start);
    for result in iter {
        match result {
            Ok(_) => count += 1,
            Err(e) => error!("Error iterating modules: {}", e),
        }
    }
    
    Ok(count)
}

// Get transaction count
pub fn get_transaction_count() -> Result<usize, DbError> {
    let conn = match DB_CONNECTION.lock() {
        Ok(conn) => conn,
        Err(e) => return Err(DbError::ConnectionFailed(format!(
            "Failed to get database connection lock: {}", e
        ))),
    };
    
    let cf_transactions = conn.cf_handle("transactions")
        .ok_or_else(|| DbError::QueryFailed("Transactions column family not found".to_string()))?;
    
    // For RocksDB, we need to iterate through all keys
    let mut count = 0;
    
    // Create an iterator over the transactions column family
    let iter = conn.iterator_cf(&cf_transactions, rocksdb::IteratorMode::Start);
    for result in iter {
        match result {
            Ok(_) => count += 1,
            Err(e) => error!("Error iterating transactions: {}", e),
        }
    }
    
    Ok(count)
}

// Get all modules with pagination
pub fn list_modules(
    limit: usize,
    offset: usize
) -> Result<Vec<StoredModule>, DbError> {
    let conn = match DB_CONNECTION.lock() {
        Ok(conn) => conn,
        Err(e) => return Err(DbError::ConnectionFailed(format!(
            "Failed to get database connection lock: {}", e
        ))),
    };
    
    let cf_modules = conn.cf_handle("modules")
        .ok_or_else(|| DbError::QueryFailed("Modules column family not found".to_string()))?;
    
    let mut modules = Vec::new();
    let mut current = 0;
    
    // Create an iterator over the modules column family
    let iter = conn.iterator_cf(&cf_modules, rocksdb::IteratorMode::Start);
    for result in iter {
        match result {
            Ok((_, value)) => {
                if current >= offset {
                    if modules.len() >= limit {
                        break;
                    }
                    
                    match bincode::deserialize(&value) {
                        Ok(module) => modules.push(module),
                        Err(e) => error!("Error deserializing module: {}", e),
                    }
                }
                current += 1;
            },
            Err(e) => error!("Error iterating modules: {}", e),
        }
    }
    
    Ok(modules)
}
