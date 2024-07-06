use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

struct Peer {
    address: String,
}

async fn handle_connection(mut stream: TcpStream, peers: Arc<Mutex<HashSet<String>>>) {
    let mut buffer = [0; 1024];

    // Example: Read a message from the peer
    match stream.read(&mut buffer).await {
        Ok(_) => {
            let msg = String::from_utf8_lossy(&buffer);
            println!("Received: {}", msg);

            // Example response
            if let Err(e) = stream.write_all(b"Hello, peer!").await {
                println!("Failed to send response: {}", e);
            }
        },
        Err(e) => println!("Failed to read from connection: {}", e),
    }
}

async fn start_server(address: &str, peers: Arc<Mutex<HashSet<String>>>) {
    let listener = TcpListener::bind(address).await.expect("Failed to bind to address");
    println!("Listening on {}", address);

    loop {
        let (stream, _) = listener.accept().await.expect("Failed to accept connection");
        let peers_clone = peers.clone();
        tokio::spawn(async move {
            handle_connection(stream, peers_clone).await;
        });
    }
}

#[tokio::main]
async fn main() {
    let peers = Arc::new(Mutex::new(HashSet::new()));
    start_server("127.0.0.1:3030", peers).await;
}
