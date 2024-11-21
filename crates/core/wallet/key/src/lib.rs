use std::{collections::HashMap, fs, path::PathBuf, str::FromStr as _, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};
use serde_json::json;
use bip39::Mnemonic;

use secp256k1::{Secp256k1, Message, SecretKey};
use rand::rngs::OsRng;
use hex;


// Import Mutex and HashMap from std::sync
use k2::{blockchain::{get_kari_dir,  BALANCES}, gas::TRANSACTION_GAS_COST, transaction::Transaction};
use k2::simulation::TRANSACTION_SENDER;

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

/// Loads a wallet from the filesystem given an address
/// 
/// # Arguments
/// * `address` - The wallet address string
/// 
/// # Returns
/// * `Option<serde_json::Value>` - The wallet data if found, None otherwise 
/// 
/// # Errors
/// Returns None if:
/// - Address is empty
/// - File cannot be read
/// - JSON parsing fails
pub fn load_wallet(address: &str) -> Option<serde_json::Value> {
    // Validate input
    if address.trim().is_empty() {
        return None;
    }

    let kari_dir = get_kari_dir();
    let wallet_file: PathBuf = kari_dir.join("wallets").join(format!("{}.json", address));

    if wallet_file.exists() {
        // Handle potential IO and parsing errors gracefully
        fs::read_to_string(&wallet_file)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
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

/// Returns list of wallet files with selection status
/// 
/// # Returns
/// * `Result<Vec<(String, bool)>>` - List of (wallet_filename, is_selected) tuples
pub fn list_wallet_files() -> Result<Vec<(String, bool)>, std::io::Error> {
    let kari_dir = get_kari_dir();
    let wallet_dir = kari_dir.join("wallets");
    
    // Get currently selected wallet
    let selected = get_selected_wallet().unwrap_or_default();

    let mut wallets = Vec::new();
    for entry in fs::read_dir(wallet_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                // Check if this wallet is selected
                let is_selected = filename.trim_end_matches(".json") == selected;
                wallets.push((filename.to_string(), is_selected));
            }
        }
    }
    Ok(wallets)
}

/// Read currently selected wallet from config
fn get_selected_wallet() -> Option<String> {
    let config_path = get_kari_dir().join("config.json");
    fs::read_to_string(config_path)
        .ok()
        .and_then(|data| serde_json::from_str::<serde_json::Value>(&data).ok())
        .and_then(|json| json.get("selected_wallet")?.as_str().map(String::from))
}

// Initialize global state
pub fn initialize_globals() {
    unsafe {
        if BALANCES.is_none() {
            BALANCES = Some(Mutex::new(HashMap::new()));
        }
    }
}

pub fn send_coins(from_address: &str, to_address: &str, amount: u64) -> Result<String, String> {
    // Load sender's wallet
    let sender_wallet = match load_wallet(from_address) {
        Some(wallet) => wallet,
        None => return Err("Sender wallet not found".to_string())
    };

    initialize_globals();

    // รับค่า TRANSACTION_SENDER
    let transaction_sender = unsafe {
        TRANSACTION_SENDER.as_ref().ok_or("Transaction sender not initialized")?
    };

    // Get sender's private key
    let private_key = sender_wallet["private_key"].as_str()
        .ok_or("Invalid wallet format")?;
    
    // Properly unwrap BALANCES Option<Mutex>
    let mut balances = unsafe {
        BALANCES.as_ref()
            .ok_or("BALANCES not initialized")?
            .lock()
            .map_err(|_| "Failed to lock BALANCES mutex")?
    };

    // Ensure sender's address is in the balances map
    if !balances.contains_key(from_address) {
        return Err("Transaction sender not initialized".to_string());
    }

    let sender_balance = balances.get(from_address).unwrap_or(&0);

    if *sender_balance < amount + TRANSACTION_GAS_COST as u64 {
        return Err("Insufficient balance".to_string());
    }

    // Proceed with the transaction
    let secp = Secp256k1::new();
    let mut transaction = Transaction::new(
        from_address.to_string(),
        to_address.to_string(),
        amount
    );

    // Sign the transaction using updated Message creation
    let message = Message::from_digest_slice(&transaction.hash()).expect("32 bytes");
    let secret_key = SecretKey::from_str(private_key).expect("Valid private key");
    let signature = secp.sign_ecdsa(&message, &secret_key);
    
    // Convert signature to hex string
    let sig_hex = hex::encode(signature.serialize_compact());
    transaction.signature = Some(sig_hex);

    // ตั้งค่า timestamp
    transaction.timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Update balances
    *balances.get_mut(from_address).unwrap() -= amount + TRANSACTION_GAS_COST as u64;
    *balances.entry(to_address.to_string()).or_insert(0) += amount;

    // ส่งธุรกรรมไปยัง TRANSACTION_SENDER
    transaction_sender.send(transaction).map_err(|e| e.to_string())?;

    Ok("Transaction sent to the node successfully".to_string())
}