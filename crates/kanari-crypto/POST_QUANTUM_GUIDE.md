# 🔐 Post-Quantum Cryptography Guide

## Kanari Crypto v2.0 - Quantum-Resistant Security

**Date**: November 24, 2025  
**Status**: ✅ Production Ready  
**Security Level**: Maximum (Level 5/5)

---

## 🎯 Overview

Kanari Crypto v2.0 introduces **Post-Quantum Cryptography (PQC)** based on NIST-standardized algorithms, providing protection against attacks from quantum computers.

### Why Post-Quantum Cryptography?

**Threat Timeline:**

- **2025-2030**: Large-scale quantum computers in development
- **2030-2035**: Quantum computers capable of breaking current cryptography
- **Now**: Implement quantum-safe solutions for long-term security

**Vulnerable to Quantum Attacks:**

- ❌ RSA, ECDSA, Ed25519 (Shor's algorithm)
- ❌ ECDH key exchange (Shor's algorithm)
- ⚠️ AES-256 → reduced to AES-128 equivalent (Grover's algorithm)

**Quantum-Resistant:**

- ✅ Dilithium signatures (lattice-based)
- ✅ SPHINCS+ signatures (hash-based)
- ✅ Kyber KEM (lattice-based)
- ✅ SHA3-512, SHAKE256 (quantum-resistant hashing)

---

## 🔑 Signature Algorithms

### Classical Algorithms (For Compatibility)

```rust
use kanari_crypto::{generate_keypair, CurveType};

// Ed25519 - Fast, 64-byte signatures (NOT quantum-safe)
let keypair = generate_keypair(CurveType::Ed25519)?;

// K256 (secp256k1) - Bitcoin/Ethereum compatible (NOT quantum-safe)
let keypair = generate_keypair(CurveType::K256)?;

// P256 (secp256r1) - NIST standard (NOT quantum-safe)
let keypair = generate_keypair(CurveType::P256)?;
```

### Post-Quantum Algorithms (NIST Standard)

```rust
// Dilithium2 - Fast, ~2.5KB signatures, NIST Level 2
let keypair = generate_keypair(CurveType::Dilithium2)?;

// Dilithium3 - Balanced, ~4KB signatures, NIST Level 3 (⭐ Recommended)
let keypair = generate_keypair(CurveType::Dilithium3)?;

// Dilithium5 - Maximum security, ~5KB signatures, NIST Level 5
let keypair = generate_keypair(CurveType::Dilithium5)?;

// SPHINCS+ - Hash-based, ~50KB signatures, ultra-secure
let keypair = generate_keypair(CurveType::SphincsPlusSha256Robust)?;
```

### Hybrid Algorithms (Best Practice)

Hybrid schemes provide **both classical and quantum security** during the transition period:

```rust
// Ed25519 + Dilithium3 - Fast classical + quantum-safe
let keypair = generate_keypair(CurveType::Ed25519Dilithium3)?;

// K256 + Dilithium3 - Bitcoin/Ethereum compatible + quantum-safe
let keypair = generate_keypair(CurveType::K256Dilithium3)?;
```

**Hybrid Security**: An attacker must break *both* classical and PQC algorithms.

---

## 🔐 Encryption Algorithms

### Classical Encryption

```rust
use kanari_crypto::{encrypt_data, decrypt_data};

// AES-256-GCM with Argon2id
let encrypted = encrypt_data(data, password)?;
let decrypted = decrypt_data(&encrypted, password)?;
```

### Post-Quantum Key Encapsulation (Future)

```rust
use kanari_crypto::encryption::{EncryptionScheme};

// Kyber768 KEM - NIST Level 3 (Coming Soon)
// Kyber1024 KEM - NIST Level 5 (Coming Soon)

// Hybrid AES + Kyber (Coming Soon)
```

---

## 🔨 Usage Examples

### Example 1: Generate Quantum-Safe Keys

```rust
use kanari_crypto::{generate_keypair, CurveType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate Dilithium3 keypair (recommended)
    let keypair = generate_keypair(CurveType::Dilithium3)?;
    
    println!("Algorithm: {}", keypair.curve_type);
    println!("Address: {}", keypair.address);
    println!("Public Key: {}", keypair.public_key);
    println!("Private Key: {}", keypair.private_key);
    
    // Security level
    println!("Security Level: {}/5", keypair.curve_type.security_level());
    println!("Quantum-Resistant: {}", keypair.curve_type.is_post_quantum());
    
    Ok(())
}
```

### Example 2: Hybrid Signatures

```rust
use kanari_crypto::{generate_keypair, CurveType};

// Best practice: Use hybrid scheme
let keypair = generate_keypair(CurveType::Ed25519Dilithium3)?;

// Address format: 0xhybrid...
println!("Hybrid Address: {}", keypair.address);

// Private key contains both classical and PQC keys
// Format: kanahybrid<ed25519_key>:<dilithium_key>
```

### Example 3: Check Security Level

```rust
use kanari_crypto::CurveType;

let algorithms = vec![
    CurveType::Ed25519,
    CurveType::Dilithium3,
    CurveType::Ed25519Dilithium3,
];

for algo in algorithms {
    println!("{}: Level {}/5, Quantum-Safe: {}", 
        algo, 
        algo.security_level(),
        algo.is_post_quantum()
    );
}
```

**Output:**

```
Ed25519: Level 3/5, Quantum-Safe: false
Dilithium3 (PQC Level 3): Level 5/5, Quantum-Safe: true
Ed25519+Dilithium3 (Hybrid): Level 5/5, Quantum-Safe: true
```

---

## 📊 Algorithm Comparison

| Algorithm | Type | Signature Size | Speed | Quantum-Safe | NIST Level | Recommended |
|-----------|------|---------------|-------|--------------|------------|-------------|
| **Ed25519** | Classical | 64 bytes | ⚡ Very Fast | ❌ No | - | Legacy only |
| **K256** | Classical | ~70 bytes | ⚡ Fast | ❌ No | - | Legacy only |
| **Dilithium2** | PQC | ~2.5 KB | 🚀 Fast | ✅ Yes | 2 | IoT devices |
| **Dilithium3** | PQC | ~4 KB | 🚀 Fast | ✅ Yes | 3 | ⭐ **Recommended** |
| **Dilithium5** | PQC | ~5 KB | 🚀 Fast | ✅ Yes | 5 | High security |
| **SPHINCS+** | PQC | ~50 KB | 🐢 Slow | ✅ Yes | 5 | Ultra-secure |
| **Ed25519+Dilithium3** | Hybrid | ~4 KB | 🚀 Fast | ✅ Yes | 3+5 | ⭐ **Best practice** |
| **K256+Dilithium3** | Hybrid | ~4 KB | 🚀 Fast | ✅ Yes | 3+5 | Blockchain apps |

---

## 🛡️ Security Recommendations

### For New Applications (2025+)

```rust
// ⭐ Best: Use hybrid scheme
let keypair = generate_keypair(CurveType::Ed25519Dilithium3)?;

// ⭐ Good: Use pure PQC
let keypair = generate_keypair(CurveType::Dilithium3)?;
```

### For Existing Applications (Migration)

**Phase 1 (2025-2027): Add PQC Support**

```rust
// Keep existing Ed25519 keys for compatibility
let old_keypair = generate_keypair(CurveType::Ed25519)?;

// Generate new hybrid keys for new transactions
let new_keypair = generate_keypair(CurveType::Ed25519Dilithium3)?;
```

**Phase 2 (2028-2030): Transition to Hybrid**

```rust
// Make hybrid the default
let keypair = generate_keypair(CurveType::Ed25519Dilithium3)?;
```

**Phase 3 (2030+): PQC Only**

```rust
// When quantum computers are imminent
let keypair = generate_keypair(CurveType::Dilithium3)?;
```

### For Long-Term Secrets

```rust
// Maximum security for data that must remain secret for 30+ years
let keypair = generate_keypair(CurveType::Dilithium5)?;

// Or use SPHINCS+ for ultimate security
let keypair = generate_keypair(CurveType::SphincsPlusSha256Robust)?;
```

---

## 🔬 Hash Functions (Quantum-Resistant)

### Enhanced Hashing

```rust
use kanari_crypto::{hash_data, hash_data_sha3_512, hash_data_shake256};

// SHA3-256 (default, quantum-resistant)
let hash = hash_data(data); // 32 bytes

// SHA3-512 (higher security)
let hash = hash_data_sha3_512(data); // 64 bytes

// SHAKE256 (extendable output)
let hash = hash_data_shake256(data); // 32 bytes default

// SHAKE256 with custom length
let hash = hash_data_shake256_custom(data, 64); // 64 bytes
```

**Quantum Resistance:**

- SHA3-256: 128-bit quantum security (from 256-bit classical)
- SHA3-512: 256-bit quantum security (from 512-bit classical)
- SHAKE256: Configurable quantum security

---

## ⚙️ Configuration

### Enable Post-Quantum Features

In your `Cargo.toml`:

```toml
[dependencies]
kanari-crypto = { version = "2.0", features = ["pqc", "hybrid"] }
```

### Feature Flags

- `default` - Includes Blake3 and PQC
- `pqc` - Post-quantum cryptography algorithms
- `hybrid` - Hybrid classical + PQC schemes

---

## 🧪 Testing

```bash
# Run all tests including PQC
cargo test --all-features

# Test specific algorithm
cargo test dilithium

# Benchmark performance
cargo bench pqc_signatures
```

---

## 📈 Performance Considerations

### Key Generation Time

| Algorithm | Time (approx.) |
|-----------|----------------|
| Ed25519 | ~0.1 ms |
| Dilithium2 | ~0.2 ms |
| Dilithium3 | ~0.3 ms |
| Dilithium5 | ~0.5 ms |
| SPHINCS+ | ~50 ms |

### Signature Size Impact

- **Storage**: PQC signatures are 30-800x larger than Ed25519
- **Network**: Consider compression for transmission
- **Trade-off**: Size vs. quantum security

**Recommendation**: Use Dilithium3 for best balance of security and performance.

---

## 🔄 Migration Guide

### Step 1: Assess Current Usage

```rust
// Identify all cryptographic operations
// - Key generation
// - Signatures
// - Encryption
// - Hashing
```

### Step 2: Add PQC Support

```rust
// Add dependency
kanari-crypto = { version = "2.0", features = ["pqc", "hybrid"] }

// Update imports
use kanari_crypto::CurveType;
```

### Step 3: Implement Hybrid Scheme

```rust
// Generate hybrid keys alongside existing keys
let classical_key = generate_keypair(CurveType::Ed25519)?;
let hybrid_key = generate_keypair(CurveType::Ed25519Dilithium3)?;

// Store both, use hybrid for new operations
wallet.add_key("legacy", classical_key);
wallet.add_key("quantum_safe", hybrid_key);
wallet.set_default("quantum_safe");
```

### Step 4: Update Storage

```rust
// PQC keys are larger - update database schema
// - Public key: up to 2KB
// - Private key: up to 5KB
// - Signature: up to 50KB (SPHINCS+)
```

### Step 5: Test Thoroughly

```bash
cargo test
cargo test --features pqc
```

---

## 🎓 Learning Resources

### NIST PQC Standards

- [NIST PQC Project](https://csrc.nist.gov/projects/post-quantum-cryptography)
- [ML-DSA (Dilithium) Specification](https://csrc.nist.gov/pubs/fips/204/final)
- [ML-KEM (Kyber) Specification](https://csrc.nist.gov/pubs/fips/203/final)

### Academic Papers

- "CRYSTALS-Dilithium: A Lattice-Based Digital Signature Scheme"
- "SPHINCS+: Stateless Hash-Based Signatures"
- "CRYSTALS-Kyber: Key Encapsulation Mechanism"

### Quantum Computing Timeline

- [Quantum Threat Timeline](https://globalriskinstitute.org/publications/quantum-threat-timeline-report/)

---

## ⚠️ Important Notes

### Limitations

1. **No BIP39 Support for PQC**: Post-quantum algorithms don't support mnemonic phrase derivation yet. Use direct key generation.

2. **Larger Key Sizes**: PQC keys and signatures are significantly larger than classical ones.

3. **Compatibility**: PQC signatures are not backward compatible with classical verifiers.

4. **Standards Evolution**: NIST standards are final as of 2024, but implementations may evolve.

### Security Considerations

- **Use Hybrid Schemes**: During transition period (2025-2030)
- **Key Rotation**: Rotate to PQC before quantum computers become practical
- **Store Securely**: PQC private keys are larger and must be protected
- **Plan Ahead**: Implement PQC now for data that must remain secret 10+ years

---

## 📞 Support

For questions or issues:

- GitHub Issues: [kanari-cp/issues](https://github.com/jamesatomc/kanari-cp/issues)
- Security: <security@kanari.network>

---

## 📜 License

Same as Kanari project license.

---

## 🎉 Summary

**Kanari Crypto v2.0** provides **maximum security** against both classical and quantum computer attacks:

✅ NIST-standardized post-quantum algorithms  
✅ Hybrid schemes for smooth transition  
✅ Production-ready implementations  
✅ Comprehensive documentation  
✅ Future-proof security architecture  

**Recommendation**: Start using `CurveType::Ed25519Dilithium3` today for quantum-safe security! 🚀
