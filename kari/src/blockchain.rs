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

// Get the path to the .kari directory
pub fn get_kari_dir() -> PathBuf {
    let mut path = dirs::home_dir().expect("Unable to find home directory");
    path.push(".kari");
    fs::create_dir_all(&path).expect("Unable to create .kari directory");
    path
}

// Save the blockchain to the file system
pub fn save_blockchain() {
    let kari_dir = get_kari_dir();
    let blockchain_file = kari_dir.join("blockchain.bin");
    unsafe {
        let data = bincode::serialize(addr_of!(BLOCKCHAIN).as_ref().unwrap()).expect("Failed to serialize blockchain");
        fs::write(&blockchain_file, data).expect("Unable to write blockchain to file");
    }
    println!("Blockchain saved to {:?}", blockchain_file);
}

// Load the blockchain from the file system
pub fn load_blockchain() {
    let kari_dir = get_kari_dir();
    let blockchain_file = kari_dir.join("blockchain.bin");
    if blockchain_file.exists() {
        unsafe {
            let data = fs::read(&blockchain_file).expect("Unable to read blockchain file");
            BLOCKCHAIN = bincode::deserialize(&data).expect("Failed to deserialize blockchain");
        }
        println!("Blockchain loaded from {:?}", blockchain_file);
    }
}