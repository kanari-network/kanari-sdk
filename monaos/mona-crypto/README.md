# Mona Crypto Library

A secure cryptographic library for the Mona blockchain platform, providing key management, digital signatures, and wallet functionality.

## Features

- Multiple curve support: K256 (secp256k1), P256 (secp256r1), and Ed25519
- Secure wallet generation, storage, and management
- Digital signatures and verification
- Encryption and decryption with AES-256-GCM
- BIP39 mnemonic phrase support

## Security Features

- Constant-time comparisons for signature verification to prevent timing attacks
- Argon2id password hashing for key derivation
- AES-256-GCM authenticated encryption
- Secure memory handling with zero-out functionality
- Input validation and robust error handling

## Usage Examples

### Key Generation

```rust
use mona_crypto::{generate_keypair, CurveType};

// Generate a new keypair
let keypair = generate_keypair(CurveType::K256).expect("Failed to generate keypair");
println!("Address: {}", keypair.address);
println!("Private key: {}", keypair.private_key);
```

### Signing and Verification

```rust
use mona_crypto::{sign_message, verify_signature, CurveType};

// Sign a message
let message = b"Hello, world!";
let signature = sign_message(&keypair.private_key, message, CurveType::K256)
    .expect("Failed to sign message");
    
// Verify the signature
let is_valid = verify_signature(&keypair.address, message, &signature)
    .expect("Failed to verify signature");
assert!(is_valid);
```

### Wallet Operations

```rust
use mona_crypto::{save_wallet, load_wallet, check_wallet_exists, CurveType};
use mona_types::address::Address;

// Create and save a wallet
let address = Address::from_str(&keypair.address).unwrap();
save_wallet(
    &address,
    &keypair.private_key,
    "seed phrase here",
    "secure_password",
    CurveType::K256
).expect("Failed to save wallet");

// Load the wallet
let wallet = load_wallet(&address.to_string(), "secure_password")
    .expect("Failed to load wallet");
```

## Security Best Practices

1. **Never hardcode passwords** in your application
2. **Always use strong, unique passwords** for wallet encryption
3. **Store private keys securely** - they should never be exposed to users
4. **Verify signatures with the correct curve type** when possible
5. **Clear sensitive data from memory** after use with `secure_clear`
6. **Use password-based key derivation** for all encryption operations
7. **Validate all inputs** before processing cryptographic operations

## License

[MIT License](LICENSE)
