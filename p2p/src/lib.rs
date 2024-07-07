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
pub struct Peer {
    pub address: String, // Now public, accessible outside the module
}

pub struct P2PNetwork {
    pub peers: Arc<Mutex<HashMap<String, Peer>>>, // Track connected peers
}

impl P2PNetwork {
    pub async fn start_listener(&self, addr: &str) -> io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Listening on {}", addr);

        loop {
            let (socket, _) = listener.accept().await?;
            let peers = self.peers.clone();

            tokio::spawn(async move {
                if let Err(e) = P2PNetwork::handle_connection(socket, peers).await {
                    println!("Failed to handle connection: {}", e);
                }
            });
        }
    }

    async fn handle_connection(mut socket: TcpStream, _peers: Arc<Mutex<HashMap<String, Peer>>>) -> io::Result<()> {
        let mut buffer = [0u8; 1024];

        // Example: Read a message and print it
        let size = socket.read(&mut buffer).await?;
        if size > 0 {
            let message: Message = bincode::deserialize(&buffer[..size]).unwrap();
            println!("Received: {:?}", message);
        }

        // Example: Send a response
        let response = Message { content: "Hello from server".to_string() };
        let encoded = bincode::serialize(&response).unwrap();
        socket.write_all(&encoded).await?;

        Ok(())
    }
}