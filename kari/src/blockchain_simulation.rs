use std::sync::{Arc, Mutex};
use std::thread;
use crate::block::Block;
use crate::gas::TRANSACTION_GAS_COST;
use crate::transaction::Transaction;
use crate::blockchain::{BALANCES, BLOCKCHAIN, TOTAL_TOKENS, save_blockchain};
use consensus_pos::Blake3Algorithm;

pub fn run_blockchain(running: Arc<Mutex<bool>>, miner_address: String) {
    let max_tokens = 11_000_000;
    let mut tokens_per_block = 250000;
    let halving_interval = 210_000;
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
        }

        loop { 
            let _running = running.lock().unwrap();

            if TOTAL_TOKENS >= max_tokens {
                println!("Reached maximum token supply. Only processing transactions.");
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

            // Move the clearing of pending transactions inside the loop
            PENDING_TRANSACTIONS.clear(); 

            let mut transactions = vec![];

            // ดึงธุรกรรมจาก PENDING_TRANSACTIONS
            transactions.append(&mut (PENDING_TRANSACTIONS.clone()));

            // ถ้าไม่มีธุรกรรมเลย ให้สร้างธุรกรรมค่าธรรมเนียม 0 ให้นักขุด
            if transactions.is_empty() {
                transactions.push(Transaction {
                    sender: "system".to_string(), // หรือใช้ address พิเศษอื่นๆ 
                    receiver: miner_address.clone(),
                    amount: 0,
                    gas_cost: TRANSACTION_GAS_COST, // ให้ค่าธรรมเนียม 0.00000150 KI
                });
            }
            
            // สร้างธุรกรรมจำลองเพิ่มเติม (ถ้าต้องการ)
            transactions.push(Transaction::new(String::from("Alice"), String::from("Bob"), 10));
            transactions.push(Transaction::new(String::from("Charlie"), String::from("Dave"), 20));

            let hasher = Blake3Algorithm;
            let new_block = Block::new(
                prev_block.index + 1, 
                new_data, 
                prev_block.hash.clone(), 
                miner_reward, // Use calculated miner_reward
                transactions, 
                miner_address.clone(), 
                hasher
            );

            if !new_block.verify(prev_block) {
                println!("Block verification failed!");
                break;
            }

            BLOCKCHAIN.push_back(new_block.clone());

            // เพิ่มรางวัลให้กับนักขุดจากค่าธรรมเนียมธุรกรรม
            let transaction_fees: u64 = new_block.transactions.iter().map(|tx| tx.calculate_total_cost() as u64).sum();
            BALANCES.as_mut().unwrap().lock().unwrap().entry(miner_address.clone()).and_modify(|balance| *balance += transaction_fees + miner_reward).or_insert(transaction_fees + miner_reward);

            // Update TOTAL_TOKENS only if it's less than the max supply
            if TOTAL_TOKENS < max_tokens {
                TOTAL_TOKENS += tokens_per_block; 
            }

            // Save blockchain every time a new block is created
            save_blockchain();

            println!("New block hash: {}", new_block.hash);
            println!("Miner reward (transaction fees): {} tokens", transaction_fees);

            if BLOCKCHAIN.len() % halving_interval == 0 && TOTAL_TOKENS < max_tokens {
                tokens_per_block /= 2;
            }

            println!("blocks: {}, Total tokens: {}", BLOCKCHAIN.len(), TOTAL_TOKENS);
            thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
