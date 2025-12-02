# Kanari Crypto Examples

This directory contains example code demonstrating the features of Kanari Crypto v2.0, including Post-Quantum Cryptography (PQC).

## Available Examples

### 1. 🚀 `pqc_demo.rs` - Post-Quantum Cryptography Demo

Comprehensive demonstration of all available cryptographic algorithms, including classical and post-quantum.

```bash
cargo run --example pqc_demo
```

**Features:**
- Classical algorithms (Ed25519, K256, P256)
- Post-quantum algorithms (Dilithium2/3/5, SPHINCS+)
- Hybrid schemes (Ed25519+Dilithium3, K256+Dilithium3)
- Security level comparison
- Recommendations by use case

### 2. 🔬 `quantum_comparison.rs` - Security Analysis

Detailed security analysis comparing classical vs post-quantum cryptography.

```bash
cargo run --example quantum_comparison
```

**Features:**
- Security comparison table
- Quantum attack scenarios (Shor's & Grover's algorithms)
- Migration timeline (2025-2036+)
- Use case recommendations
- Live key generation demo

### 3. ✍️ `sign_verify.rs` - Digital Signatures

Examples of signing messages and verifying signatures with different algorithms.

```bash
cargo run --example sign_verify
```

**Features:**
- K256 (secp256k1) signing
- P256 (secp256r1) signing
- Ed25519 signing
- Import from private key
- Import from mnemonic phrase
- Post-quantum signature demo

### 4. 💰 `hd_wallet_example.rs` - HD Wallet

Hierarchical Deterministic (HD) wallet creation and management.

```bash
cargo run --example hd_wallet_example
```

**Features:**
- BIP39 mnemonic generation
- BIP32/BIP44 HD wallet derivation
- Wallet persistence
- Password encryption

**Note:** HD wallets currently support classical algorithms only (Ed25519, K256, P256).

### 5. 🌐 `move_coin_wallet.rs` - Move Integration

Integration with Move blockchain platform.

```bash
cargo run --example move_coin_wallet
```

## Quick Start Guide

### Generate Quantum-Safe Keys

```rust
use kanari_crypto::keys::{CurveType, generate_keypair};

// Recommended: Hybrid scheme for best security
let keypair = generate_keypair(CurveType::Ed25519Dilithium3)?;

println!("Address: {}", keypair.address);
println!("Security Level: {}/5", keypair.curve_type.security_level());
println!("Quantum-Safe: {}", keypair.curve_type.is_post_quantum());
```

### Algorithm Selection Guide

| Use Case | Recommended Algorithm | Why |
|----------|----------------------|-----|
| **New Applications** | `Ed25519Dilithium3` | Quantum-safe + Fast + Compatible |
| **Blockchain/Crypto** | `K256Dilithium3` | Bitcoin/Ethereum compatible + Quantum-safe |
| **High Security** | `Dilithium5` | Maximum NIST Level 5 security |
| **Long-Term Secrets** | `SphincsPlusSha256Robust` | Hash-based, ultra-secure |
| **IoT/Embedded** | `Dilithium2` | Lighter weight, still quantum-safe |
| **Legacy Systems** | `Ed25519` or `K256` | Compatible (but migrate soon!) |

## Security Levels

### Classical Algorithms (NOT Quantum-Safe)

```rust
CurveType::Ed25519  // Level 3/5 - Fast, 64-byte signatures
CurveType::K256     // Level 3/5 - Bitcoin/Ethereum compatible
CurveType::P256     // Level 3/5 - NIST P-256
```

⚠️ **Warning:** Vulnerable to quantum computer attacks (Shor's algorithm)

### Post-Quantum Algorithms (Quantum-Safe)

```rust
CurveType::Dilithium2              // Level 4/5 - Fast, NIST Level 2
CurveType::Dilithium3              // Level 5/5 - Balanced, NIST Level 3
CurveType::Dilithium5              // Level 5/5 - Maximum, NIST Level 5
CurveType::SphincsPlusSha256Robust // Level 5/5 - Ultra-secure
```

✅ **Protected:** Resistant to quantum attacks

### Hybrid Schemes (Best Practice)

```rust
CurveType::Ed25519Dilithium3  // Level 5/5 - Best of both worlds
CurveType::K256Dilithium3     // Level 5/5 - Blockchain + Quantum-safe
```

✅ **Recommended:** Combines classical and quantum security

## Performance Comparison

| Algorithm | Key Gen | Signature Size | Speed |
|-----------|---------|----------------|-------|
| Ed25519 | ~0.1 ms | 64 bytes | ⚡ Very Fast |
| K256 | ~0.1 ms | ~70 bytes | ⚡ Fast |
| Dilithium2 | ~0.2 ms | ~2.5 KB | 🚀 Fast |
| Dilithium3 | ~0.3 ms | ~4 KB | 🚀 Fast |
| Dilithium5 | ~0.5 ms | ~5 KB | 🚀 Fast |
| SPHINCS+ | ~50 ms | ~50 KB | 🐢 Slow |
| Ed25519+Dilithium3 | ~0.4 ms | ~4 KB | 🚀 Fast |

## Migration Path

### Phase 1 (2025-2027): Add PQC Support ✅

```rust
// Keep existing classical keys
let classical = generate_keypair(CurveType::Ed25519)?;

// Add new PQC keys
let pqc = generate_keypair(CurveType::Dilithium3)?;
```

### Phase 2 (2028-2030): Use Hybrid

```rust
// Default to hybrid for new keys
let keypair = generate_keypair(CurveType::Ed25519Dilithium3)?;
```

### Phase 3 (2031+): Pure PQC

```rust
// Migrate to pure post-quantum
let keypair = generate_keypair(CurveType::Dilithium3)?;
```

## Common Patterns

### Check Algorithm Properties

```rust
let keypair = generate_keypair(CurveType::Ed25519Dilithium3)?;

// Check if quantum-safe
if keypair.curve_type.is_post_quantum() {
    println!("✅ This algorithm is quantum-safe");
}

// Check if hybrid
if keypair.curve_type.is_hybrid() {
    println!("✅ This is a hybrid scheme");
}

// Get security level
let level = keypair.curve_type.security_level();
println!("Security: {}/5", level);
```

### Sign and Verify Messages

```rust
use kanari_crypto::{sign_message, verify_signature};

let keypair = generate_keypair(CurveType::Ed25519)?;
let message = b"Hello, Quantum World!";

// Sign
let signature = sign_message(&keypair.private_key, message, CurveType::Ed25519)?;

// Verify
let is_valid = verify_signature(&keypair.address, message, &signature)?;
```

## Testing

Run all examples:

```bash
# Build all examples
cargo build --examples

# Run specific example
cargo run --example pqc_demo
cargo run --example quantum_comparison
cargo run --example sign_verify
cargo run --example hd_wallet_example
```

## Documentation

For more information, see:

- [POST_QUANTUM_GUIDE.md](../POST_QUANTUM_GUIDE.md) - Complete PQC guide
- [QUANTUM_SECURITY_ANALYSIS.md](../QUANTUM_SECURITY_ANALYSIS.md) - Security analysis
- [SECURITY_ENHANCEMENTS.md](../SECURITY_ENHANCEMENTS.md) - All security features
- [UPGRADE_SUMMARY_TH.md](../UPGRADE_SUMMARY_TH.md) - Thai summary

## Support

- GitHub Issues: [kanari-cp/issues](https://github.com/jamesatomc/kanari-cp/issues)
- Documentation: `/crates/kanari-crypto/`

---

**Kanari Crypto v2.0** - Quantum-Safe Cryptography 🔐
