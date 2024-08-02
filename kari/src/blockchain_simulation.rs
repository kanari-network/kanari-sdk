// x/src/blockchain_simulation.rs
use std::sync::{Arc, Mutex};
use std::thread;
use crate::block::Block;
use crate::transaction::Transaction;
use crate::blockchain::{BALANCES, BLOCKCHAIN, TOTAL_TOKENS, save_blockchain};
use consensus_pos::Blake3Algorithm;

pub fn run_blockchain(running: Arc<Mutex<bool>>, miner_address: String) {
    let max_tokens = 11_000_000;
    let mut tokens_per_block = 25;
    let halving_interval = 210_000;
    let block_size = 2_250_000; // 2.25 MB in bytes

    // Assume there's a global variable for pending transactions
    static mut PENDING_TRANSACTIONS: Vec<Transaction> = Vec::new();

    // Clear the pending transactions after copying them
    unsafe { PENDING_TRANSACTIONS.clear(); }

    // Include `transactions` in the call to Block::new

    unsafe {
        if BLOCKCHAIN.is_empty() {
            let genesis_data = vec![0; block_size];
            let genesis_transactions = vec![];
            let hasher = Blake3Algorithm;
            BLOCKCHAIN.push_back(Block::new(
                0,
                genesis_data,
                String::from("0"),
                tokens_per_block,
                genesis_transactions,
                miner_address.clone(),
                hasher
            ));
            TOTAL_TOKENS += tokens_per_block;
            BALANCES.as_mut().unwrap().lock().unwrap().entry(miner_address.clone()).and_modify(|balance| *balance += tokens_per_block).or_insert(tokens_per_block);
        }

        while TOTAL_TOKENS < max_tokens {
            if !*running.lock().unwrap() {
                break;
            }

            let prev_block = BLOCKCHAIN.back().unwrap();
            let new_data = vec![0; block_size];

            let transactions = vec![
                Transaction { sender: String::from("Alice"), receiver: String::from("Bob"), amount: 10 },
                Transaction { sender: String::from("Charlie"), receiver: String::from("Dave"), amount: 20 },
            ];

            let hasher = Blake3Algorithm;
            let new_block = Block::new(prev_block.index + 1, new_data, prev_block.hash.clone(), tokens_per_block, transactions, miner_address.clone(), hasher);
            if !new_block.verify(prev_block) {
                println!("Block verification failed!");
                break;
            }

            BLOCKCHAIN.push_back(new_block.clone());
            TOTAL_TOKENS += tokens_per_block;
            BALANCES.as_mut().unwrap().lock().unwrap().entry(miner_address.clone()).and_modify(|balance| *balance += tokens_per_block).or_insert(tokens_per_block);

            // Save blockchain every time a new block is created
            save_blockchain();

            println!("New block hash: {}", new_block.hash);
            println!("Miner reward: {} tokens", tokens_per_block);

            if BLOCKCHAIN.len() % halving_interval == 0 {
                tokens_per_block /= 2;
            }

            println!("blocks: {}, Total tokens: {}", BLOCKCHAIN.len(), TOTAL_TOKENS);
            thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}