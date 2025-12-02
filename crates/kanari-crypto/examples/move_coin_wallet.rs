use kanari_crypto::keys::{CurveType, generate_keypair};
use kanari_crypto::wallet::save_wallet;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate a new keypair (default K256)
    let kp = generate_keypair(CurveType::K256).expect("failed to generate keypair");

    println!("Generated address: {}", kp.address);

    // Derive Address type from string using kanari-types
    let address = kp.address.parse()?;

    // Save wallet using a dummy password (for demo only)
    let password = "password123!";

    save_wallet(&address, &kp.private_key, "", password, kp.curve_type)
        .expect("failed to save wallet");

    println!("Saved wallet for address {}", kp.address);

    // Show how to call local movec in a shell (not executed here)
    if env::var_os("KANARI_SHOW_MOVEC_CMD").is_some() {
        println!("Run movec to compile Move modules:");
        println!("  movec build --package-dir path\\to\\move_package");
    }

    Ok(())
}
