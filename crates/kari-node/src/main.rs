use k2::transaction::{Transaction, TransactionType};
use kari_node::Shard;



#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a new shard instance
    let shard = Shard::new();

    // Create some example transactions
    let transactions = vec![
        Transaction {sender:"user1".to_string(),tx_type:TransactionType::MoveModuleDeploy(vec![1,2,3]), receiver: todo!(), amount: todo!(), gas_cost: todo!(), timestamp: todo!(), signature: todo!(), data: todo!(), coin_type: todo!() },
        // Add more test transactions as needed
    ];

    // Process the transactions
    shard.process_shard_transactions(transactions).await;

    println!("Transactions processed successfully");
    Ok(())
}