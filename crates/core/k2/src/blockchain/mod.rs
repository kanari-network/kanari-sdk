use crate::block::Block;
use bincode;
use consensus_pos::Blake3Algorithm;
use dirs;
use rocksdb::*;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::ptr::addr_of;
use std::sync::Mutex;

// Define global variables for the blockchain
pub static mut BALANCES: Option<Mutex<HashMap<String, u64>>> = None;
pub static mut TOTAL_TOKENS: u64 = 0;
pub static mut BLOCKCHAIN: VecDeque<Block<Blake3Algorithm>> = VecDeque::new();
pub static mut MOVE_MODULES: Option<Mutex<HashMap<String, Vec<u8>>>> = None;



pub fn get_kari_dir() -> PathBuf {
    let mut path = dirs::home_dir().expect("Unable to find home directory");
    path.push(".kari");
    fs::create_dir_all(&path).expect("Unable to create .kari directory");
    path
}

pub fn save_blockchain() {
    let kari_dir = get_kari_dir();
    let db_path = kari_dir.join("blockchain_db");
    let db = match DB::open_default(&db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Unable to open RocksDB: {}", e);
            return;
        }
    };

    unsafe {
        let data = bincode::serialize(addr_of!(BLOCKCHAIN).as_ref().unwrap())
            .expect("Failed to serialize blockchain");
        if let Err(e) = db.put(b"blockchain", data) {
            eprintln!("Unable to write blockchain to RocksDB: {}", e);
        }
    }

    if let Err(e) = db.flush() {
        eprintln!("Failed to flush RocksDB: {}", e);
    }
    drop(db); // Ensure the database is properly closed
}

pub fn load_blockchain() {
    let kari_dir = get_kari_dir();
    let db_path = kari_dir.join("blockchain_db");
    let db = match DB::open_default(&db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Unable to open RocksDB: {}", e);
            return;
        }
    };

    unsafe {
        match db.get(b"blockchain") {
            Ok(Some(value)) => {
                BLOCKCHAIN = bincode::deserialize(&value).expect("Failed to deserialize blockchain");
            }
            Ok(None) => {
                println!("No blockchain data found in RocksDB");
            }
            Err(e) => {
                println!("Error reading from RocksDB: {}", e);
            }
        }

        // After loading the blockchain, update BALANCES and TOTAL_TOKENS
        let mut balances = HashMap::new();
        let mut total_tokens = 0;

        // Process blocks
        for block in BLOCKCHAIN.iter() {
            total_tokens += block.tokens;
            *balances.entry(block.address.clone()).or_insert(0) += block.tokens;

            // Process transactions
            for tx in &block.transactions {
                *balances.entry(tx.sender.clone()).or_insert(0) -= tx.amount;
                *balances.entry(tx.receiver.clone()).or_insert(0) += tx.amount;
            }

            if !block.data.is_empty() {
                if let Some(modules_mutex) = MOVE_MODULES.as_ref() {
                    if let Ok(mut modules) = modules_mutex.lock() {
                        modules.insert(block.hash.clone(), block.data.clone());
                    }
                } else {
                    // Handle the case where MOVE_MODULES is None
                    eprintln!("MOVE_MODULES is not initialized");
                }
            }
        }

        BALANCES = Some(Mutex::new(balances));
        TOTAL_TOKENS = total_tokens;

        println!("Blockchain loaded successfully from RocksDB");
    }

    db.flush().expect("Failed to flush RocksDB");
    drop(db); // Ensure the database is properly closed
}