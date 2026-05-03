# Migration Guide - Security Updates v0.1.6

**Effective Date:** 2026-05-02  
**Version:** run-auth v0.1.6  
**Impact:** Breaking changes to encrypted key retrieval endpoint

---

## Overview

This release addresses critical security vulnerabilities in the authentication system. All users must update their client code to accommodate the breaking changes.

---

## Breaking Changes

### 1. Encrypted Key Retrieval Endpoint Changed

**Endpoint:** `/api/v1/user/encrypted-key`

#### Before (INSECURE - REMOVED)
```http
GET /api/v1/user/encrypted-key?email=user@example.com
```

#### After (SECURE - REQUIRED)
```http
POST /api/v1/user/encrypted-key
Content-Type: application/json

{
  "email": "user@example.com",
  "sessionId": "your-valid-session-id"
}
```

**Why This Change?**
The previous endpoint allowed anyone to download encrypted private keys by only knowing an email address. The new implementation requires a valid session, ensuring only authenticated users can access their own encrypted keys.

#### Code Migration Examples

**JavaScript/TypeScript (Axios):**
```typescript
// ❌ OLD CODE (No longer works)
const response = await axios.get(
  '/api/v1/user/encrypted-key',
  { params: { email: 'user@example.com' } }
);

// ✅ NEW CODE (Required)
const response = await axios.post(
  '/api/v1/user/encrypted-key',
  {
    email: 'user@example.com',
    sessionId: sessionStorage.getItem('sessionId')
  }
);
```

**Python (Requests):**
```python
# ❌ OLD CODE
response = requests.get(
    'http://localhost:3000/api/v1/user/encrypted-key',
    params={'email': 'user@example.com'}
)

# ✅ NEW CODE
response = requests.post(
    'http://localhost:3000/api/v1/user/encrypted-key',
    json={
        'email': 'user@example.com',
        'sessionId': session_id
    }
)
```

**Flutter/Dart:**
```dart
// ❌ OLD CODE
final response = await http.get(
  Uri.parse('$baseUrl/api/v1/user/encrypted-key?email=$email'),
);

// ✅ NEW CODE
final response = await http.post(
  Uri.parse('$baseUrl/api/v1/user/encrypted-key'),
  headers: {'Content-Type': 'application/json'},
  body: jsonEncode({
    'email': email,
    'sessionId': sessionId,
  }),
);
```

---

## Non-Breaking Enhancements

### 2. Password Validation Enforced

The registration endpoint now enforces strong password requirements. If your application allows weak passwords, users will see validation errors.

**New Requirements:**
- Minimum 8 characters
- At least one uppercase letter (A-Z)
- At least one lowercase letter (a-z)
- At least one digit (0-9)
- At least one special character (!@#$%^&*...)

**Example Valid Passwords:**
- `SecurePass123!`
- `MyP@ssw0rd2024`
- `K@nari$ecure99`

**Error Response Example:**
```json
{
  "success": false,
  "error": "Password validation failed: Password must contain at least one uppercase letter"
}
```

**Migration Tip:** Add client-side password strength validation before submitting to avoid round-trip errors.

---

### 3. Session Validation for Transaction Signing

Transaction signing endpoints (`/api/v1/sign/transfer` and `/api/v1/sign/transaction`) now properly validate sessions. If you were using expired or invalid sessions, these will now return `401 Unauthorized`.

**Before:** Mock sessions allowed any session_id to work  
**After:** Sessions are validated against database

**Ensure Your Code:**
1. Stores session_id securely after login
2. Checks session expiration before signing
3. Re-authenticates if session expires

```typescript
// ✅ Best Practice: Check session before signing
async function signTransfer(sessionId: string, recipient: string, amount: number) {
  // Validate session first
  const validation = await axios.get(`/api/v1/session/validate/${sessionId}`);
  
  if (!validation.data.data.valid) {
    // Prompt user to re-login
    await promptReauth();
    return;
  }
  
  // Proceed with signing
  return axios.post('/api/v1/sign/transfer', {
    sessionId,
    recipient,
    amount
  });
}
```

---

## Testing Checklist

After updating your client code, verify:

- [ ] User registration rejects weak passwords with clear error messages
- [ ] Login returns a valid session_id
- [ ] Encrypted key retrieval requires session_id in POST body
- [ ] Transaction signing fails with expired/invalid sessions
- [ ] Session validation endpoint correctly reports validity
- [ ] Logout invalidates sessions properly

---

## Rollback Plan

If you encounter issues, you can temporarily rollback to v0.1.5:

```bash
git checkout v0.1.5
cargo build -p run-auth --release
```

⚠️ **Warning:** v0.1.5 contains critical security vulnerabilities. Only use for emergency rollback while fixing client code.

---

## Support

For questions or issues migrating to v0.1.6:

1. Review the [Security Audit Report](./SECURITY_AUDIT.md)
2. Check API documentation in [README.md](./README.md)
3. Open an issue on GitHub with the label `security-migration`

---

## Timeline

- **2026-05-02:** Security fixes released (v0.1.6)
- **2026-05-09:** Grace period for migration (7 days)
- **2026-05-09+:** Old encrypted key endpoint permanently removed

**Act now to ensure uninterrupted service!**

---

**Kanari SDK Team**  
Building secure, modular blockchain infrastructure
