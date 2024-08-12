// blockchain_sim// blockchain_simulation.rs
use std::sync::{Arc, Mutex};
use std::thread;
use consensus_pos::{Blake3Algorithm, HashAlgorithm};
use simulation::block::Block;
use simulation::blockchain::{save_blockchain, BALANCES, BLOCKCHAIN, TOTAL_TOKENS};
use simulation::gas::TRANSACTION_GAS_COST;
use simulation::transaction::Transaction;
use std::sync::mpsc::{self, Sender, Receiver};

pub static mut TRANSACTION_SENDER: Option<Sender<Transaction>> = None;
pub static mut TRANSACTION_RECEIVER: Option<Receiver<Transaction>> = None;

pub fn run_blockchain(running: Arc<Mutex<bool>>, miner_address: String) {
    let max_tokens = 11_000_000;
    let mut tokens_per_block = 25;
    let halving_interval = 210_000;
    let block_size = 2_250_000;

    static mut PENDING_TRANSACTIONS: Vec<Transaction> = Vec::new();

    unsafe {
        let (sender, receiver) = mpsc::channel();
        TRANSACTION_SENDER = Some(sender);
        TRANSACTION_RECEIVER = Some(receiver);        

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

        loop { 
            if let Ok(transaction) = TRANSACTION_RECEIVER.as_ref().unwrap().try_recv() {
                // Before adding the transaction to the pending list, verify it
                if transaction.is_valid() {
                    PENDING_TRANSACTIONS.push(transaction.clone());
                    println!("Transaction added to pending list: {:?}", transaction); // Debugging line
                } else {
                    println!("Invalid transaction received and discarded."); // Debugging line
                }
            }

            

            let _running = running.lock().unwrap();

            if TOTAL_TOKENS >= max_tokens {
                println!("Reached maximum token supply. Only processing transactions.");
                tokens_per_block = 0;
            }

            let miner_reward = if TOTAL_TOKENS < max_tokens {
                tokens_per_block
            } else {
                0 
            };

            let prev_block = BLOCKCHAIN.back().unwrap();
            let new_data = vec![0; block_size];

            PENDING_TRANSACTIONS.clear(); 

            let mut transactions = vec![];

            transactions.append(&mut (PENDING_TRANSACTIONS.clone()));
            PENDING_TRANSACTIONS.clear(); 

            if transactions.is_empty() {
                transactions.push(Transaction {
                    sender: String::from("system"),
                    receiver: miner_address.clone(),
                    amount: 0,
                    gas_cost: TRANSACTION_GAS_COST,
                });
            }

            let hasher = Blake3Algorithm;
            let new_block = Block::new(
                prev_block.index + 1, 
                new_data, 
                prev_block.hash.clone(), 
                miner_reward, 
                transactions.clone(), 
                miner_address.clone(), 
                hasher
            );

            if !new_block.verify(prev_block) {
                println!("Block verification failed!");
                break;
            }

            BLOCKCHAIN.push_back(new_block.clone());

            let transaction_fees: u64 = new_block.transactions.iter().map(|tx| tx.calculate_total_cost() as u64).sum();
            BALANCES.as_mut().unwrap().lock().unwrap().entry(miner_address.clone()).and_modify(|balance| *balance += transaction_fees + miner_reward).or_insert(transaction_fees + miner_reward);

            if TOTAL_TOKENS < max_tokens {
                TOTAL_TOKENS += miner_reward;
            }

            save_blockchain();

            println!("New block hash: {}", new_block.hash);
            print!("tx: ");
            for transaction in &transactions {
                println!("  - Sender: {}", transaction.sender);
                println!("    Receiver: {}", transaction.receiver);
                println!("    Amount: {}", transaction.amount);
                println!("    Fee: {:.8} KI", transaction.gas_cost); // Format fee to 8 decimal places
                println!("    Transaction Hash: {}", Blake3Algorithm.hash(serde_json::to_string(transaction).unwrap().as_bytes())); // Calculate and print a unique hash for each transaction
            }

            println!("Miner reward (transaction fees): {} tokens", transaction_fees);

            if BLOCKCHAIN.len() % halving_interval == 0 && TOTAL_TOKENS < max_tokens {
                tokens_per_block /= 2;
            }
            
            println!("blocks: {}, Total tokens: {}", BLOCKCHAIN.len(), TOTAL_TOKENS);
            thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}