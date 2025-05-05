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
- Secure memory handling with automatic clearing of sensitive data
- Input validation and robust error handling
- Password strength validation

## Usage Examples

### Key Generation

```rust
use mona_crypto::{generate_keypair, CurveType};

// Generate a new keypair
let keypair = generate_keypair(CurveType::K256).expect("Failed to generate keypair");
println!("Address: {}", keypair.address);
// NEVER print or log private keys in production code
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

// Always clear sensitive data after use
use mona_crypto::secure_clear;
let mut private_key_bytes = keypair.private_key.clone().into_bytes();
secure_clear(&mut private_key_bytes);
```

### Wallet Operations

```rust
use mona_crypto::{save_wallet, load_wallet, check_wallet_exists, is_password_strong, CurveType};
use mona_types::address::Address;

// Check password strength
let password = "SecureP@ssw0rd123";
if !is_password_strong(password) {
    eprintln!("Password not strong enough!");
    return;
}

// Create and save a wallet
let address = Address::from_str(&keypair.address).unwrap();
save_wallet(
    &address,
    &keypair.private_key,
    "seed phrase here",
    password,
    CurveType::K256
).expect("Failed to save wallet");

// Load the wallet
let wallet = load_wallet(&address.to_string(), password)
    .expect("Failed to load wallet");

// Use wallet for signing (with password validation)
let signature = wallet.sign(message, password)
    .expect("Failed to sign message");
```

## Security Best Practices

1. **Never hardcode passwords** in your application
2. **Always use strong, unique passwords** for wallet encryption (use `is_password_strong` function)
3. **Never print or log private keys** - they should never be exposed
4. **Clear sensitive data from memory** after use with `secure_clear` or `secure_erase`
5. **Verify signatures with correct curve type** when possible
6. **Always validate user input** before processing cryptographic operations
7. **Use password-based key derivation** for all encryption operations
8. **Implement proper error handling** without revealing sensitive details in error messages

## License

[MIT License](LICENSE)
