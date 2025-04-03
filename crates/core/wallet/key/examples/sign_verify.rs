use key::{
    generate_karix_address, sign_message_k256, sign_message_p256, verify_signature,
    CurveType, import_from_private_key,
};

fn main() {
    println!("Kanari Wallet - Signing and Verification Example");
    println!("================================================");
    
    // Example 1: K256 (secp256k1) signing and verification
    println!("\nExample 1: K256 Curve");
    println!("---------------------");
    
    // Generate a new K256 wallet
    let (private_key, public_address, seed_phrase) = generate_karix_address(12, CurveType::K256);
    
    println!("Generated new K256 wallet:");
    println!("  Address: {}", public_address);
    println!("  Private Key: {}", private_key);
    println!("  Seed Phrase: {}", seed_phrase);
    
    // Sign a message
    let message = b"Hello, Kanari!";
    let k256_signature = match sign_message_k256(&private_key, message) {
        Ok(sig) => {
            println!("\nSigned message successfully!");
            println!("  Message: \"{}\"", String::from_utf8_lossy(message));
            println!("  Signature (hex): {}", hex::encode(&sig));
            sig
        },
        Err(e) => {
            println!("Error signing message: {}", e);
            Vec::new()
        }
    };
    
    // Verify the signature
    if !k256_signature.is_empty() {
        match verify_signature(&public_address, message, &k256_signature) {
            Ok(true) => println!("✅ Signature verification successful!"),
            Ok(false) => println!("❌ Signature verification failed!"),
            Err(e) => println!("Error verifying signature: {}", e),
        }
        
        // Try with wrong message
        let wrong_message = b"Wrong message!";
        match verify_signature(&public_address, wrong_message, &k256_signature) {
            Ok(true) => println!("❌ Signature incorrectly verified with wrong message!"),
            Ok(false) => println!("✅ Signature correctly rejected for wrong message!"),
            Err(e) => println!("Error verifying signature: {}", e),
        }
    }
    
    // Example 2: P256 (secp256r1) signing and verification
    println!("\nExample 2: P256 Curve");
    println!("---------------------");
    
    // Generate a new P256 wallet
    let (private_key, public_address, seed_phrase) = generate_karix_address(12, CurveType::P256);
    
    println!("Generated new P256 wallet:");
    println!("  Address: {}", public_address);
    println!("  Private Key: {}", private_key);
    println!("  Seed Phrase: {}", seed_phrase);
    
    // Sign a message
    let message = b"Hello, Kanari with P256!";
    let p256_signature = match sign_message_p256(&private_key, message) {
        Ok(sig) => {
            println!("\nSigned message successfully!");
            println!("  Message: \"{}\"", String::from_utf8_lossy(message));
            println!("  Signature (hex): {}", hex::encode(&sig));
            sig
        },
        Err(e) => {
            println!("Error signing message: {}", e);
            Vec::new()
        }
    };
    
    // Verify the signature
    if !p256_signature.is_empty() {
        match verify_signature(&public_address, message, &p256_signature) {
            Ok(true) => println!("✅ Signature verification successful!"),
            Ok(false) => println!("❌ Signature verification failed!"),
            Err(e) => println!("Error verifying signature: {}", e),
        }
    }
    
    // Example 3: Importing from private key and signing
    println!("\nExample 3: Importing from Private Key");
    println!("----------------------------------");
    
    // Import the first wallet from private key
    match import_from_private_key(&private_key, CurveType::P256) {
        Ok((imported_private_key, _, imported_public_address)) => {
            println!("Successfully imported wallet:");
            println!("  Address: {}", imported_public_address);
            
            // Verify it's the same address
            assert_eq!(public_address, imported_public_address);
            println!("✅ Imported address matches original!");
            
            // Sign the same message with imported key
            let message = b"Hello, Kanari with P256!";
            match sign_message_p256(&imported_private_key, message) {
                Ok(sig) => {
                    println!("\nSigned message with imported key successfully!");
                    println!("  Signature (hex): {}", hex::encode(&sig));
                    
                    // Verify this signature matches the original
                    if sig == p256_signature {
                        println!("✅ Signature from imported key matches original signature!");
                    } else {
                        println!("❌ Signature from imported key differs from original!");
                    }
                },
                Err(e) => println!("Error signing with imported key: {}", e),
            }
        },
        Err(e) => println!("Error importing wallet: {}", e),
    }
}
