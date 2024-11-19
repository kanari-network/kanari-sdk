use std::fs;
use serde_json::json;
use bip39::Mnemonic;
use secp256k1::Secp256k1;
use rand::rngs::OsRng;
use hex;




// Import Mutex and HashMap from std::sync
use k2::{blockchain::{get_kari_dir, BALANCES}, gas::{TRANSACTION_GAS_COST, TRANSACTION_SENDER}, transaction::Transaction};

pub fn check_wallet_exists() -> bool {
    match list_wallet_files() {
        Ok(wallets) => !wallets.is_empty(),
        Err(_) => false
    }
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


pub fn send_coins(from_address: &str, to_address: &str, amount: u64) -> Result<String, String> {
    // Load sender's wallet
    let sender_wallet = match load_wallet(from_address) {
        Some(wallet) => wallet,
        None => return Err("Sender wallet not found".to_string())
    };

    // Get sender's private key
    let private_key = sender_wallet["private_key"].as_str()
        .ok_or("Invalid wallet format")?;

    // Properly unwrap BALANCES Option<Mutex>
    let mut balances = unsafe { BALANCES.as_ref().expect("BALANCES not initialized").lock().unwrap() };
    let sender_balance = balances.get(from_address).unwrap_or(&0);
    
    if *sender_balance < amount + TRANSACTION_GAS_COST as u64 {
        return Err("Insufficient balance".to_string());
    }

    // Create and sign transaction (now with 3 params)
    let secp = Secp256k1::new();
    let mut transaction = Transaction::new(
        from_address.to_string(),
        to_address.to_string(),
        amount
    );

    // Sign transaction
    let private_key_bytes = hex::decode(private_key)
        .map_err(|_| "Invalid private key")?;
    let signature = transaction.sign(&secp, &private_key_bytes)?;

    // Update balances with proper mutex handling
    *balances.entry(from_address.to_string())
        .or_insert(0) -= amount + TRANSACTION_GAS_COST as u64;
    *balances.entry(to_address.to_string())
        .or_insert(0) += amount;

    // Send the transaction to the blockchain simulation
    unsafe {
        if let Some(sender) = TRANSACTION_SENDER.as_ref() {
            sender.send(transaction).expect("Failed to send transaction");
        } else {
            return Err("Transaction sender not initialized".to_string());
        }
    }

    Ok(signature)
}