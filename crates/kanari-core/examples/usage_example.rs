// Example: Using Cryptographic Signatures and Persistent Storage together
//
// This example demonstrates how to:
// 1. Generate Ed25519 keypair
// 2. Sign DAG vertices with Ed25519
// 3. Store vertices persistently with RocksDB
// 4. Verify signatures on retrieval
//
// Run with: cargo run -p kanari-core --example usage_example

use kanari_core::blockchain::{DagVertex, Ed25519Keypair, PersistentDagStore};

fn main() -> anyhow::Result<()> {
    // 1. Generate cryptographic keypair
    println!("Generating Ed25519 keypair...");
    let ed25519_keypair = Ed25519Keypair::generate();

    println!("Ed25519 public key: {:?}", ed25519_keypair.public());

    // 2. Create a DAG vertex
    println!("\nCreating DAG vertex...");
    let vertex = DagVertex::new(
        1,                         // round
        "validator_0".to_string(), // author (AuthorityId)
        vec![],                    // parents
        vec![],                    // transactions
        vec![0xaa; 32],            // state_root
    );

    // 3. Sign the vertex
    let vertex_bytes = bcs::to_bytes(&vertex)?;
    let ed_signature = ed25519_keypair.sign(&vertex_bytes);

    println!("Vertex ID: {:?}", vertex.id);
    println!("Ed25519 signature created: {} bytes", ed_signature.len());

    // 4. Verify the signature
    println!("\nVerifying signature...");
    let verified = Ed25519Keypair::verify(&ed25519_keypair.public(), &vertex_bytes, &ed_signature);
    println!("Signature verification: {}", verified.is_ok());

    // 5. Store vertex in RocksDB
    println!("\nStoring vertex in RocksDB...");
    let store = PersistentDagStore::new("./example_dag_data")?;
    store.put_vertex(&vertex)?;
    println!("Vertex stored successfully");

    // 6. Retrieve vertex from storage
    println!("\nRetrieving vertex from storage...");
    let retrieved = store.get_vertex(&vertex.id)?;

    if let Some(v) = retrieved {
        println!("Vertex retrieved: round={}, author={}", v.round, v.author);

        // Verify signature on retrieved vertex
        let retrieved_bytes = bcs::to_bytes(&v)?;
        let re_verified =
            Ed25519Keypair::verify(&ed25519_keypair.public(), &retrieved_bytes, &ed_signature);
        println!("Re-verification after retrieval: {}", re_verified.is_ok());
    }

    // 7. Storage statistics
    println!("\n--- Storage Statistics ---");
    let stats = store.get_stats()?;
    println!("Total vertices: {}", stats.vertex_count);
    println!("Total checkpoints: {}", stats.checkpoint_count);

    // 8. Query vertices by round
    println!("\n--- Query Vertices by Round ---");
    let round_1_vertices = store.get_vertices_by_round(1)?;
    println!("Found {} vertices in round 1", round_1_vertices.len());

    println!("\n✅ Example completed successfully!");
    println!("Note: Data saved in ./example_dag_data (remove manually if needed)");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_example() {
        main().expect("Example should run successfully");
    }
}
