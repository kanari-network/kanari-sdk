# Kanari Crypto

Secure, modern cryptography for the Kanari blockchain. This crate provides:

- Key generation and management (K256, P256, Ed25519, Dilithium, SPHINCS+)
- Digital signatures (ECC, PQC, and hybrid schemes)
- AES‑256‑GCM encryption with Argon2id key derivation
- Wallet/HD wallet, keystore, backup, and audit logging

## Quick Start

```rust
use kanari_crypto::{sign_message, verify_signature_with_curve};
use kanari_crypto::keys::{CurveType, generate_keypair};

let kp = generate_keypair(CurveType::K256).unwrap();
let msg = b"hello";
let sig = sign_message(&kp.private_key, msg, CurveType::K256).unwrap();
let ok = verify_signature_with_curve(&kp.address, msg, &sig, CurveType::K256).unwrap();
assert!(ok);
```

```rust
use kanari_crypto::{encrypt_string, decrypt_string};
let enc = encrypt_string("secret", "StrongPassw0rd!").unwrap();
let dec = decrypt_string(&enc, "StrongPassw0rd!").unwrap();
assert_eq!(dec, "secret");
```

## Security

- Zeroize for secret memory clearing
- Argon2id for password‑based key derivation
- Post‑quantum ready with hybrid signatures

## License

Apache‑2.0
