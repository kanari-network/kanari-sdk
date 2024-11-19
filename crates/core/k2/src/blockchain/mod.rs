// blockchain.rs
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::ptr::addr_of;
use std::fs;
use std::path::PathBuf;
use dirs;
use bincode;
use consensus_pos::Blake3Algorithm;
use crate::block::Block;

// Define global variables for the blockchain
pub static mut BALANCES: Option<Mutex<HashMap<String, u64>>> = None;
// Define global variables for the blockchain
pub static mut TOTAL_TOKENS: u64 = 0;
// Define global variables for the blockchain
pub static mut BLOCKCHAIN: VecDeque<Block<Blake3Algorithm>> = VecDeque::new();

use std::io::BufReader;

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
        fs::write(&blockchain_file, data)
            .expect("Unable to write blockchain to file");
    }
    println!("Blockchain saved to {:?}", blockchain_file);
}

pub fn load_blockchain() {
    let kari_dir = get_kari_dir();
    let blockchain_file = kari_dir.join("data.log");
    
    if blockchain_file.exists() {
        unsafe {
            // Use BufReader for faster reading
            let file = fs::File::open(&blockchain_file)
                .expect("Cannot open blockchain file");
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

            // Process blocks
            for block in BLOCKCHAIN.iter() {
                total_tokens += block.tokens;
                *balances.entry(block.miner_address.clone()).or_insert(0) += block.tokens;
                
                // Process transactions
                for tx in &block.transactions {
                    *balances.entry(tx.sender.clone()).or_insert(0) -= tx.amount;
                    *balances.entry(tx.receiver.clone()).or_insert(0) += tx.amount;
                }                
            }

            BALANCES = Some(Mutex::new(balances));
            TOTAL_TOKENS = total_tokens;
            
            println!("Blockchain loaded successfully from {:?}", blockchain_file);
        }
    }
}