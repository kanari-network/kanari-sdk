// Quantum Security Comparison Example
// cargo run -p kanari-crypto --example quantum_comparison

use kanari_crypto::keys::{CurveType, generate_keypair};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("🔬 Quantum Security Analysis - Kanari Crypto v2.0");
    println!("==================================================\n");

    println!("📊 SECURITY COMPARISON TABLE");
    println!("============================\n");

    print_security_table();

    println!("\n⚔️  QUANTUM ATTACK SCENARIOS");
    println!("============================\n");

    analyze_shor_attack();
    analyze_grover_attack();

    println!("\n📅 MIGRATION TIMELINE");
    println!("====================\n");

    print_migration_timeline();

    println!("\n💡 RECOMMENDATIONS BY USE CASE");
    println!("===============================\n");

    print_use_case_recommendations();

    println!("\n🧪 LIVE DEMO: Generate Keys");
    println!("============================\n");

    demo_key_generation()?;

    Ok(())
}

fn print_security_table() {
    println!("┌─────────────────────┬──────────────┬────────────────┬───────────┐");
    println!("│ Algorithm           │ Classical    │ Quantum        │ Status    │");
    println!("├─────────────────────┼──────────────┼────────────────┼───────────┤");
    println!("│ Ed25519             │ ⭐⭐⭐⭐⭐    │ ❌ Broken      │ Legacy    │");
    println!("│ K256 (secp256k1)    │ ⭐⭐⭐⭐⭐    │ ❌ Broken      │ Legacy    │");
    println!("│ P256 (secp256r1)    │ ⭐⭐⭐⭐⭐    │ ❌ Broken      │ Legacy    │");
    println!("├─────────────────────┼──────────────┼────────────────┼───────────┤");
    println!("│ Dilithium2          │ ⭐⭐⭐⭐      │ ⭐⭐⭐⭐        │ PQC       │");
    println!("│ Dilithium3          │ ⭐⭐⭐⭐⭐    │ ⭐⭐⭐⭐⭐      │ PQC ⭐    │");
    println!("│ Dilithium5          │ ⭐⭐⭐⭐⭐    │ ⭐⭐⭐⭐⭐      │ PQC       │");
    println!("│ SPHINCS+            │ ⭐⭐⭐⭐⭐    │ ⭐⭐⭐⭐⭐      │ PQC       │");
    println!("├─────────────────────┼──────────────┼────────────────┼───────────┤");
    println!("│ Ed25519+Dilithium3  │ ⭐⭐⭐⭐⭐    │ ⭐⭐⭐⭐⭐      │ Hybrid ⭐  │");
    println!("│ K256+Dilithium3     │ ⭐⭐⭐⭐⭐    │ ⭐⭐⭐⭐⭐      │ Hybrid ⭐  │");
    println!("└─────────────────────┴──────────────┴────────────────┴───────────┘");
}

fn analyze_shor_attack() {
    println!("1. Shor's Algorithm Impact:");
    println!("   Target: Factorization & Discrete Logarithm problems");
    println!("   Effect: Breaks RSA, ECDSA, ECDH in polynomial time");
    println!();
    println!("   ❌ Vulnerable:");
    println!("      - Ed25519 (EdDSA)");
    println!("      - K256/P256 (ECDSA)");
    println!("      - RSA");
    println!();
    println!("   ✅ Protected:");
    println!("      - Dilithium (lattice-based)");
    println!("      - SPHINCS+ (hash-based)");
    println!("      - Hybrid schemes");
}

fn analyze_grover_attack() {
    println!("\n2. Grover's Algorithm Impact:");
    println!("   Target: Symmetric encryption & hash functions");
    println!("   Effect: Quadratic speedup (reduces security by 50%)");
    println!();
    println!("   ⚠️  Affected:");
    println!("      - AES-256 → effectively AES-128 (still secure)");
    println!("      - SHA-256 → effectively SHA-128");
    println!();
    println!("   ✅ Mitigation:");
    println!("      - Use AES-256 (remains secure at 128-bit)");
    println!("      - Use SHA3-512 for higher security margin");
    println!("      - Combine with Kyber KEM (future)");
}

fn print_migration_timeline() {
    println!("Phase 1 (2025-2027): Preparation");
    println!("  ✅ Add PQC support (DONE)");
    println!("  ✅ Implement hybrid schemes (DONE)");
    println!("  → Action: Test and validate PQC implementations");
    println!();
    println!("Phase 2 (2028-2030): Gradual Adoption");
    println!("  → New users: Issue PQC/Hybrid keys by default");
    println!("  → Existing users: Voluntary migration");
    println!("  → Critical systems: Mandate hybrid keys");
    println!();
    println!("Phase 3 (2031-2035): Full Migration");
    println!("  → Deprecate classical-only keys");
    println!("  → All new keys use PQC or hybrid");
    println!("  → Provide migration tools");
    println!();
    println!("Phase 4 (2036+): PQC Only");
    println!("  → Remove classical support (except verification)");
    println!("  → Pure post-quantum operations");
    println!("  → Full quantum resistance");
}

fn print_use_case_recommendations() {
    println!("1. 💰 Financial & Banking:");
    println!("   Algorithm: Dilithium5 or SPHINCS+");
    println!("   Reason: Maximum security for 30+ year protection");
    println!();
    println!("2. 🔗 Blockchain & Cryptocurrency:");
    println!("   Algorithm: K256+Dilithium3 (Hybrid)");
    println!("   Reason: Maintains compatibility + quantum-safe");
    println!();
    println!("3. 🏢 Enterprise & Government:");
    println!("   Algorithm: Ed25519+Dilithium3 (Hybrid)");
    println!("   Reason: Meets compliance + smooth transition");
    println!();
    println!("4. 📱 IoT & Embedded:");
    println!("   Algorithm: Dilithium2");
    println!("   Reason: Lighter weight, still quantum-safe");
    println!();
    println!("5. 🌐 General Purpose:");
    println!("   Algorithm: Dilithium3");
    println!("   Reason: Best balance of security & performance");
}

fn demo_key_generation() -> Result<(), Box<dyn Error>> {
    println!("Generating keys with different security levels...\n");

    // Classical
    println!("1. Classical (Ed25519) - NOT Quantum-Safe:");
    let classical = generate_keypair(CurveType::Ed25519)?;
    println!("   Address: {}", classical.address);
    println!("   Security: {}/5", classical.curve_type.security_level());
    println!("   Quantum-Safe: ❌");
    println!("   Size: Small (64-byte signature)");

    // Post-Quantum
    println!("\n2. Post-Quantum (Dilithium3) - Quantum-Safe:");
    let pqc = generate_keypair(CurveType::Dilithium3)?;
    println!("   Address: {}", pqc.address);
    println!("   Security: {}/5", pqc.curve_type.security_level());
    println!("   Quantum-Safe: ✅");
    println!("   Size: Medium (~4KB signature)");

    // Hybrid
    println!("\n3. Hybrid (Ed25519+Dilithium3) - Best of Both:");
    let hybrid = generate_keypair(CurveType::Ed25519Dilithium3)?;
    println!("   Address: {}", hybrid.address);
    println!("   Security: {}/5", hybrid.curve_type.security_level());
    println!("   Quantum-Safe: ✅");
    println!("   Is Hybrid: {}", hybrid.curve_type.is_hybrid());
    println!("   Size: Medium (~4KB signature)");
    println!("   Benefits: Fast + Quantum-safe + Compatible");

    println!("\n✅ Key generation completed!");
    println!("\n🎯 Recommendation:");
    println!("   Use Hybrid scheme (Ed25519+Dilithium3) for best results");

    Ok(())
}
