// blockchain_sim// blockchain_simulation.rs
use std::sync::{Arc, Mutex};
use std::thread;
use consensus_pos::Blake3Algorithm;
use key::PENDING_TRANSACTIONS;
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
                    PENDING_TRANSACTIONS.lock().unwrap().push(transaction.clone()); 
                    println!("Transaction added to pending list: {:?}", transaction); // Debugging line

                    // Update sender and receiver balances
                    let mut balances = BALANCES.as_mut().unwrap().lock().unwrap();
                    let sender_balance = balances.entry(transaction.sender.clone()).or_insert(0);

                    // Check for underflow when subtracting amount
                    if let Some(new_balance) = sender_balance.checked_sub(transaction.amount) {
                        *sender_balance = new_balance;
                    } else {
                        println!("Error: Insufficient funds for sender: {}", transaction.sender);
                        continue; // Skip to the next iteration of the loop
                    }

                    // Check for underflow when subtracting gas cost
                    if let Some(new_balance) = sender_balance.checked_sub(transaction.calculate_total_cost() as u64) {
                        *sender_balance = new_balance;
                    } else {
                        println!("Error: Insufficient funds for gas cost: {}", transaction.sender);
                        continue;
                    }

                    let receiver_balance = balances.entry(transaction.receiver.clone()).or_insert(0);
                    *receiver_balance += transaction.amount;
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

            

            let mut transactions = vec![];

        // Correctly access and clear PENDING_TRANSACTIONS
        {
            let mut pending_txs = PENDING_TRANSACTIONS.lock().unwrap();
            transactions.append(&mut pending_txs);
            pending_txs.clear();
        }

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

            // Check if there are any user-generated transactions
            let has_user_transactions = transactions.iter().any(|tx| tx.sender != "system");

            print!("tx: ");
            if transactions.is_empty() || !has_user_transactions { 
                // No user transactions, add a fee-less system transaction for mining reward
                transactions.push(Transaction {
                    sender: String::from("system"),
                    receiver: miner_address.clone(),
                    amount: 0,
                    gas_cost: 0.0, // No fee for system transactions
                });
            } else {
                // User transactions exist, apply fees
                for transaction in &mut transactions {
                    if transaction.sender != "system" {
                        transaction.gas_cost = TRANSACTION_GAS_COST; 
                    }
                }
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
