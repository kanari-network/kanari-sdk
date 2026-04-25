# Kanari Auth - Email & Password Transaction Signing System

## Overview

The `kanari-auth` crate provides a secure email and password-based authentication system for signing transactions in the Kanari SDK. It integrates with the existing wallet infrastructure to enable users to authenticate and sign blockchain transactions using familiar credentials.

## Features

- ✅ **Email-based Registration**: Register new accounts with email and password
- ✅ **Secure Password Hashing**: Uses Argon2id for industry-standard password security
- ✅ **Session Management**: Time-limited sessions with automatic expiration
- ✅ **Account Lockout Protection**: Prevents brute-force attacks with automatic lockout after failed attempts
- ✅ **Transaction Signing**: Sign Move transactions using authenticated sessions
- ✅ **Email Validation**: RFC-compliant email format validation
- ✅ **Password Strength Requirements**: Enforces strong password policies
- ✅ **In-memory User Store**: Thread-safe user data management
- ✅ **Logging Integration**: Comprehensive audit trail using tracing

## Architecture

```
┌─────────────────────────────────────────┐
│         AuthManager (Public API)        │
│  - register()                           │
│  - login()                              │
│  - logout()                             │
│  - sign_transaction()                   │
│  - change_password()                    │
└──────────────┬──────────────────────────┘
               │
    ┌──────────┴──────────┐
    │                     │
┌───▼──────┐      ┌──────▼────────┐
│UserStore │      │SessionManager │
│          │      │               │
│- Users   │      │- Sessions     │
│- Wallets │      │- Expiry       │
│- Hashes  │      │- Validation   │
└──────────┘      └───────────────┘
```

## Quick Start

### 1. Add Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
kanari-auth = { path = "../crates/kanari-auth" }
```

### 2. Basic Usage

```rust
use kanari_auth::AuthManager;
use kanari_types::transaction::Transaction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize auth manager
    let mut auth = AuthManager::new();
    
    // Register a new user
    auth.register(
        "alice@example.com",
        "SecurePass123!",
        None, // Use default curve (Ed25519)
    )?;
    
    // Login with credentials
    let session = auth.login(
        "alice@example.com",
        "SecurePass123!",
        None, // Use default session timeout (24 hours)
    )?;
    
    println!("Logged in! Session ID: {}", session.session_id);
    println!("Wallet Address: {}", session.wallet_address);
    
    // Create a transaction (example)
    let tx = Transaction::Transfer { /* ... */ };
    
    // Sign transaction using session
    let signed_tx = auth.sign_transaction(&session, tx)?;
    
    println!("Transaction signed! Hash: {:?}", signed_tx.transaction.hash());
    
    // Logout when done
    auth.logout(&session.session_id)?;
    
    Ok(())
}
```

### 3. Advanced Configuration

```rust
use std::time::Duration;
use kanari_crypto::keys::CurveType;

// Custom session timeout (1 hour)
let timeout = Duration::from_secs(3600);
let session = auth.login("user@example.com", "password", Some(timeout))?;

// Use Secp256k1 curve instead of Ed25519
auth.register(
    "user@example.com",
    "password",
    Some(CurveType::Secp256k1),
)?;
```

## Security Features

### Password Requirements

Passwords must meet the following criteria:

- Minimum 8 characters
- Maximum 128 characters
- At least one uppercase letter
- At least one lowercase letter
- At least one digit
- At least one special character (!@#$%^&* etc.)

### Account Lockout

After 5 consecutive failed login attempts:

- Account is locked for 15 minutes
- Further login attempts are rejected until lockout expires
- Successful login resets the failed attempt counter

### Session Security

- Sessions have configurable timeouts (default: 24 hours)
- Expired sessions are automatically invalidated
- Each session is tied to a specific user and wallet
- Session IDs are UUID v4 (cryptographically random)

### Password Storage

- Passwords are hashed using **Argon2id** (OWASP recommended)
- Random salt generated per user (16 bytes)
- Default parameters:
  - Memory: 4096 KB
  - Iterations: 3
  - Parallelism: 1
- Original passwords are never stored

## API Reference

### AuthManager

#### `new() -> Self`

Create a new authentication manager instance.

#### `register(email: &str, password: &str, curve_type: Option<CurveType>) -> AuthResult<String>`

Register a new user account.

**Returns**: Wallet address hex string

**Example**:

```rust
let address = auth.register("user@example.com", "StrongPass1!", None)?;
println!("Wallet created: {}", address);
```

#### `login(email: &str, password: &str, session_timeout: Option<Duration>) -> AuthResult<Session>`

Authenticate user and create a session.

**Returns**: Valid session object

**Errors**:

- `AuthenticationFailed`: Invalid credentials
- `AccountLocked`: Too many failed attempts
- `UserNotFound`: Email not registered

#### `logout(session_id: &str) -> AuthResult<()>`

Terminate an active session.

#### `sign_transaction(session: &Session, transaction: Transaction) -> AuthResult<SignedTransaction>`

Sign a transaction using the authenticated session's wallet.

**Requirements**:

- Session must be valid and not expired
- User must have a wallet associated

#### `change_password(email: &str, old_password: &str, new_password: &str) -> AuthResult<()>`

Change user's password with verification.

**Security**: Requires current password for authorization

#### `get_user_info(email: &str) -> AuthResult<UserInfo>`

Retrieve non-sensitive user information.

**Returns**: Email, wallet address, creation date, login count

#### `is_session_valid(session_id: &str) -> bool`

Check if a session is still valid.

### Session

```rust
pub struct Session {
    pub session_id: String,        // UUID v4
    pub email: String,             // Normalized email
    pub wallet_address: String,    // Hex address
    pub created_at: DateTime<Utc>, // Creation timestamp
    pub expires_at: DateTime<Utc>, // Expiration timestamp
}

impl Session {
    pub fn is_expired(&self) -> bool;
    pub fn time_remaining(&self) -> Duration;
}
```

### UserInfo

```rust
pub struct UserInfo {
    pub email: String,
    pub wallet_address: String,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub login_count: u64,
}
```

## Error Handling

All operations return `AuthResult<T>` which is `Result<T, AuthError>`.

Common errors:

- `InvalidEmail`: Email format validation failed
- `WeakPassword`: Password doesn't meet requirements
- `UserAlreadyExists`: Email already registered
- `UserNotFound`: No account with given email
- `AuthenticationFailed`: Invalid password
- `AccountLocked`: Temporary lockout due to failed attempts
- `SessionExpired`: Session timeout exceeded
- `SigningError`: Transaction signing failed

**Example**:

```rust
match auth.login("user@example.com", "wrong") {
    Ok(session) => println!("Success!"),
    Err(AuthError::AuthenticationFailed) => eprintln!("Wrong password"),
    Err(AuthError::AccountLocked) => eprintln!("Account locked, try again later"),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Examples

See `examples/email_auth_example.rs` for complete working examples including:

- User registration flow
- Login/logout cycle
- Transaction signing
- Password change
- Session validation
- Error handling patterns

Run example:

```bash
cargo run --example email_auth_example
```

## Testing

Run tests:

```bash
cargo test -p kanari-auth
```

## Production Considerations

### ⚠️ Current Limitations

This implementation is designed for development and testing. For production deployment, consider:

1. **Persistent Storage**: Replace in-memory `UserStore` with database (PostgreSQL, etc.)
2. **Encryption at Rest**: Encrypt private keys with AES-256-GCM before storage
3. **Rate Limiting**: Implement IP-based rate limiting for login attempts
4. **Email Verification**: Add email confirmation flow with tokens
5. **Two-Factor Authentication**: Support TOTP or SMS-based 2FA
6. **Audit Logging**: Store authentication events in immutable log
7. **Key Rotation**: Implement periodic key rotation mechanisms
8. **Backup & Recovery**: Secure backup procedures for user wallets

### Recommended Enhancements

```rust
// Example: Add email verification
auth.send_verification_email("user@example.com")?;
auth.verify_email_token("user@example.com", "token123")?;

// Example: Enable 2FA
auth.enable_2fa("user@example.com", "totp_secret")?;
auth.login_with_2fa("user@example.com", "password", "123456")?;
```

## Integration with Kanari SDK

The `kanari-auth` crate seamlessly integrates with other Kanari components:

```rust
use kanari_auth::AuthManager;
use kanari_rpc_client::RpcClient;

// Authenticate user
let mut auth = AuthManager::new();
let session = auth.login("user@example.com", "password", None)?;

// Create RPC client
let client = RpcClient::new("http://localhost:19002")?;

// Sign and submit transaction
let tx = create_transfer_transaction();
let signed_tx = auth.sign_transaction(&session, tx)?;
client.submit_transaction(&signed_tx).await?;
```

## License

Same as Kanari SDK workspace license.

## Contributing

Contributions welcome! Please ensure:

- All tests pass: `cargo test`
- Code follows Rust conventions
- Security implications reviewed
- Documentation updated

---

**Version**: 0.1.5  
**Author**: Kanari Team  
**Last Updated**: 2026-04-25
