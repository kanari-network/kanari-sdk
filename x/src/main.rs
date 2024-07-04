use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use bincode::Options;
use digest::Digest;
use serde::{Deserialize, Serialize};
use hex;
use sha2::Sha256;
use jsonrpc_core::futures::FutureExt;
use jsonrpc_core::{IoHandler, Params, Result as JsonRpcResult};
use jsonrpc_http_server::{ServerBuilder, AccessControlAllowOrigin, DomainsValidation};
use serde_json::Value as JsonValue;
use secp256k1::{Secp256k1, PublicKey, SecretKey};
use rand::rngs::OsRng;


static CHAIN_ID: u64 = 1; // หรือค่าอื่นที่คุณต้องการ

#[derive(Serialize, Deserialize, Clone)]
struct Transaction {
    sender: String,
    receiver: String,
    amount: u64,
}

#[derive(Serialize, Deserialize, Clone)]
struct Block {
    chain_id: u64,
    index: u32,
    timestamp: u64,
    data: Vec<u8>, // เปลี่ยนเป็น Vec<u8> เพื่อเก็บข้อมูลแบบไบนารี
    hash: String,
    prev_hash: String,
    tokens: u64,
    token_name: String,
    transactions: Vec<Transaction>,
}

impl Block {
    fn new(index: u32, data: Vec<u8>, prev_hash: String, tokens: u64, transactions: Vec<Transaction>) -> Block {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut hasher = Sha256::new();
        hasher.update(&index.to_le_bytes());
        hasher.update(&timestamp.to_le_bytes());
        hasher.update(&data);
        hasher.update(prev_hash.as_bytes());
        hasher.update(&tokens.to_le_bytes());

        let hash = hex::encode(hasher.finalize());

        Block {
            chain_id: CHAIN_ID,
            index,
            timestamp,
            data,
            hash,
            prev_hash,
            tokens,
            token_name: String::from("Kanari"),
            transactions,
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

static mut TOTAL_TOKENS: u64 = 0;

static mut BLOCKCHAIN: VecDeque<Block> = VecDeque::new();

fn run_blockchain(running: Arc<Mutex<bool>>) {
    let max_tokens = 11_000_000;
    let mut tokens_per_block = 25;
    let halving_interval = 210_000;
    let block_size = 2_250_000; // 2.25 MB in bytes

    unsafe {
        if BLOCKCHAIN.is_empty() {
            let genesis_data = vec![0; block_size]; // สร้างข้อมูลขนาด 2.25 MB
            let genesis_transactions = vec![];
            BLOCKCHAIN.push_back(Block::new(0, genesis_data, String::from("0"), tokens_per_block, genesis_transactions));
            TOTAL_TOKENS += tokens_per_block;
        }

        while TOTAL_TOKENS < max_tokens {
            if !*running.lock().unwrap() {
                break;
            }

            let prev_block = BLOCKCHAIN.back().unwrap();
            let new_data = vec![0; block_size]; // สร้างข้อมูลขนาด 2.25 MB

            let transactions = vec![
                Transaction { sender: String::from("Alice"), receiver: String::from("Bob"), amount: 10 },
                Transaction { sender: String::from("Charlie"), receiver: String::from("Dave"), amount: 20 },
            ];

            let new_block = Block::new(prev_block.index + 1, new_data, prev_block.hash.clone(), tokens_per_block, transactions);
            if !new_block.verify(prev_block) {
                println!("Block verification failed!");
                break;
            }

            BLOCKCHAIN.push_back(new_block.clone());
            TOTAL_TOKENS += tokens_per_block;

            println!("New block hash: {}", new_block.hash);

            if BLOCKCHAIN.len() % halving_interval == 0 {
                tokens_per_block /= 2;
            }

            println!("blocks: {}, Total tokens: {}", BLOCKCHAIN.len(), TOTAL_TOKENS);
            thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}

fn save_blockchain() {
    unsafe {
        let data = bincode::serialize(&BLOCKCHAIN).expect("Failed to serialize blockchain");
        std::fs::write("blockchain.bin", data).expect("Unable to write to file");
    }
}

fn load_blockchain() {
    unsafe {
        if std::path::Path::new("blockchain.bin").exists() {
            let data = std::fs::read("blockchain.bin").expect("Unable to read file");
            BLOCKCHAIN = bincode::deserialize(&data).expect("Failed to deserialize blockchain");
        }
    }
}

fn generate_address() -> (String, String) {
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

    (
        secret_key.display_secret().to_string(),
        hex::encode(&public_key.serialize_uncompressed()[1..]) // Remove the first byte (0x04) which indicates uncompressed key
    )
}

fn main() {
    let mut input = String::new();
    load_blockchain();
    let running = Arc::new(Mutex::new(false));
    let running_clone = Arc::clone(&running);

    let blockchain_handle = thread::spawn(move || {
        run_blockchain(running_clone);
    });

    let rpc_handle = thread::spawn(|| {
        start_rpc_server();
    });

    print!("Enter command:\n \
        start     start new network\n \
        transfer  send coin\n \
        move      send coin\n \
        client    send coin\n \
        keytool   send coin\n \
    ");
    loop {

        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "start" => {
                *running.lock().unwrap() = true;
                println!("Starting blockchain...");
                let running_clone = Arc::clone(&running);
                thread::spawn(move || {
                    run_blockchain(running_clone);
                });
            }
            "keytool" => {
                let (private_key, public_address) = generate_address();
                println!("New address generated:");
                println!("Private Key: {}", private_key);
                println!("Public Address: {}", public_address);
            }
            "stop" => {
                *running.lock().unwrap() = false;
                println!("Stopping blockchain...");
                save_blockchain();
                break;  // ออกจาก loop เมื่อหยุด
            }
            _ => println!("Invalid command"),
        }
        input.clear();
    }

    // รอให้ thread ทำงานเสร็จ
    blockchain_handle.join().unwrap();
    rpc_handle.join().unwrap();
}


async fn get_latest_block(_params: Params) -> JsonRpcResult<JsonValue> {
    unsafe {
        if let Some(block) = BLOCKCHAIN.back() {
            Ok(serde_json::to_value(block).unwrap())
        } else {
            Ok(JsonValue::Null)
        }
    }
}

async fn get_chain_id(_params: Params) -> JsonRpcResult<JsonValue> {
    Ok(JsonValue::Number(CHAIN_ID.into()))
}

async fn get_block_by_index(params: Params) -> JsonRpcResult<JsonValue> {
    let index: u32 = params.parse().map_err(|e| jsonrpc_core::Error::invalid_params(format!("Invalid index parameter: {}", e)))?;

    unsafe {
        if let Some(block) = BLOCKCHAIN.iter().find(|b| b.index == index) {
            Ok(serde_json::to_value(block).unwrap())
        } else {
            Ok(JsonValue::Null)
        }
    }
}

fn start_rpc_server() {
    let mut io = IoHandler::new();

    io.add_method("get_latest_block", |params| {
        async move {
            get_latest_block(params).await
        }.boxed()
    });

    io.add_method("get_chain_id", |params| {
        async move {
            get_chain_id(params).await
        }.boxed()
    });

    io.add_method("get_block_by_index", |params| {
        async move {
            get_block_by_index(params).await
        }.boxed()
    });

    io.add_method("get_total_tokens", |_params| {
        async move {
            unsafe {
                Ok(JsonValue::Number(TOTAL_TOKENS.into()))
            }
        }.boxed()
    });

    let server = ServerBuilder::new(io)
        .cors(DomainsValidation::AllowOnly(vec![
            AccessControlAllowOrigin::Any,
        ]))
        .start_http(&"127.0.0.1:3030".parse().unwrap())
        .expect("Unable to start RPC server");

    println!("RPC server running on http://127.0.0.1:3030");

    server.wait();
}