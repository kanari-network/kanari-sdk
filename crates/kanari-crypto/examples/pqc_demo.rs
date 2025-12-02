// Example demonstrating Post-Quantum Cryptography in Kanari Crypto v2.0
// cargo run -p kanari-crypto --example pqc_demo

use kanari_crypto::keys::{CurveType, generate_keypair};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("🔐 Kanari Crypto v2.0 - Post-Quantum Cryptography Demo");
    println!("======================================================\n");

    // Classical Algorithms (For comparison - NOT quantum-safe)
    println!("📊 CLASSICAL ALGORITHMS (Legacy)");
    println!("--------------------------------");

    demo_algorithm(CurveType::Ed25519, "Fast, 64-byte signatures")?;
    demo_algorithm(CurveType::K256, "Bitcoin/Ethereum compatible")?;
    demo_algorithm(CurveType::P256, "NIST P-256")?;

    println!("\n🚀 POST-QUANTUM ALGORITHMS (NIST Standard)");
    println!("-------------------------------------------");

    demo_algorithm(CurveType::Dilithium2, "Fast, NIST Level 2")?;
    demo_algorithm(
        CurveType::Dilithium3,
        "Balanced, NIST Level 3 (Recommended)",
    )?;
    demo_algorithm(CurveType::Dilithium5, "Maximum security, NIST Level 5")?;
    demo_algorithm(
        CurveType::SphincsPlusSha256Robust,
        "Hash-based, ultra-secure",
    )?;

    println!("\n⭐ HYBRID SCHEMES (Best Practice)");
    println!("----------------------------------");

    demo_algorithm(CurveType::Ed25519Dilithium3, "Ed25519 + Dilithium3")?;
    demo_algorithm(CurveType::K256Dilithium3, "K256 + Dilithium3")?;

    println!("\n📈 COMPARISON");
    println!("------------");
    compare_algorithms();

    println!("\n✅ RECOMMENDATIONS");
    println!("------------------");
    print_recommendations();

    Ok(())
}

fn demo_algorithm(curve_type: CurveType, description: &str) -> Result<(), Box<dyn Error>> {
    let keypair = generate_keypair(curve_type)?;

    let quantum_safe = if keypair.curve_type.is_post_quantum() {
        "✅"
    } else {
        "❌"
    };
    let hybrid = if keypair.curve_type.is_hybrid() {
        "✅"
    } else {
        "  "
    };

    println!("\n{}", curve_type);
    println!("  Description: {}", description);
    println!(
        "  Security Level: {}/5",
        keypair.curve_type.security_level()
    );
    println!("  Quantum-Safe: {}", quantum_safe);
    println!("  Hybrid: {}", hybrid);
    println!("  Address: {}", keypair.address);
    println!("  Public Key Length: {} chars", keypair.public_key.len());
    println!("  Private Key Length: {} chars", keypair.private_key.len());

    Ok(())
}

fn compare_algorithms() {
    println!("\n| Algorithm              | Quantum-Safe | Security | Size    |");
    println!("|------------------------|--------------|----------|---------|");
    println!("| Ed25519                | ❌           | 3/5      | Small   |");
    println!("| K256 (secp256k1)       | ❌           | 3/5      | Small   |");
    println!("| Dilithium2             | ✅           | 4/5      | Medium  |");
    println!("| Dilithium3             | ✅           | 5/5      | Medium  |");
    println!("| Dilithium5             | ✅           | 5/5      | Large   |");
    println!("| SPHINCS+               | ✅           | 5/5      | X-Large |");
    println!("| Ed25519+Dilithium3     | ✅           | 5/5      | Medium  |");
    println!("| K256+Dilithium3        | ✅           | 5/5      | Medium  |");
}

fn print_recommendations() {
    println!("\n💡 For New Applications (2025+):");
    println!("   Use: CurveType::Ed25519Dilithium3 (Hybrid)");
    println!("   Why: Quantum-safe + Fast + Compatible");

    println!("\n💡 For High Security:");
    println!("   Use: CurveType::Dilithium5");
    println!("   Why: Maximum NIST Level 5 security");

    println!("\n💡 For Blockchain/Crypto:");
    println!("   Use: CurveType::K256Dilithium3 (Hybrid)");
    println!("   Why: Bitcoin/Ethereum compatible + Quantum-safe");

    println!("\n💡 For Long-Term Secrets (30+ years):");
    println!("   Use: CurveType::SphincsPlusSha256Robust");
    println!("   Why: Hash-based, ultra-secure, future-proof");

    println!("\n⚠️  For Legacy Systems:");
    println!("   Use: Classical algorithms (Ed25519, K256)");
    println!("   Note: NOT quantum-safe, migrate to hybrid ASAP");
}
