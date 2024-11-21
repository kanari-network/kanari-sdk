use std::sync::{Arc, Mutex};
use std::thread;
use consensus_pos::Blake3Algorithm;
use std::time::{SystemTime, UNIX_EPOCH};
use log::{info, warn, error};

use crate::block::Block;
use crate::blockchain::{save_blockchain, BALANCES, BLOCKCHAIN, TOTAL_TOKENS};
use crate::gas::{TRANSACTION_GAS_COST, TRANSACTION_RECEIVER};
use crate::transaction::Transaction;



pub fn run_blockchain(running: Arc<Mutex<bool>>, miner_address: String) {
    let max_tokens = 11_000_000; // Maximum token supply   
    let mut tokens_per_block = 25; // Initial block reward
    let halving_interval = 210_000; // Halve the block reward every 210,000 blocks
    let block_size = 2_250_000; // 2.25 MB in bytes

    // Assume there's a global variable for pending transactions
    static mut PENDING_TRANSACTIONS: Vec<Transaction> = Vec::new();

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
            info!("Genesis block created with hash: {}", BLOCKCHAIN.back().unwrap().hash);
        }

        loop { 
            // Receive transactions from the channel
            if let Ok(transaction) = TRANSACTION_RECEIVER.as_ref().unwrap().try_recv() {
                info!("Received new transaction: {:?}", transaction);
                PENDING_TRANSACTIONS.push(transaction);
            }

            let _running = running.lock().unwrap();

            if TOTAL_TOKENS >= max_tokens {
                warn!("Reached maximum token supply. Only processing transactions.");
                tokens_per_block = 0; // Set block reward to 0
            }

            // Calculate miner reward based on token supply
            let miner_reward = if TOTAL_TOKENS < max_tokens {
                tokens_per_block
            } else {
                0 
            };

            let prev_block = BLOCKCHAIN.back().unwrap();
            let new_data = vec![0; block_size];

            let mut transactions = vec![];

            // Move the clearing of pending transactions after they are processed
            transactions.append(&mut PENDING_TRANSACTIONS);

            // If there are no transactions, create a zero-fee transaction for the miner
            if transactions.is_empty() {
                transactions.push(Transaction {
                    sender: "system".to_string(),
                    receiver: miner_address.clone(),
                    amount: 0,
                    gas_cost: TRANSACTION_GAS_COST,
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    signature: None, // Add an empty signature or a valid one if available
                });
                info!("No transactions found. Created a zero-fee transaction for the miner.");
            }

            let hasher = Blake3Algorithm;
            let new_block = Block::new(
                prev_block.index + 1, 
                new_data, 
                prev_block.hash.clone(), 
                miner_reward, // Use calculated miner_reward
                transactions.clone(), 
                miner_address.clone(), 
                hasher
            );

            if !new_block.verify(prev_block) {
                error!("Block verification failed!");
                break;
            }

            BLOCKCHAIN.push_back(new_block.clone());

            // Update balances for each transaction
            for tx in transactions.iter() {
                let mut balances = BALANCES.as_mut().unwrap().lock().unwrap();
                if let Some(sender_balance) = balances.get_mut(&tx.sender) {
                    *sender_balance -= tx.amount + tx.gas_cost as u64;
                }
                *balances.entry(tx.receiver.clone()).or_insert(0) += tx.amount;
            }

            // Add transaction fees to the miner's reward
            let transaction_fees: u64 = new_block.transactions.iter().map(|tx| tx.gas_cost as u64).sum();
            BALANCES.as_mut().unwrap().lock().unwrap().entry(miner_address.clone()).and_modify(|balance| *balance += transaction_fees + miner_reward).or_insert(transaction_fees + miner_reward);

            // Update TOTAL_TOKENS only if it's less than the max supply
            if TOTAL_TOKENS < max_tokens {
                TOTAL_TOKENS += tokens_per_block; 
            }

            // Save blockchain every time a new block is created
            save_blockchain();

            info!("New block created with hash: {}", new_block.hash);
            info!("Miner reward (transaction fees): {} tokens", transaction_fees);

            if BLOCKCHAIN.len() % halving_interval == 0 && TOTAL_TOKENS < max_tokens {
                tokens_per_block /= 2;
                info!("Block reward halved to: {}", tokens_per_block);
            }

            info!("Total blocks: {}, Total tokens: {}", BLOCKCHAIN.len(), TOTAL_TOKENS);
            thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}