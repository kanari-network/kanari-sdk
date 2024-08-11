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

pub static mut BALANCES: Option<Mutex<HashMap<String, u64>>> = None;
pub static mut TOTAL_TOKENS: u64 = 0;
pub static mut BLOCKCHAIN: VecDeque<Block<Blake3Algorithm>> = VecDeque::new();

pub fn get_kari_dir() -> PathBuf {
    let mut path = dirs::home_dir().expect("Unable to find home directory");
    path.push(".kari");
    fs::create_dir_all(&path).expect("Unable to create .kari directory");
    path
}

pub fn save_blockchain() {
    let kari_dir = get_kari_dir();
    let blockchain_file = kari_dir.join("blockchain.bin");
    unsafe {
        let data = bincode::serialize(addr_of!(BLOCKCHAIN).as_ref().unwrap()).expect("Failed to serialize blockchain");
        fs::write(&blockchain_file, data).expect("Unable to write blockchain to file");
    }
    println!("Blockchain saved to {:?}", blockchain_file);
}

pub fn load_blockchain() {
    let kari_dir = get_kari_dir();
    let blockchain_file = kari_dir.join("blockchain.bin");
    if blockchain_file.exists() {
        unsafe {
            let data = fs::read(&blockchain_file).expect("Unable to read blockchain file");
            BLOCKCHAIN = bincode::deserialize(&data).expect("Failed to deserialize blockchain");

            let mut balances = HashMap::new();
            let mut total_tokens = 0;
            for block in BLOCKCHAIN.iter() {
                total_tokens += block.tokens;
                *balances.entry(block.miner_address.clone()).or_insert(0) += block.tokens;
                for tx in &block.transactions {
                    if let Some(sender_balance) = balances.get_mut(&tx.sender) {
                        if *sender_balance >= tx.amount {
                            *sender_balance -= tx.amount;
                            *balances.entry(tx.receiver.clone()).or_insert(0) += tx.amount;
                        } else {
                            eprintln!("Transaction failed: Insufficient funds for sender {}", tx.sender);
                        }
                    } else {
                        eprintln!("Transaction failed: Sender {} not found", tx.sender);
                    }
                }                
            }
            BALANCES = Some(Mutex::new(balances));
            TOTAL_TOKENS = total_tokens;
        }
        println!("Blockchain loaded from {:?}", blockchain_file);
    }
}