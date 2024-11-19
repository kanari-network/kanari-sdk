// kari/src/gas.rs
// kari/src/gas.rs
use std::sync::mpsc::Sender;
use crate::transaction::Transaction;


pub const TRANSACTION_GAS_COST: f64 = 0.00000150; // 0.00000150 KI
pub const DEPLOY_SM_TRANSACTION_GAS_COST: f64 = 0.00005000; // 0.00005000 KI
pub static mut TRANSACTION_SENDER: Option<Sender<Transaction>> = None; // Change to Option<Sender<Transaction>>
pub const MINT_NFT_TRANSACTION_GAS_COST: f64 = 0.00010000; // 0.00010000 KI
pub const TRANSFER_NFT_TRANSACTION_GAS_COST: f64 = 0.00000100; // 0.00000100 KI
pub const PAYMENT_TRANSACTION_GAS_COST: f64 = 0.00020000; // 0.00020000 KI