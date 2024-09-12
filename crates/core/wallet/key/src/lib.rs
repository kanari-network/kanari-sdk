
use std::fs;
use serde_json::json;
use bip39::Mnemonic;
use secp256k1::Secp256k1;
use rand::rngs::OsRng;
use hex;
use simulation::{blockchain::{get_kari_dir, BALANCES}, gas::TRANSACTION_GAS_COST, transaction::Transaction};


pub fn send_coins(sender: String, receiver: String, amount: u64) -> Option<Transaction> {
    // Access BALANCES safely
    let balances_option = unsafe { BALANCES.as_ref() };

    // Check if BALANCES is initialized
    if let Some(balances_mutex) = balances_option {
        // Lock the mutex to access balances
        let mut balances = balances_mutex.lock().unwrap(); 

        // Check if sender's balance exists
        if let Some(sender_balance) = balances.get_mut(&sender) {
            // Check if sender has enough balance
            if *sender_balance >= amount {
                // Deduct amount from sender
                *sender_balance -= amount;
                // Add amount to receiver
                *balances.entry(receiver.clone()).or_insert(0) += amount;

                // Create and return the transaction
                let transaction = Transaction {
                    sender,
                    receiver,
                    amount,
                    gas_cost: TRANSACTION_GAS_COST, 
                };

                return Some(transaction);
            } else {
                // Insufficient funds
                println!("Insufficient funds in sender's account.");
            }
        } else {
            // Sender's address not found
            println!("Sender's address not found in balances.");
        }
    } else {
        // BALANCES is not initialized
        println!("BALANCES is not initialized!");
    }

    // Return None if any check fails
    None 
}

pub fn save_wallet(address: &str, private_key: &str, seed_phrase: &str) {
    let kari_dir = get_kari_dir();
    let wallet_dir = kari_dir.join("wallets");
    fs::create_dir_all(&wallet_dir).expect("Unable to create wallets directory");

    let wallet_file = wallet_dir.join(format!("{}.json", address));
    let wallet_data = json!({
        "address": address,
        "private_key": private_key,
        "seed_phrase": seed_phrase
    });

    fs::write(&wallet_file, serde_json::to_string_pretty(&wallet_data).unwrap())
        .expect("Unable to write wallet to file");
    println!("Wallet saved to {:?}", wallet_file);
}

pub fn load_wallet(address: &str) -> Option<serde_json::Value> {
    let kari_dir = get_kari_dir();
    let wallet_file = kari_dir.join("wallets").join(format!("{}.json", address));

    if wallet_file.exists() {
        let data = fs::read_to_string(wallet_file).expect("Unable to read wallet file");
        Some(serde_json::from_str(&data).expect("Unable to parse wallet data"))
    } else {
        None
    }
}

pub fn generate_karix_address(word_count: usize) -> (String, String, String) {
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

    // Serialize and encode the public key
    let mut hex_encoded = hex::encode(&public_key.serialize_uncompressed()[1..]);
    hex_encoded.truncate(64); // Adjust as needed

    let karix_public_address = format!("0x{}", hex_encoded);

    // Generate mnemonic with specified word count
    let mnemonic_result = match word_count {
        12 => Mnemonic::generate(12),
        24 => Mnemonic::generate(24),
        _ => panic!("Unsupported word count: {}", word_count),
    };

    let mnemonic = match mnemonic_result {
        Ok(m) => m,
        Err(e) => panic!("Failed to generate mnemonic: {:?}", e),
    };
    let seed_phrase = mnemonic.to_string(); // Directly convert Mnemonic to String

    (
        secret_key.display_secret().to_string(),
        karix_public_address,
        seed_phrase
    )
}

pub fn list_wallet_files() -> Result<Vec<String>, std::io::Error> {
    let kari_dir = get_kari_dir();
    let wallet_dir = kari_dir.join("wallets");

    let mut wallets = Vec::new();
    for entry in fs::read_dir(wallet_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                wallets.push(filename.to_string());
            }
        }
    }
    Ok(wallets)
}

