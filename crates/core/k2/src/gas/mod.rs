// kari/src/gas.rs
use std::sync::mpsc::{ Sender, Receiver}; // Import Sender and Receiver
use crate::transaction::Transaction;


pub const TRANSACTION_GAS_COST: f64 = 0.00000150; // 0.00000150 KI
pub const DEPLOY_SM_TRANSACTION_GAS_COST: f64 = 0.00005000; // 0.00005000 KI
pub const MINT_NFT_TRANSACTION_GAS_COST: f64 = 0.00010000; // 0.00010000 KI
pub const TRANSFER_NFT_TRANSACTION_GAS_COST: f64 = 0.00000100; // 0.00000100 KI
pub const PAYMENT_TRANSACTION_GAS_COST: f64 = 0.00020000; // 0.00020000 KI

// Define the Sender and Receiver separately
pub static mut TRANSACTION_SENDER: Option<Sender<Transaction>> = None;
pub static mut TRANSACTION_RECEIVER: Option<Receiver<Transaction>> = None;