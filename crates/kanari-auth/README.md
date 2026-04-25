# Kanari Auth - Email & Password Authentication with SQLite

## Overview

This crate provides email and password-based authentication for the Kanari SDK with **persistent SQLite storage**, enabling users to sign transactions using familiar credentials.

## Features

✅ **SQLite Persistence** - User data stored in SQLite database  
✅ **Secure Authentication** - Argon2id password hashing (OWASP standard)  
✅ **Account Security** - Lockout after 5 failed attempts (15-minute cooldown)  
✅ **Session Management** - Configurable timeouts with automatic expiration  
✅ **Password Strength** - Enforced requirements (8+ chars, mixed case, digits, special chars)  
✅ **Transaction Signing** - Seamless integration with Kanari wallet infrastructure  
✅ **Multi-Curve Support** - Ed25519, Secp256k1, BLS12-381  

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
kanari-auth = { path = "../crates/kanari-auth" }
```

## Quick Start

### In-Memory Mode (Development/Testing)

```rust
use kanari_auth::AuthManager;

// Create auth manager with in-memory SQLite database
let mut auth = AuthManager::new();

// Register a new user
let wallet = auth.register_user("alice@example.com", "SecurePass123!", None)?;
println!("Wallet: {}", wallet.address);

// Login
let session = auth.login("alice@example.com", "SecurePass123!", None)?;
println!("Session: {}", session.session_id);

// Sign transaction
let signed_tx = auth.sign_transfer(
    &session,
    "0xRecipient...",
    1_000_000,  // amount in mist
    Some(100_000),  // gas limit
    Some(1_000),    // gas price
)?;

// Logout
auth.logout(&session.session_id)?;
```

### Persistent Storage Mode (Production)

```rust
use kanari_auth::AuthManager;
use std::path::PathBuf;

// Create auth manager with persistent SQLite database
let db_path = PathBuf::from("data/auth.db");
let mut auth = AuthManager::with_persistence(db_path)?;

// All operations now persist across restarts
auth.register_user("alice@example.com", "SecurePass123!", None)?;
// ... application restarts ...
// Data is still available!
let session = auth.login("alice@example.com", "SecurePass123!", None)?;
```

## API Reference

### AuthManager

#### Construction

- `AuthManager::new()` - Creates instance with in-memory SQLite database
- `AuthManager::with_persistence(db_path)` - Creates instance with persistent SQLite file

#### User Management

- `register_user(email, password, curve_type)` - Register new user with wallet
- `login(email, password, session_timeout)` - Authenticate and create session
- `logout(session_id)` - Invalidate specific session
- `logout_all(email)` - Invalidate all sessions for a user
- `change_password(email, old_password, new_password)` - Update password (invalidates all sessions)
- `delete_account(email, password)` - Delete user account
- `list_users()` - List all registered user emails
- `user_count()` - Get total number of users

#### Transaction Operations

- `sign_transaction(session, transaction)` - Sign any transaction type
- `sign_transfer(session, recipient, amount, gas_limit, gas_price)` - Convenience method for transfers

#### Session Management

- `get_user_info(session)` - Retrieve user details from valid session
- `is_session_valid(session_id)` - Check if session is still active

## Database Schema

The SQLite database uses the following schema:

```sql
CREATE TABLE users (
    email TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    encrypted_private_key TEXT,
    created_at TEXT NOT NULL,
    last_login TEXT,
    failed_attempts INTEGER NOT NULL DEFAULT 0,
    locked_until TEXT,
    is_active BOOLEAN NOT NULL DEFAULT 1
);

CREATE INDEX idx_users_active ON users(is_active);
```

## Security Features

### Password Requirements

- Minimum 8 characters
- At least one uppercase letter
- At least one lowercase letter
- At least one digit
- At least one special character (!@#$%^&* etc.)

### Account Lockout

- After 5 consecutive failed login attempts
- 15-minute lockout duration
- Automatic unlock after timeout

### Session Security

- UUID v4 session IDs (cryptographically random)
- Configurable timeout (default: 24 hours)
- Automatic invalidation on password change
- Time-based expiration checking

### Password Hashing

- Algorithm: Argon2id (OWASP recommended)
- Memory-hard function resistant to GPU attacks
- Unique salt per password

## Testing

Run unit tests:

```bash
cargo test -p kanari-auth
```

Run example:

```bash
cargo run -p kanari-auth --example email_auth_example
```

## Error Handling

All operations return `AuthResult<T>` which is `Result<T, AuthError>`:

```rust
pub enum AuthError {
    InvalidEmail(String),
    InvalidPassword(String),
    UserNotFound(String),
    UserAlreadyExists(String),
    AuthenticationFailed,
    SessionExpired,
    InvalidSession,
    WalletError(WalletError),
    SigningError(String),
    SerializationError(String),
    IoError(std::io::Error),
    ValidationError(String),
    RateLimitExceeded,
    AccountLocked,
    CryptoError(String),
    DatabaseError(String),  // SQLite errors
}
```

## Migration from In-Memory to SQLite

If you were using the previous in-memory version:

**Before:**

```rust
let mut auth = AuthManager::new();  // HashMap-based
```

**After:**

```rust
let mut auth = AuthManager::new();  // Now uses in-memory SQLite
// OR for persistence:
let mut auth = AuthManager::with_persistence(PathBuf::from("auth.db"))?;
```

The API remains the same - just the storage backend changed!

## Performance Considerations

- **In-Memory SQLite**: ~microsecond query times, no I/O overhead
- **Persistent SQLite**: ~millisecond query times (with WAL mode enabled by default)
- **Connection Pooling**: Not implemented yet (single connection per AuthManager)
- **Indexing**: Active user index for faster lookups

## Future Enhancements

- [ ] Connection pooling for high-concurrency scenarios
- [ ] Encrypted private key storage (currently plaintext)
- [ ] Email verification workflow
- [ ] Two-factor authentication (2FA)
- [ ] OAuth2 integration
- [ ] Database migration support
- [ ] Backup/restore functionality
- [ ] Read replicas for scaling

## License

Apache-2.0

## Contributing

Contributions welcome! Please read our contributing guidelines and submit pull requests.
