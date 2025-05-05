use rusqlite::{Connection, Result, params};
use log::{info, error, debug};
use serde_json::Value;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;
use std::sync::{Arc, Mutex, Once};

use crate::VMTransaction;

static INIT_DB: Once = Once::new();
lazy_static::lazy_static! {
    static ref DB_CONNECTION: Arc<Mutex<Connection>> = Arc::new(Mutex::new(create_connection().expect("Failed to create database connection")));
}

#[derive(Debug)]
pub enum DbError {
    ConnectionFailed(String),
    QueryFailed(String),
    TransactionFailed(String),
    SerializationError(String),
}

// Struct representing a stored module
#[derive(Debug)]
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
#[derive(Debug)]
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

// Create database connection and initialize schema
fn create_connection() -> Result<Connection, DbError> {
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
    
    let db_path = db_dir.join("db_mvsm.sqlite");
    debug!("Opening database at: {}", db_path.display());
    
    // Open database connection
    match Connection::open(&db_path) {
        Ok(conn) => {
            // Initialize schema if needed
            INIT_DB.call_once(|| {
                if let Err(e) = init_db_schema(&conn) {
                    error!("Failed to initialize database schema: {:?}", e);
                }
            });
            Ok(conn)
        },
        Err(e) => Err(DbError::ConnectionFailed(format!(
            "Failed to open database connection: {}", e
        ))),
    }
}

// Get database directory
fn get_db_directory() -> PathBuf {
    let kari_dir = common::get_kari_dir();
    kari_dir.join("db")
}

// Initialize database schema
fn init_db_schema(conn: &Connection) -> Result<(), DbError> {
    info!("Initializing Move VM database schema");
    
    // Create modules table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS modules (
            module_id TEXT PRIMARY KEY,
            address TEXT NOT NULL,
            name TEXT NOT NULL,
            bytecode BLOB NOT NULL,
            abi TEXT NOT NULL,
            deploy_time INTEGER NOT NULL,
            deploy_tx_id TEXT NOT NULL,
            deploy_block_height INTEGER NOT NULL
        )",
        [],
    )
    .map_err(|e| DbError::QueryFailed(format!("Failed to create modules table: {}", e)))?;
    
    // Create transactions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS transactions (
            tx_id TEXT PRIMARY KEY,
            module_id TEXT NOT NULL,
            function TEXT NOT NULL,
            args BLOB NOT NULL,
            sender TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            gas_used INTEGER NOT NULL,
            success INTEGER NOT NULL,
            result_data TEXT NOT NULL,
            block_height INTEGER NOT NULL,
            FOREIGN KEY (module_id) REFERENCES modules (module_id)
        )",
        [],
    )
    .map_err(|e| DbError::QueryFailed(format!("Failed to create transactions table: {}", e)))?;
    
    // Create index on module_id for transactions
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_transactions_module_id ON transactions (module_id)",
        [],
    )
    .map_err(|e| DbError::QueryFailed(format!("Failed to create module_id index: {}", e)))?;
    
    // Create index on timestamp for transactions
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_transactions_timestamp ON transactions (timestamp)",
        [],
    )
    .map_err(|e| DbError::QueryFailed(format!("Failed to create timestamp index: {}", e)))?;
    
    info!("Move VM database schema initialized successfully");
    Ok(())
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
    
    conn.execute(
        "INSERT OR REPLACE INTO modules 
        (module_id, address, name, bytecode, abi, deploy_time, deploy_tx_id, deploy_block_height)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            module_id,
            address,
            name,
            bytecode,
            abi,
            deploy_time,
            deploy_tx_id,
            deploy_block_height
        ],
    )
    .map_err(|e| DbError::QueryFailed(format!(
        "Failed to store module {}: {}", module_id, e
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
    
    // Serialize args to binary
    let serialized_args = match bincode::serialize(args) {
        Ok(data) => data,
        Err(e) => return Err(DbError::SerializationError(format!(
            "Failed to serialize arguments: {}", e
        ))),
    };
    
    conn.execute(
        "INSERT OR REPLACE INTO transactions 
        (tx_id, module_id, function, args, sender, timestamp, gas_used, success, result_data, block_height)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            tx_id,
            module_id,
            function,
            serialized_args,
            sender,
            timestamp,
            gas_used,
            success as i32,
            result_data,
            block_height
        ],
    )
    .map_err(|e| DbError::QueryFailed(format!(
        "Failed to store transaction {}: {}", tx_id, e
    )))?;
    
    debug!("Transaction {} ({}.{}) stored in database", tx_id, module_id, function);
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
    
    let mut stmt = conn.prepare(
        "SELECT module_id, address, name, bytecode, abi, deploy_time, deploy_tx_id, deploy_block_height
         FROM modules WHERE module_id = ?"
    )
    .map_err(|e| DbError::QueryFailed(format!("Failed to prepare query: {}", e)))?;
    
    let module_result = stmt.query_row(params![module_id], |row| {
        Ok(StoredModule {
            module_id: row.get(0)?,
            address: row.get(1)?,
            name: row.get(2)?,
            bytecode: row.get(3)?,
            abi: row.get(4)?,
            deploy_time: row.get(5)?,
            deploy_tx_id: row.get(6)?,
            deploy_block_height: row.get(7)?,
        })
    });
    
    match module_result {
        Ok(module) => Ok(Some(module)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(DbError::QueryFailed(format!(
            "Failed to query module {}: {}", module_id, e
        ))),
    }
}

// Get transactions for a module
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
    
    let mut stmt = conn.prepare(
        "SELECT tx_id, module_id, function, args, sender, timestamp, gas_used, success, result_data, block_height
         FROM transactions 
         WHERE module_id = ?
         ORDER BY timestamp DESC
         LIMIT ? OFFSET ?"
    )
    .map_err(|e| DbError::QueryFailed(format!("Failed to prepare query: {}", e)))?;
    
    let transaction_iter = stmt.query_map(
        params![module_id, limit as i64, offset as i64],
        |row| {
            let args_blob: Vec<u8> = row.get(3)?;
            let args: Vec<Vec<u8>> = match bincode::deserialize(&args_blob) {
                Ok(a) => a,
                Err(_) => Vec::new(), // Return empty vec on error
            };
            
            Ok(StoredTransaction {
                tx_id: row.get(0)?,
                module_id: row.get(1)?,
                function: row.get(2)?,
                args,
                sender: row.get(4)?,
                timestamp: row.get(5)?,
                gas_used: row.get(6)?,
                success: row.get::<_, i32>(7)? != 0,
                result_data: row.get(8)?,
                block_height: row.get(9)?,
            })
        },
    )
    .map_err(|e| DbError::QueryFailed(format!(
        "Failed to query transactions for module {}: {}", module_id, e
    )))?;
    
    let mut transactions = Vec::new();
    for tx in transaction_iter {
        match tx {
            Ok(t) => transactions.push(t),
            Err(e) => error!("Error retrieving transaction: {}", e),
        }
    }
    
    Ok(transactions)
}

// Get transactions by sender
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
    
    let mut stmt = conn.prepare(
        "SELECT tx_id, module_id, function, args, sender, timestamp, gas_used, success, result_data, block_height
         FROM transactions 
         WHERE sender = ?
         ORDER BY timestamp DESC
         LIMIT ? OFFSET ?"
    )
    .map_err(|e| DbError::QueryFailed(format!("Failed to prepare query: {}", e)))?;
    
    let transaction_iter = stmt.query_map(
        params![sender, limit as i64, offset as i64],
        |row| {
            let args_blob: Vec<u8> = row.get(3)?;
            let args: Vec<Vec<u8>> = match bincode::deserialize(&args_blob) {
                Ok(a) => a,
                Err(_) => Vec::new(),
            };
            
            Ok(StoredTransaction {
                tx_id: row.get(0)?,
                module_id: row.get(1)?,
                function: row.get(2)?,
                args,
                sender: row.get(4)?,
                timestamp: row.get(5)?,
                gas_used: row.get(6)?,
                success: row.get::<_, i32>(7)? != 0,
                result_data: row.get(8)?,
                block_height: row.get(9)?,
            })
        },
    )
    .map_err(|e| DbError::QueryFailed(format!(
        "Failed to query transactions for sender {}: {}", sender, e
    )))?;
    
    let mut transactions = Vec::new();
    for tx in transaction_iter {
        match tx {
            Ok(t) => transactions.push(t),
            Err(e) => error!("Error retrieving transaction: {}", e),
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
    
    let mut stmt = conn.prepare(
        "SELECT tx_id, module_id, function, args, sender, timestamp, gas_used, success, result_data, block_height
         FROM transactions WHERE tx_id = ?"
    )
    .map_err(|e| DbError::QueryFailed(format!("Failed to prepare query: {}", e)))?;
    
    let transaction_result = stmt.query_row(params![tx_id], |row| {
        let args_blob: Vec<u8> = row.get(3)?;
        let args: Vec<Vec<u8>> = match bincode::deserialize(&args_blob) {
            Ok(a) => a,
            Err(_) => Vec::new(),
        };
        
        Ok(StoredTransaction {
            tx_id: row.get(0)?,
            module_id: row.get(1)?,
            function: row.get(2)?,
            args,
            sender: row.get(4)?,
            timestamp: row.get(5)?,
            gas_used: row.get(6)?,
            success: row.get::<_, i32>(7)? != 0,
            result_data: row.get(8)?,
            block_height: row.get(9)?,
        })
    });
    
    match transaction_result {
        Ok(tx) => Ok(Some(tx)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
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
    
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM modules",
        [],
        |row| row.get(0),
    )
    .map_err(|e| DbError::QueryFailed(format!("Failed to count modules: {}", e)))?;
    
    Ok(count as usize)
}

// Get transaction count
pub fn get_transaction_count() -> Result<usize, DbError> {
    let conn = match DB_CONNECTION.lock() {
        Ok(conn) => conn,
        Err(e) => return Err(DbError::ConnectionFailed(format!(
            "Failed to get database connection lock: {}", e
        ))),
    };
    
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM transactions",
        [],
        |row| row.get(0),
    )
    .map_err(|e| DbError::QueryFailed(format!("Failed to count transactions: {}", e)))?;
    
    Ok(count as usize)
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
    
    let mut stmt = conn.prepare(
        "SELECT module_id, address, name, bytecode, abi, deploy_time, deploy_tx_id, deploy_block_height
         FROM modules
         ORDER BY deploy_time DESC
         LIMIT ? OFFSET ?"
    )
    .map_err(|e| DbError::QueryFailed(format!("Failed to prepare query: {}", e)))?;
    
    let module_iter = stmt.query_map(
        params![limit as i64, offset as i64],
        |row| {
            Ok(StoredModule {
                module_id: row.get(0)?,
                address: row.get(1)?,
                name: row.get(2)?,
                bytecode: row.get(3)?,
                abi: row.get(4)?,
                deploy_time: row.get(5)?,
                deploy_tx_id: row.get(6)?,
                deploy_block_height: row.get(7)?,
            })
        },
    )
    .map_err(|e| DbError::QueryFailed(format!("Failed to list modules: {}", e)))?;
    
    let mut modules = Vec::new();
    for module in module_iter {
        match module {
            Ok(m) => modules.push(m),
            Err(e) => error!("Error retrieving module: {}", e),
        }
    }
    
    Ok(modules)
}
