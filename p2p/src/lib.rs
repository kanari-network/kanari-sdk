use serde::{Serialize, Deserialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Stake {
    pub amount: u64,
    pub validator_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Validator {
    pub address: String,
    pub stake: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Peer {
    pub address: String,
}

pub struct P2PNetwork {
    pub peers: Arc<Mutex<HashMap<String, Peer>>>,
    pub stakes: Arc<Mutex<HashMap<String, Stake>>>,
    pub validators: Arc<Mutex<Vec<Validator>>>,
}

impl P2PNetwork {
    pub fn new() -> Self {
        P2PNetwork {
            peers: Arc::new(Mutex::new(HashMap::new())),
            stakes: Arc::new(Mutex::new(HashMap::new())),
            validators: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_stake(&self, address: String, amount: u64) {
        let mut stakes = self.stakes.lock().unwrap();
        let stake = stakes.entry(address.clone()).or_insert(Stake {
            amount: 0,
            validator_address: address.clone(),
        });
        stake.amount += amount;

        self.update_validators();
    }

    fn update_validators(&self) {
        let stakes = self.stakes.lock().unwrap();
        let mut validators = self.validators.lock().unwrap();
        validators.clear();

        for stake in stakes.values() {
            validators.push(Validator {
                address: stake.validator_address.clone(),
                stake: stake.amount,
            });
        }

        validators.sort_by(|a, b| b.stake.cmp(&a.stake));
    }

    pub fn select_validator(&self) -> Option<Validator> {
        let validators = self.validators.lock().unwrap();
        validators.first().cloned()
    }

    pub async fn start_listener(&self, addr: &str) -> io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Listening on {}", addr);

        loop {
            let (socket, _) = listener.accept().await?;
            let validators = self.validators.clone();

            tokio::spawn(async move {
                if let Err(e) = P2PNetwork::handle_connection(socket, validators).await {
                    println!("Failed to handle connection: {}", e);
                }
            });
        }
    }

    async fn handle_connection(
        mut socket: TcpStream,
        validators: Arc<Mutex<Vec<Validator>>>,
    ) -> io::Result<()> {
        let mut buffer = [0u8; 1024];

        let size = socket.read(&mut buffer).await?;
        if size > 0 {
            let message: Message = bincode::deserialize(&buffer[..size]).unwrap();
            println!("Received: {:?}", message);

            let validator = validators.lock().unwrap().first().cloned();
            if let Some(validator) = validator {
                println!("Current validator: {:?}", validator);
                // Add block validation logic here
            }
        }

        let response = Message { content: "Hello from server".to_string() };
        let encoded = bincode::serialize(&response).unwrap();
        socket.write_all(&encoded).await?;

        Ok(())
    }
}