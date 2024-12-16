// blockchain.rs
use crate::block::Block;
use bincode;
use consensus_pos::Blake3Algorithm;
use dirs;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::ptr::addr_of;
use std::sync::Mutex;

// Define global variables for the blockchain
pub static mut BALANCES: Option<Mutex<HashMap<String, u64>>> = None;
// Define global variables for the blockchain
pub static mut TOTAL_TOKENS: u64 = 0;
// Define global variables for the blockchain
pub static mut BLOCKCHAIN: VecDeque<Block<Blake3Algorithm>> = VecDeque::new();

// Add Move module storage
pub static mut MOVE_MODULES: Option<Mutex<HashMap<String, Vec<u8>>>> = None;

// Add Move module transaction type
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MoveModuleTransaction {
    pub sender: String,
    pub module_bytes: Vec<u8>,
    pub gas_cost: f64,
    pub timestamp: u64,
}

pub fn get_kari_dir() -> PathBuf {
    let mut path = dirs::home_dir().expect("Unable to find home directory");
    path.push(".kari");
    fs::create_dir_all(&path).expect("Unable to create .kari directory");
    path
}

pub fn save_blockchain() {
    let kari_dir = get_kari_dir();
    let blockchain_file = kari_dir.join("data.log");
    unsafe {
        let data = bincode::serialize(addr_of!(BLOCKCHAIN).as_ref().unwrap())
            .expect("Failed to serialize blockchain");
        fs::write(&blockchain_file, data).expect("Unable to write blockchain to file");
    }
    println!("Blockchain saved to {:?}", blockchain_file);
}

pub fn load_blockchain() {
    let kari_dir = get_kari_dir();
    let blockchain_file = kari_dir.join("data.log");

    if blockchain_file.exists() {
        unsafe {
            // Use BufReader for faster reading
            let file = fs::File::open(&blockchain_file).expect("Cannot open blockchain file");
            let reader = BufReader::new(file);

            // Deserialize with better error handling
            BLOCKCHAIN = match bincode::deserialize_from(reader) {
                Ok(blockchain) => blockchain,
                Err(e) => {
                    println!("Error deserializing blockchain: {}", e);
                    return;
                }
            };

            // After loading the blockchain, update BALANCES and TOTAL_TOKENS
            let mut balances = HashMap::new();
            let mut total_tokens = 0;

            if MOVE_MODULES.is_none() {
                MOVE_MODULES = Some(Mutex::new(HashMap::new()));
            }

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
                    if let Ok(mut modules) = MOVE_MODULES.as_ref().unwrap().lock() {
                        modules.insert(block.hash.clone(), block.data.clone());
                    }
                }
            }

            BALANCES = Some(Mutex::new(balances));
            TOTAL_TOKENS = total_tokens;

            println!("Blockchain loaded successfully from {:?}", blockchain_file);
        }
    }
}
