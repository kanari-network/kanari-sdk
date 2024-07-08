use std::collections::{HashMap, VecDeque};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::{env, process, thread};
use std::time::{SystemTime, UNIX_EPOCH};
use digest::Digest;
use serde::{Deserialize, Serialize};
use std::ptr::addr_of;
use sha2::Sha256;
use jsonrpc_core::futures::FutureExt;
use jsonrpc_core::{IoHandler, Params, Result as JsonRpcResult};
use jsonrpc_http_server::{ServerBuilder, AccessControlAllowOrigin, DomainsValidation};
use serde_json::{json, Value as JsonValue};
use bip39::{Mnemonic};
use colored::Colorize;
use secp256k1::{Secp256k1};
use rand::rngs::OsRng;
use hex;
use std::fs;
use std::path::PathBuf;
use dirs;
use consensus_core::NetworkConfig;
use consensus_pow::adjust_difficulty;
use p2p_protocol::P2PNetwork;

static CHAIN_ID: &str = "kari-c1";

#[derive(Serialize, Deserialize, Clone)]
struct Transaction {
    sender: String,
    receiver: String,
    amount: u64,
}

#[derive(Serialize, Deserialize, Clone)]
struct Block {
    chain_id: String,
    index: u32,
    timestamp: u64,
    data: Vec<u8>, // เปลี่ยนเป็น Vec<u8> เพื่อเก็บข้อมูลแบบไบนารี
    hash: String,
    prev_hash: String,
    tokens: u64,
    token_name: String,
    transactions: Vec<Transaction>,
    miner_address: String,
}

static mut BALANCES: Option<Mutex<HashMap<String, u64>>> = None;


impl Block {
    fn new(index: u32, data: Vec<u8>, prev_hash: String, tokens: u64, transactions: Vec<Transaction>, miner_address: String) -> Block {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut hasher = Sha256::new();
        hasher.update(&index.to_le_bytes());
        hasher.update(&timestamp.to_le_bytes());
        hasher.update(&data);
        hasher.update(prev_hash.as_bytes());
        hasher.update(&tokens.to_le_bytes());

        let hash = hex::encode(hasher.finalize());

        Block {
            chain_id: CHAIN_ID.to_string(), // Use to_string() to create a String from &str
            index,
            timestamp,
            data,
            hash,
            prev_hash,
            tokens,
            token_name: String::from("Kanari"),
            transactions,
            miner_address,
        }
    }

    fn verify(&self, prev_block: &Block) -> bool {
        if self.index != prev_block.index + 1 {
            return false;
        }
        if self.prev_hash != prev_block.hash {
            return false;
        }

        let mut hasher = Sha256::new();
        hasher.update(&self.index.to_le_bytes());
        hasher.update(&self.timestamp.to_le_bytes());
        hasher.update(&self.data);
        hasher.update(self.prev_hash.as_bytes());
        hasher.update(&self.tokens.to_le_bytes());

        let hash = hex::encode(hasher.finalize());

        if self.hash != hash {
            return false;
        }
        true
    }
}

// สร้างตัวแปร global สำหรับเก็บจำนวน token ทั้งหมด
static mut TOTAL_TOKENS: u64 = 0;

// สร้าง blockchain แบบ global ให้ thread สามารถเข้าถึงได้
static mut BLOCKCHAIN: VecDeque<Block> = VecDeque::new();


fn get_kari_dir() -> PathBuf {
    let mut path = dirs::home_dir().expect("Unable to find home directory");
    path.push(".kari");
    fs::create_dir_all(&path).expect("Unable to create .kari directory");
    path
}

fn send_coins(sender: String, receiver: String, amount: u64) -> bool {
    let mut balances = unsafe { BALANCES.as_ref().unwrap().lock().unwrap() };

    if let Some(sender_balance) = balances.get(&sender) {
        if *sender_balance >= amount {
            // Deduct the amount from the sender's balance
            *balances.entry(sender.clone()).or_insert(0) -= amount;

            // Add the amount to the receiver's balance
            *balances.entry(receiver.clone()).or_insert(0) += amount;

            // Here, you would also add the transaction to a list of pending transactions
            // For simplicity, this step is omitted

            return true;
        }
    }
    false
}

// Blockchain simulation
fn run_blockchain(running: Arc<Mutex<bool>>, miner_address: String) {
    let max_tokens = 11_000_000;
    let mut tokens_per_block = 25;
    let halving_interval = 210_000;
    let block_size = 2_250_000; // 2.25 MB in bytes

    // Adjust difficulty based on some criteria, e.g., number of miners
    let current_miners = 10; // Example value, replace with actual logic to determine the number of miners
    let _difficulty = adjust_difficulty(current_miners);

    // Assume there's a global variable for pending transactions
    static mut PENDING_TRANSACTIONS: Vec<Transaction> = Vec::new();

    // Clear the pending transactions after copying them
    unsafe { PENDING_TRANSACTIONS.clear(); }

    // Include `transactions` in the call to Block::new

    unsafe {
        if BLOCKCHAIN.is_empty() {
            let genesis_data = vec![0; block_size];
            let genesis_transactions = vec![];
            BLOCKCHAIN.push_back(Block::new(
                0,
                genesis_data,
                String::from("0"),
                tokens_per_block,
                genesis_transactions,
                miner_address.clone()
            ));
            TOTAL_TOKENS += tokens_per_block;
            BALANCES.as_mut().unwrap().lock().unwrap().entry(miner_address.clone()).and_modify(|balance| *balance += tokens_per_block).or_insert(tokens_per_block);
        }

        while TOTAL_TOKENS < max_tokens {
            if !*running.lock().unwrap() {
                break;
            }

            let prev_block = BLOCKCHAIN.back().unwrap();
            let new_data = vec![0; block_size];

            let transactions = vec![
                Transaction { sender: String::from("Alice"), receiver: String::from("Bob"), amount: 10 },
                Transaction { sender: String::from("Charlie"), receiver: String::from("Dave"), amount: 20 },
            ];

            let new_block = Block::new(prev_block.index + 1, new_data, prev_block.hash.clone(), tokens_per_block, transactions, miner_address.clone());
            if !new_block.verify(prev_block) {
                println!("Block verification failed!");
                break;
            }

            BLOCKCHAIN.push_back(new_block.clone());
            TOTAL_TOKENS += tokens_per_block;
            BALANCES.as_mut().unwrap().lock().unwrap().entry(miner_address.clone()).and_modify(|balance| *balance += tokens_per_block).or_insert(tokens_per_block);

            // เพิ่มการบันทึก blockchain ทุกครั้งที่สร้าง block ใหม่
            save_blockchain();

            println!("New block hash: {}", new_block.hash);
            println!("Miner reward: {} tokens", tokens_per_block);


            if BLOCKCHAIN.len() % halving_interval == 0 {
                tokens_per_block /= 2;
            }

            println!("blocks: {}, Total tokens: {}", BLOCKCHAIN.len(), TOTAL_TOKENS);
            thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
// Save blockchain to file
fn save_blockchain() {
    let kari_dir = get_kari_dir();
    let blockchain_file = kari_dir.join("blockchain.bin");
    unsafe {
        let data = bincode::serialize(addr_of!(BLOCKCHAIN).as_ref().unwrap()).expect("Failed to serialize blockchain");
        fs::write(&blockchain_file, data).expect("Unable to write blockchain to file");
    }
    println!("Blockchain saved to {:?}", blockchain_file);
}

// Load blockchain from file
fn load_blockchain() {
    let kari_dir = get_kari_dir();
    let blockchain_file = kari_dir.join("blockchain.bin");
    if blockchain_file.exists() {
        unsafe {
            let data = fs::read(&blockchain_file).expect("Unable to read blockchain file");
            BLOCKCHAIN = bincode::deserialize(&data).expect("Failed to deserialize blockchain");
        }
        println!("Blockchain loaded from {:?}", blockchain_file);
    }
}

fn save_wallet(address: &str, private_key: &str, seed_phrase: &str) {
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

fn load_wallet(address: &str) -> Option<serde_json::Value> {
    let kari_dir = get_kari_dir();
    let wallet_file = kari_dir.join("wallets").join(format!("{}.json", address));

    if wallet_file.exists() {
        let data = fs::read_to_string(wallet_file).expect("Unable to read wallet file");
        Some(serde_json::from_str(&data).expect("Unable to parse wallet data"))
    } else {
        None
    }
}

// Corrected Mnemonic generation
fn generate_karix_address(word_count: usize) -> (String, String, String) {
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
fn print_coin_icon() {
    // Placeholder for the coin icon in CLI
    println!("Coin Icon: {}", "🪙"); // Using a Unicode emoji as a placeholder
}

fn save_chain_id(chain_id: &str) -> io::Result<()> {
    let home_dir = dirs::home_dir().expect("Unable to find home directory");
    let config_path = home_dir.join(".kari").join("network");
    fs::create_dir_all(&config_path)?;

    let config_file_path = config_path.join("config.json");
    let chain_id_json = json!({ "chain_id": chain_id });
    fs::write(config_file_path, serde_json::to_string_pretty(&chain_id_json)?)?;
    Ok(())
}

// Main function
#[tokio::main]
async fn main() {
    let config = NetworkConfig {
        node_address: "127.0.0.1".to_string(),
        port: 8080,
        peers: vec![],
        chain_id: "kari-c1".to_string(), // Updated chain_id to "kari-c1"
        max_connections: 100,
        api_enabled: true,
    };


    save_chain_id(&config.chain_id).expect("Failed to save chain id");

    // Initialize and start the P2P network
    let p2p_network = P2PNetwork {
        peers: Arc::new(Mutex::new(HashMap::new())),
    };

    // If you plan to use it later, prefix with an underscore
    let _p2p_network_clone = p2p_network.peers.clone();

    tokio::spawn(async move {
        p2p_network.start_listener("127.0.0.1:8080").await.unwrap();
    });

    let mut input = String::new();
    load_blockchain();
    let running = Arc::new(Mutex::new(true));

    unsafe {
        BALANCES = Some(Mutex::new(HashMap::new()));
    }

    println!("{}", "Welcome to the Rust Blockchain CLI".bold().cyan());
    print_coin_icon();

    let mut miner_address = String::new();

    // เริ่ม RPC server ในแบ็คกราวนด์
    let rpc_handle = tokio::spawn(async {
        start_rpc_server().await;
    });

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: kari.exe <COMMAND>\n");
        println!("Commands:");
        println!("  start                  Start kar network");
        println!("  keytool                kari keystore tool");
        println!("  move                   Tool to build and test Move applications");
        println!(" ....");
        println!("\nOptions:");
        println!("  -h, --help     Print help");
        println!("  -V, --version  Print version");
        process::exit(1);
    }

        match args[1].as_str() {
            "start" => {
                if miner_address.is_empty() {
                    println!("Please generate an address first using the keytool command.");
                } else {
                    *running.lock().unwrap() = true;
                    println!("{}", "Starting blockchain...".green());
                    let running_clone = Arc::clone(&running);
                    let miner_address_clone = miner_address.clone();
                    tokio::spawn(async move {
                        run_blockchain(running_clone, miner_address_clone);
                    });
                }
            },
            "keytool" => {
                let result = handle_keytool_command();
                if let Some(address) = result {
                    miner_address = address;
                }
            },
            "stop" => {
                *running.lock().unwrap() = false;
                println!("{}", "Stopping blockchain...".red());
                save_blockchain();
            },
            "-h" | "--help" => {
                println!("Usage: kari.exe <COMMAND>\n");
                println!("Commands:");
                println!("  start                  Start kar network");
                println!("  keytool                kari keystore tool");
                println!("  move                   Tool to build and test Move applications");
                println!(" ....");
                println!("\nOptions:");
                println!("  -h, --help     Print help");
                println!("  -V, --version  Print version");
            },
            "-V" | "--version" => {
                // Implement the logic to print version
                println!("kari.exe version 1.0.0");
            },
            _ => {
                println!("Invalid command. Use -h or --help for usage.");
            }
        }
        input.clear();


    // รอให้ RPC server หยุดทำงาน
    rpc_handle.abort();
}

fn handle_keytool_command() -> Option<String> {
    println!("Enter command (1: Generate new address, 2: Check balance, 3: Load existing wallet):");
    let mut command_str = String::new();
    io::stdin().read_line(&mut command_str).unwrap();
    let command: usize = command_str.trim().parse().expect("Invalid input");

    match command {
        1 => {
            println!("Enter mnemonic length (12 or 24):");
            let mut mnemonic_length_str = String::new();
            io::stdin().read_line(&mut mnemonic_length_str).unwrap();
            let mnemonic_length: usize = mnemonic_length_str.trim().parse().expect("Invalid input");

            let (private_key, public_address, seed_phrase) = generate_karix_address(mnemonic_length);
            println!("New address generated:");
            println!("Private Key: {}", private_key.green());
            println!("Public Address: {}", public_address.green());
            println!("Seed Phrase: {}", seed_phrase.green());

            save_wallet(&public_address, &private_key, &seed_phrase);

            Some(public_address)
        },
        2 => {
            println!("Enter public address:");
            let mut public_address = String::new();
            io::stdin().read_line(&mut public_address).unwrap();
            public_address = public_address.trim().to_string();

            // โหลด blockchain ก่อนตรวจสอบยอดคงเหลือ
            load_blockchain();

            let balance = unsafe {
                BALANCES.as_ref().unwrap().lock().unwrap().get(&public_address).cloned().unwrap_or(0)
            };

            println!("Balance for {}: {}", public_address.green(), balance.to_string().green());
            None
        },
        3 => {
            println!("Enter public address to load:");
            let mut public_address = String::new();
            io::stdin().read_line(&mut public_address).unwrap();
            public_address = public_address.trim().to_string();

            if let Some(wallet_data) = load_wallet(&public_address) {
                println!("Wallet loaded:");
                println!("Address: {}", wallet_data["address"].as_str().unwrap().green());
                println!("Private Key: {}", wallet_data["private_key"].as_str().unwrap().green());
                println!("Seed Phrase: {}", wallet_data["seed_phrase"].as_str().unwrap().green());
                Some(public_address)
            } else {
                println!("Wallet not found for address: {}", public_address.red());
                None
            }
        },
        4 => {
            println!("Enter sender's public address:");
            let mut sender_address = String::new();
            io::stdin().read_line(&mut sender_address).unwrap();
            let sender_address = sender_address.trim().to_string();

            println!("Enter receiver's public address:");
            let mut receiver_address = String::new();
            io::stdin().read_line(&mut receiver_address).unwrap();
            let receiver_address = receiver_address.trim().to_string();

            println!("Enter amount to send:");
            let mut amount_str = String::new();
            io::stdin().read_line(&mut amount_str).unwrap();
            let amount: u64 = amount_str.trim().parse().expect("Invalid input for amount");

            if send_coins(sender_address, receiver_address, amount) {
                println!("Transaction successful.");
            } else {
                println!("Transaction failed.");
            }
            None
        },
        _ => {
            println!("{}", "Invalid command".red());
            None
        },
    }
}

// RPC server
fn get_latest_block(_params: Params) -> JsonRpcResult<JsonValue> {
    unsafe {
        if let Some(block) = BLOCKCHAIN.back() {
            Ok(serde_json::to_value(block).unwrap())
        } else {
            Ok(JsonValue::Null)
        }
    }
}

fn get_chain_id(_params: Params) -> JsonRpcResult<JsonValue> {
    Ok(JsonValue::String(CHAIN_ID.to_string()))
}

fn get_block_by_index(params: Params) -> JsonRpcResult<JsonValue> {
    let index: u32 = params.parse().map_err(|e| jsonrpc_core::Error::invalid_params(format!("Invalid index parameter: {}", e)))?;

    unsafe {
        if let Some(block) = BLOCKCHAIN.iter().find(|b| b.index == index) {
            Ok(serde_json::to_value(block).unwrap())
        } else {
            Ok(JsonValue::Null)
        }
    }
}

async fn start_rpc_server() {
    let mut io = IoHandler::new();

    io.add_method("get_latest_block", |params| {
        futures::future::ready(get_latest_block(params)).boxed()
    });

    io.add_method("get_chain_id", |params| {
        futures::future::ready(get_chain_id(params)).boxed()
    });

    io.add_method("get_block_by_index", |params| {
        futures::future::ready(get_block_by_index(params)).boxed()
    });

    let server = ServerBuilder::new(io)
        .cors(DomainsValidation::AllowOnly(vec![AccessControlAllowOrigin::Any]))
        .start_http(&"127.0.0.1:3030".parse().unwrap())
        .expect("Unable to start RPC server");

    println!("RPC server running on http://127.0.0.1:3030");
    server.wait();
}