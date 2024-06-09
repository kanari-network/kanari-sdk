use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use digest::Digest;
use serde::{Deserialize, Serialize};
use hex;
use serde_json::{json, Value as JsonValue};
use sha2::Sha256;

#[derive(Serialize, Deserialize, Clone)]
struct Transaction {
    sender: String,
    receiver: String,
    amount: u64,
}

#[derive(Serialize, Deserialize, Clone)]
struct Block {
    index: u32,
    timestamp: u64,
    data: JsonValue, // Changed from String to JsonValue
    hash: String,
    prev_hash: String,
    tokens: u64,
    token_name: String,
    transactions: Vec<Transaction>,
}

impl Block {
    fn new(index: u32, data: JsonValue, prev_hash: String, tokens: u64, transactions: Vec<Transaction>) -> Block {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}{}{}{}", index, timestamp, data, prev_hash, tokens).as_bytes());

        let hash = hex::encode(hasher.finalize());

        Block {
            index,
            timestamp,
            data,
            hash,
            prev_hash,
            tokens,
            token_name: String::from("Kanari"),
            transactions,
        }
    }

    fn verify(&self, prev_block: &Block) -> bool {
        if self.index != prev_block.index + 1 {
            return false;
        }
        if self.prev_hash != prev_block.hash {
            return false;
        }

        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}{}{}{}", self.index, self.timestamp, self.data, self.prev_hash, self.tokens).as_bytes());

        let hash = hex::encode(hasher.finalize());

        if self.hash != hash {
            return false;
        }
        true
    }
}

static mut TOTAL_TOKENS: u64 = 0;

static mut BLOCKCHAIN: Vec<Block> = vec![];



fn run_blockchain(running: Arc<Mutex<bool>>) {
    let max_tokens = 11_000_000;
    let mut tokens_per_block = 50;
    let halving_interval = 210_000; // Halving every 210,000 blocks

    unsafe {
        if BLOCKCHAIN.is_empty() {
            let genesis_data: JsonValue = json!({
                "type": "genesis",
                "message": "Genesis Block"
            });

            let genesis_transactions = vec![]; // No transactions in the genesis block
            BLOCKCHAIN.push(Block::new(0, genesis_data, String::from("0"), tokens_per_block, genesis_transactions));
            TOTAL_TOKENS += tokens_per_block;
        }

        while TOTAL_TOKENS < max_tokens {
            if !*running.lock().unwrap() {
                break;
            }

            let prev_block = BLOCKCHAIN.last().unwrap();
            let new_data = json!({
                "type": "normal",
                "message": "New Data"
            });

            let transactions = vec![
                Transaction { sender: String::from("Alice"), receiver: String::from("Bob"), amount: 10 },
                Transaction { sender: String::from("Charlie"), receiver: String::from("Dave"), amount: 20 }, // New transaction
                // Add more transactions as needed
            ];

            let new_block = Block::new(prev_block.index + 1, new_data, prev_block.hash.clone(), tokens_per_block, transactions);
            if !new_block.verify(prev_block) {
                println!("Block verification failed!");
                break;
            }

            BLOCKCHAIN.push(new_block.clone()); // Clone the block to avoid borrowing issues
            TOTAL_TOKENS += tokens_per_block;

            // Print the hash of the new block
            println!("New block hash: {}", new_block.hash);

            if BLOCKCHAIN.len() % halving_interval == 0 {
                tokens_per_block /= 2;
            }

            println!("blocks: {}, Total tokens: {}", BLOCKCHAIN.len(), TOTAL_TOKENS);
            thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}



fn main() {
    let running = Arc::new(Mutex::new(false));
    let mut input = String::new();


    loop {

        print!("Enter command:\n start\n stop\n transfer\n");
        io::stdout().flush().unwrap(); // Flush stdout to display the prompt before waiting for input
        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "start" => {
                *running.lock().unwrap() = true;
                println!("Starting blockchain...");
                let running_clone = Arc::clone(&running);
                thread::spawn(move || {
                    run_blockchain(running_clone);
                });
            }
            "stop" => {
                *running.lock().unwrap() = false;
                println!("Stopping blockchain...");
            }
            _ => println!("Invalid command"),
        }


        input.clear(); // Clear the input string for the next iteration
    }
}