use std::{collections::HashMap, fs, io::Write, path::PathBuf, str::FromStr as _, sync::Mutex};
use serde::{Deserialize, Serialize};
use serde_json::json;
use bip39::Mnemonic;

use secp256k1::{Secp256k1, Message, SecretKey};
use rand::rngs::OsRng;
use hex;


// Import Mutex and HashMap from std::sync
use k2::{blockchain::{get_kari_dir, save_blockchain, BALANCES}, gas::TRANSACTION_GAS_COST, transaction::Transaction};

pub fn check_wallet_exists() -> bool {
    match list_wallet_files() {
        Ok(wallets) => !wallets.is_empty(),
        Err(_) => false
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Wallet {
    pub address: String,
    pub private_key: String,
    pub seed_phrase: String,
}

pub fn save_wallet(address: &str, private_key: &str, seed_phrase: &str) {
    let kari_dir = get_kari_dir();
    let wallet_dir = kari_dir.join("wallets");
    fs::create_dir_all(&wallet_dir).expect("Unable to create wallets directory");

    let wallet_file = wallet_dir.join(format!("{}.toml", address));
    let wallet_data = Wallet {
        address: address.to_string(),
        private_key: private_key.to_string(),
        seed_phrase: seed_phrase.to_string(),
    };

    let toml_string = toml::to_string(&wallet_data).expect("Unable to serialize wallet to TOML");
    let mut file = fs::File::create(&wallet_file).expect("Unable to create wallet file");
    file.write_all(toml_string.as_bytes()).expect("Unable to write wallet to file");
    println!("Wallet saved to {:?}", wallet_file);
}


pub fn load_wallet(address: &str) -> Option<Wallet> {
    // Validate input
    if address.trim().is_empty() {
        return None;
    }

    let kari_dir = get_kari_dir();
    let wallet_file: PathBuf = kari_dir.join("wallets").join(format!("{}.toml", address));

    if wallet_file.exists() {
        // Handle potential IO and parsing errors gracefully
        fs::read_to_string(&wallet_file)
            .ok()
            .and_then(|data| toml::from_str(&data).ok())
    } else {
        None
    }
}

pub fn generate_karix_address(custom_name: &str, word_count: usize) -> (String, String, String) {
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

    // Serialize and encode the public key
    let mut hex_encoded = hex::encode(&public_key.serialize_uncompressed()[1..]);
    hex_encoded.truncate(64); // Adjust as needed

    let karix_public_address = format!("{}", custom_name);

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

    // Save the wallet to a .toml file
    let wallet_data = Wallet {
        address: karix_public_address.clone(),
        private_key: secret_key.display_secret().to_string(),
        seed_phrase: seed_phrase.clone(),
    };

    let kari_dir = get_kari_dir();
    let wallet_dir = kari_dir.join("wallets");
    fs::create_dir_all(&wallet_dir).expect("Unable to create wallets directory");

    let wallet_file = wallet_dir.join(format!("{}.toml", custom_name));
    let toml_string = toml::to_string(&wallet_data).expect("Unable to serialize wallet to TOML");
    let mut file = fs::File::create(&wallet_file).expect("Unable to create wallet file");
    file.write_all(toml_string.as_bytes()).expect("Unable to write wallet to file");
    println!("Wallet saved to {:?}", wallet_file);

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
                let is_selected = filename.trim_end_matches(".toml") == selected;
                wallets.push((filename.to_string(), is_selected));
            }
        }
    }
    Ok(wallets)
}

/// Read currently selected wallet from config
fn get_selected_wallet() -> Option<String> {
    let config_path = get_kari_dir().join("config.toml");
    fs::read_to_string(config_path)
        .ok()
        .and_then(|data| toml::from_str::<toml::Value>(&data).ok())
        .and_then(|toml| toml.get("selected_wallet")?.as_str().map(String::from))
}




// // Initialize global state
// pub fn initialize_globals() {
//     unsafe {
//         if BALANCES.is_none() {
//             BALANCES = Some(Mutex::new(HashMap::new()));
//         }
//     }
// }

// pub fn send_coins(from_address: &str, to_address: &str, amount: u64) -> Result<String, String> {
//     // Load sender's wallet
//     let sender_wallet = match load_wallet(from_address) {
//         Some(wallet) => wallet,
//         None => return Err("Sender wallet not found".to_string())
//     };

//     initialize_globals();

//     // Get sender's private key
//     let private_key = sender_wallet["private_key"].as_str()
//         .ok_or("Invalid wallet format")?;

//     // Properly unwrap BALANCES Option<Mutex>
//     let mut balances = unsafe {
//         BALANCES.as_ref()
//             .ok_or("BALANCES not initialized")?
//             .lock()
//             .map_err(|_| "Failed to lock BALANCES mutex")?
//     };

//     // Ensure sender's address is in the balances map
//     if !balances.contains_key(from_address) {
//         return Err("Transaction sender not initialized".to_string());
//     }

//     let sender_balance = balances.get(from_address).unwrap_or(&0);

// //     if *sender_balance < amount + TRANSACTION_GAS_COST as u64 {
// //         return Err("Insufficient balance".to_string());
// //     }

//     // Proceed with the transaction
//     let secp = Secp256k1::new();
//     let mut transaction = Transaction::new(
//         from_address.to_string(),
//         to_address.to_string(),
//         amount
//     );

//     // Sign the transaction using updated Message creation
//     let message = Message::from_digest_slice(&transaction.hash()).expect("32 bytes");
//     let secret_key = SecretKey::from_str(private_key).expect("Valid private key");
//     let signature = secp.sign_ecdsa(&message, &secret_key);
    
//     // Convert signature to hex string
//     let sig_hex = hex::encode(signature.serialize_compact());
//     transaction.signature = Some(sig_hex);

//     // Update balances
//     *balances.get_mut(from_address).unwrap() -= amount + TRANSACTION_GAS_COST as u64;
//     *balances.entry(to_address.to_string()).or_insert(0) += amount;

//     // After updating balances, save blockchain state
//     save_blockchain();
        
//     Ok("Transaction successful".to_string())
// }