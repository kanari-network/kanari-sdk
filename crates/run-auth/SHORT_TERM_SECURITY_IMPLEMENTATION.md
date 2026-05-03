# Short-term Security Improvements Implementation Guide

## Overview

This document describes the implementation of three critical short-term security improvements for the Kanari Auth API:

1. **Rate Limiting** - Prevent brute force attacks and API abuse
2. **Two-Factor Authentication (TOTP)** - Enhanced account security
3. **Audit Logging** - Comprehensive security event tracking

---

## 1. Rate Limiting Implementation ✅

### Architecture

- **Custom Token Bucket Algorithm**: Lightweight, no external dependencies
- **Per-IP Tracking**: Each IP address has its own rate limiter
- **Thread-Safe**: Uses `tokio::sync::Mutex` with `HashMap`
- **Configurable**: Three preset profiles (strict/moderate/relaxed)

### Configuration Profiles

```rust
// Strict: For login/register endpoints (10 req/min)
let config = RateLimitConfig::strict();

// Moderate: For general endpoints (60 req/min)
let config = RateLimitConfig::moderate();

// Relaxed: For read-only endpoints (120 req/min)
let config = RateLimitConfig::relaxed();
```

### Usage in Handlers

```rust
use crate::rate_limiter;

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> (StatusCode, Json<ApiResponse<LoginResponse>>) {
    // Extract client IP from request
    let ip = extract_client_ip(&req).await?;
    
    // Check rate limit before processing
    if let Err(_) = state.rate_limiter.check_rate_limit(ip).await {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiResponse::error("Rate limit exceeded"))
        );
    }
    
    // ... proceed with login logic
}
```

### Key Features

- ✅ Automatic token refill based on time interval
- ✅ No external dependencies (removed governor/dashmap)
- ✅ Memory-efficient: Only stores active IPs
- ✅ Configurable limits per endpoint type
- ✅ Returns HTTP 429 with Retry-After header

### Files Modified

- `crates/run-auth/src/rate_limiter.rs` - Core implementation
- `crates/run-auth/src/main.rs` - Integration into AppState
- `crates/run-auth/Cargo.toml` - Removed unused dependencies

---

## 2. Two-Factor Authentication (TOTP) ✅

### Architecture

- **TOTP Standard**: RFC 6238 compliant (Time-based One-Time Password)
- **QR Code Generation**: SVG format for easy display
- **Backup Codes**: 10 one-time recovery codes per user
- **Base32 Encoding**: Industry standard for secret storage

### Setup Flow

```
1. User requests 2FA setup → POST /api/v1/2fa/setup
   Request: { "email": "...", "password": "..." }
   
2. Server generates TOTP secret + QR code
   
3. Response includes:
   - Secret (base32 encoded)
   - OTPAuth URL (for QR code)
   - 10 backup codes
   
4. User scans QR code with authenticator app
   (Google Authenticator, Authy, etc.)
   
5. User verifies setup → POST /api/v1/2fa/enable
   Request: { "email": "...", "password": "...", "code": "123456" }
   
6. Server validates code and enables 2FA
```

### API Endpoints

#### Setup 2FA

```bash
POST /api/v1/2fa/setup
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "StrongPassword123!"
}

Response:
{
  "success": true,
  "data": {
    "secret": "JBSWY3DPEHPK3PXP",
    "otpauthUrl": "otpauth://totp/Kanari%20Auth:user@example.com?secret=...",
    "backupCodes": ["A1B2C3D4", "E5F6G7H8", ...],
    "message": "Scan the QR code with your authenticator app"
  }
}
```

#### Enable 2FA

```bash
POST /api/v1/2fa/enable
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "StrongPassword123!",
  "code": "123456"
}

Response:
{
  "success": true,
  "message": "2FA enabled successfully"
}
```

#### Disable 2FA

```bash
POST /api/v1/2fa/disable
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "StrongPassword123!"
}
```

#### Verify TOTP Code

```bash
POST /api/v1/2fa/verify
Content-Type: application/json

{
  "email": "user@example.com",
  "code": "123456"
}
```

### Implementation Details

```rust
// Generate random 20-byte secret
let secret_bytes: Vec<u8> = (0..20).map(|_| rand::random::<u8>()).collect();

// Create TOTP instance
let totp = TOTP::new(
    Algorithm::SHA1,
    6,              // 6 digits
    1,              // 1 step skew
    30,             // 30 second interval
    secret_bytes.clone(),
    Some("Kanari Auth".to_string()),
    email.to_string(),
)?;

// Generate backup codes
let backup_codes = (0..10)
    .map(|_| generate_random_code(8))
    .collect();
```

### Database Schema Requirements

2FA enrollment state is now persisted in SQLite through `kanari-auth`, so secrets and backup codes survive API restarts. The `users` table includes these fields:

```sql
ALTER TABLE users ADD COLUMN totp_secret TEXT NULL;
ALTER TABLE users ADD COLUMN totp_enabled BOOLEAN DEFAULT FALSE;
ALTER TABLE users ADD COLUMN backup_codes TEXT NULL; -- JSON array
```

### Files Created

- `crates/run-auth/src/two_factor.rs` - TOTP manager implementation
- Added routes in `crates/run-auth/src/main.rs`
- Added models in `crates/run-auth/src/models.rs`

### Dependencies Added

```toml
totp-rs = { version = "5.4", features = ["gen_secret", "otpauth"] }
qrcode = "0.14"
data-encoding = "2.4"
rand = "0.8"
```

---

## 3. Audit Logging ✅

### Architecture

- **Structured JSON Logs**: Machine-parseable format
- **File-Based Storage**: Separate from application logs
- **Severity Levels**: Info, Warning, Error, Critical
- **Event Types**: Categorized security events
- **IP Tracking**: Captures client IP for all events

### Event Types

```rust
pub enum AuditEventType {
    LoginSuccess,
    LoginFailure,
    Registration,
    Logout,
    PasswordChange,
    AccountDeletion,
    SessionValidation,
    TransactionSigning,
    EncryptedKeyAccess,
    SuspiciousActivity,
    RateLimitExceeded,
    TwoFactorSetup,
    TwoFactorVerification,
}
```

### Severity Levels

```rust
pub enum AuditSeverity {
    Info,      // Normal operations
    Warning,   // Potential issues
    Error,     // Failed operations
    Critical,  // Security breaches
}
```

### Usage in Handlers

```rust
// Log successful login
state.audit_logger.log_success(
    AuditEventType::LoginSuccess,
    Some(email.clone()),
    Some(client_ip),
    Some(session_id.clone()),
    serde_json::json!({"method": "password"})
).await;

// Log failed login attempt
state.audit_logger.log_failure(
    AuditEventType::LoginFailure,
    AuditSeverity::Warning,
    Some(email.clone()),
    Some(client_ip),
    None,
    serde_json::json!({"reason": "invalid_password"}),
    "Invalid password provided"
).await;
```

### Log File Format

Each log entry is a JSON object:

```json
{
  "timestamp": "2026-05-02T19:30:45.123Z",
  "event_type": "LoginFailure",
  "severity": "Warning",
  "email": "user@example.com",
  "ip_address": "192.168.1.100",
  "session_id": null,
  "metadata": {
    "reason": "invalid_password",
    "attempt_count": 3
  },
  "error_message": "Invalid password provided"
}
```

### Configuration

```rust
// Default: logs stored in current directory
let audit_logger = AuditLogger::new(None);

// Custom directory
let audit_logger = AuditLogger::new(Some(PathBuf::from("/var/log/kanari-audit")));

// Environment variable override
// AUDIT_LOG_DIR=/var/log/kanari-audit
```

### Log Rotation (Future Enhancement)

Consider implementing:

- Daily log rotation
- Maximum file size limits
- Compression of old logs
- Automated cleanup after N days

### Files Created

- `crates/run-auth/src/audit_logger.rs` - Complete implementation
- Integrated into `AppState` in `main.rs`

---

## Integration Summary

### AppState Structure

```rust
#[derive(Clone)]
pub struct AppState {
    pub auth_manager: Arc<Mutex<AuthManager>>,
    pub audit_logger: AuditLogger,        // NEW
    pub rate_limiter: RateLimiter,         // NEW
    pub totp_manager: TotpManager,         // NEW
}
```

### Initialization in main.rs

```rust
// Initialize audit logger
let audit_log_dir = std::env::var("AUDIT_LOG_DIR")
    .ok()
    .map(PathBuf::from);
let audit_logger = AuditLogger::new(audit_log_dir);

// Initialize rate limiter (strict for auth endpoints)
let rate_limiter = RateLimiter::new(RateLimitConfig::strict());

// Initialize TOTP manager
let totp_manager = TotpManager::new(None);

let state = AppState {
    auth_manager: Arc::new(Mutex::new(auth_manager)),
    audit_logger,
    rate_limiter,
    totp_manager,
};
```

---

## Testing Recommendations

### 1. Rate Limiting Tests

```bash
# Test strict rate limit (10 req/min)
for i in {1..15}; do
  curl -X POST http://localhost:3000/api/v1/login \
    -H "Content-Type: application/json" \
    -d '{"email":"test@example.com","password":"wrong"}'
done

# Expected: First 10 succeed, last 5 return 429
```

### 2. 2FA Setup Tests

```bash
# Generate QR code
curl -X POST http://localhost:3000/api/v1/2fa/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "TestPassword123!"
  }'

# Verify TOTP code (after scanning QR)
curl -X POST http://localhost:3000/api/v1/2fa/verify \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "code": "123456"
  }'
```

### 3. Audit Log Verification

```bash
# Check audit log exists
ls -la audit_*.log

# View recent entries
tail -n 20 audit_$(date +%Y-%m-%d).log | jq .

# Search for failed logins
grep "LoginFailure" audit_*.log | jq '.email'
```

---

## Production Deployment Checklist

### Pre-Deployment

- [ ] Set `AUDIT_LOG_DIR` environment variable
- [ ] Configure log rotation policy
- [ ] Test rate limiting thresholds with load testing
- [ ] Verify 2FA QR code generation works on mobile devices
- [ ] Backup existing database before schema changes

### Post-Deployment

- [ ] Monitor audit log file size growth
- [ ] Review rate limit violation patterns
- [ ] Collect user feedback on 2FA setup flow
- [ ] Set up alerts for Critical severity events
- [ ] Document incident response procedures

### Monitoring Metrics

- Rate limit hit rate (should be < 5% for legitimate users)
- 2FA adoption rate (% of users enabling 2FA)
- Failed login attempts per hour
- Audit log write latency

---

## Known Limitations & Future Work

### Current Limitations

1. **2FA State Is Single-Database**: Secrets and backup codes persist across restart, but are still local to the configured SQLite database file
2. **Rate Limiter In-Memory**: Resets on server restart (consider Redis for persistence)
3. **No Distributed Rate Limiting**: Each server instance tracks independently
4. **Audit Logs Local Only**: No centralized log aggregation

### Recommended Enhancements

#### Phase 2 (Next 1-2 Months)

- [ ] Implement distributed rate limiting with Redis
- [ ] Add SMS-based 2FA as alternative to TOTP
- [ ] Integrate with SIEM system (Splunk, ELK)
- [ ] Add geolocation tracking for login attempts
- [ ] Implement device fingerprinting

#### Phase 3 (3-6 Months)

- [ ] Hardware Security Module (HSM) integration
- [ ] Multi-signature support for high-value transactions
- [ ] Behavioral analysis for anomaly detection
- [ ] OAuth 2.0 / OIDC integration
- [ ] WebAuthn/FIDO2 support for passwordless login

---

## Security Best Practices

### Rate Limiting

- ✅ Use strict limits on authentication endpoints
- ✅ Return generic error messages (don't reveal if email exists)
- ✅ Log rate limit violations for security analysis
- ⚠️ Consider CAPTCHA after multiple failures

### 2FA

- ✅ Store TOTP secrets encrypted at rest
- ✅ Provide backup codes for account recovery
- ✅ Force re-authentication before enabling/disabling 2FA
- ⚠️ Never display secret in plain text (always use QR code)

### Audit Logging

- ✅ Log all authentication events (success/failure)
- ✅ Include IP addresses and timestamps
- ✅ Use structured JSON for easy parsing
- ⚠️ Ensure logs don't contain passwords or private keys
- ⚠️ Implement log integrity verification (prevent tampering)

---

## Troubleshooting

### Rate Limiting Issues

**Problem**: Legitimate users getting rate limited

```bash
# Solution: Increase limits or whitelist IPs
let rate_limiter = RateLimiter::new(RateLimitConfig::moderate());
```

**Problem**: Rate limiter not working

```bash
# Check: Ensure ConnectInfo middleware is enabled
# Verify: Check logs for "Rate limit exceeded" messages
```

### 2FA Issues

**Problem**: QR code not scanning

```bash
# Verify: OTPAuth URL format is correct
# Check: Secret is properly base32 encoded
# Test: Manually verify code generation matches expected values
```

**Problem**: TOTP verification fails

```bash
# Check: Time synchronization between server and client
# Verify: Base32 decoding is correct
# Debug: Log generated vs provided codes (in dev only!)
```

### Audit Logging Issues

**Problem**: Logs not being written

```bash
# Check: Directory permissions (must be writable)
# Verify: AUDIT_LOG_DIR environment variable is set
# Test: Write test entry manually
```

**Problem**: Log files growing too large

```bash
# Solution: Implement log rotation
# Alternative: Ship logs to centralized system (ELK, Splunk)
```

---

## Conclusion

All three short-term security improvements have been successfully implemented:

✅ **Rate Limiting**: Protects against brute force attacks  
✅ **Two-Factor Authentication**: Adds strong account security layer  
✅ **Audit Logging**: Provides comprehensive security event tracking  

The system is now significantly more secure and production-ready. Next steps include completing the 2FA database integration and implementing distributed rate limiting for multi-server deployments.

For questions or issues, refer to the troubleshooting section or contact the security team.
