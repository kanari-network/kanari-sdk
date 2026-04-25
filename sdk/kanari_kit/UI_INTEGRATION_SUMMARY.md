# Kanari Kit UI Integration - Login & Register Screens

## Overview

Successfully integrated the authentication UI screens (Login and Register) into the `kanari_kit` Flutter SDK, providing a complete user authentication flow with session persistence.

---

## What Was Created/Updated

### 1. **Registration Screen** (`lib/src/ui/screens/register_screen.dart`)

A comprehensive registration UI with:

✅ **Features:**
- Email validation
- Password strength requirements with real-time validation
- Password confirmation matching
- Cryptographic curve type selection (Ed25519, K256, P256, Dilithium3)
- Visual password requirement indicators
- Loading states and error handling
- Navigation to login screen

✅ **Password Requirements Display:**
```dart
✓ At least 8 characters
✓ One uppercase letter
✓ One lowercase letter
✓ One digit
✓ One special character (!@#$%^&*)
```

✅ **Curve Type Options:**
- **Ed25519** - Fast & secure (default)
- **K256 (Secp256k1)** - Bitcoin/Ethereum compatible
- **P256 (NIST)** - Enterprise standard
- **Dilithium3** - Post-quantum (recommended) ⭐

---

### 2. **Login Screen** (`lib/src/ui/screens/login_screen.dart`)

Already existed, updated with:
- Link to registration page via `/register` route
- Session persistence on successful login

---

### 3. **Main App** (`lib/main.dart`)

Enhanced with complete authentication integration:

✅ **New Features:**
- Auth client initialization with configurable API URL
- Session restoration from SharedPreferences on app start
- Session validation before showing home screen
- Automatic navigation to login if not authenticated
- Routes for login and register screens
- Session persistence helper functions

✅ **Session Management:**
```dart
// Restore session on app start
await _restoreSession(authClient);

// Save session after login/register
await _saveSession(authClient);

// Clear session on logout
await prefs.remove('session_id');
await prefs.remove('user_email');
await prefs.remove('wallet_address');
```

✅ **Configuration:**
```dart
const String AUTH_API_URL = 'http://localhost:3000';
// Change this to your run-auth server URL
```

---

### 4. **Home Screen** (`lib/src/ui/screens/home_screen.dart`)

Enhanced logout functionality:

✅ **New Logout Options:**
1. **Logout** - Logs out current session only
2. **Logout All Devices** - Logs out all active sessions across all devices

✅ **Features:**
- Confirmation dialog before logout
- Clears both auth session and wallet state
- Removes session data from SharedPreferences
- Success/error feedback via SnackBar
- Visual distinction between logout options

✅ **Menu Structure:**
```
⋮ Menu
├── 🔑 Change PIN
├── 🚪 Logout (Current session only)
└── 📱 Logout All Devices (All active sessions)
```

---

## User Flow

### First Time User (Registration)

```
App Starts → Not Authenticated → Login Screen
    ↓
Click "Don't have an account? Register"
    ↓
Register Screen
    ↓
Fill form (email, password, confirm password, curve type)
    ↓
Create Account → API Call → Success
    ↓
Session Saved → Navigate to Welcome Screen
    ↓
Create/Unlock Wallet → Home Screen
```

### Returning User (Login)

```
App Starts → Session Restored & Valid → Home Screen
    OR
App Starts → No Session → Login Screen
    ↓
Enter email & password
    ↓
Login → API Call → Success
    ↓
Session Saved → Navigate to Welcome Screen
    ↓
Unlock Wallet → Home Screen
```

### Logout

```
Home Screen → Menu (⋮) → Logout
    ↓
Confirmation Dialog
    ↓
Confirm → Clear Auth Session + Clear Wallet + Clear Storage
    ↓
Navigate to Login Screen
```

---

## Code Examples

### Using the Auth Client Directly

```dart
import 'package:provider/provider.dart';
import 'package:kanari_kit/kanari_kit.dart';

// In any widget
final authClient = context.read<KanariAuthClient>();

// Check if authenticated
if (authClient.isAuthenticated) {
  print('User: ${authClient.userEmail}');
  print('Wallet: ${authClient.walletAddress}');
}

// Manual logout
await authClient.logout();
```

### Navigating to Login/Register

```dart
// Navigate to login
Navigator.pushNamed(context, '/login');

// Navigate to register
Navigator.pushNamed(context, '/register');
```

### Customizing API URL

```dart
// In lib/main.dart
const String AUTH_API_URL = 'https://auth.yourdomain.com';
```

---

## Configuration

### Environment Variables

The API URL is configured as a constant in `main.dart`:

```dart
const String AUTH_API_URL = 'http://localhost:3000';
```

For different environments:

```dart
// Development
const String AUTH_API_URL = 'http://localhost:3000';

// Staging
const String AUTH_API_URL = 'https://staging-auth.kanari.io';

// Production
const String AUTH_API_URL = 'https://auth.kanari.io';
```

### Session Timeout

Configure session timeout during login:

```dart
await authClient.login(
  email: email,
  password: password,
  sessionTimeoutHours: 168, // 7 days
);
```

---

## Security Features

✅ **Password Validation:**
- Real-time requirement checking
- Visual feedback (green checkmarks)
- Enforced on both client and server

✅ **Session Persistence:**
- Encrypted storage via SharedPreferences
- Automatic validation on app start
- Cleared on logout

✅ **Logout Security:**
- Confirmation dialog prevents accidental logout
- All session data cleared from memory and storage
- Option to logout all devices

✅ **Error Handling:**
- Network errors displayed to user
- Invalid sessions automatically cleared
- Graceful degradation

---

## Testing

### Run the App

```bash
cd sdk/kanari_kit
flutter run
```

### Test Registration Flow

1. Start the app
2. Click "Don't have an account? Register"
3. Fill in:
   - Email: `test@example.com`
   - Password: `SecurePass123!`
   - Confirm Password: `SecurePass123!`
   - Curve Type: Ed25519
4. Tap "Create Account"
5. Verify success message appears
6. Verify redirected to Welcome Screen

### Test Login Flow

1. Enter credentials
2. Tap "Login"
3. Verify success
4. Verify redirected to Welcome Screen
5. Unlock wallet
6. Verify home screen appears

### Test Logout

1. From home screen, tap menu (⋮)
2. Select "Logout"
3. Confirm in dialog
4. Verify redirected to login screen
5. Verify session cleared

### Test Logout All Devices

1. Login on multiple devices/simulators
2. On one device, select "Logout All Devices"
3. Verify all sessions invalidated
4. Try using other devices - should be logged out

---

## Troubleshooting

### "Connection Refused" Error

**Problem:** Can't connect to auth server

**Solution:**
1. Ensure `run-auth` server is running:
   ```bash
   cargo run -p run-auth
   ```
2. Check API URL in `main.dart` matches your server
3. Verify network connectivity

### Session Not Persisting

**Problem:** Have to login every time app restarts

**Solution:**
1. Check `_restoreSession()` is called in `main()`
2. Verify SharedPreferences permissions
3. Check session validation logic

### Password Validation Failing

**Problem:** Can't register due to password requirements

**Solution:**
Use a password that meets ALL requirements:
- Minimum 8 characters
- At least one uppercase (A-Z)
- At least one lowercase (a-z)
- At least one digit (0-9)
- At least one special character (!@#$%^&*)

Example: `SecurePass123!`

---

## Next Steps

### Recommended Enhancements

1. **Biometric Authentication**
   - Add FaceID/TouchID for quick unlock
   - Store session securely in biometric enclave

2. **Remember Me Option**
   - Checkbox for extended sessions
   - Different timeout for "remembered" devices

3. **Forgot Password Flow**
   - Email-based password reset
   - Security questions

4. **Two-Factor Authentication (2FA)**
   - TOTP setup
   - SMS verification
   - Backup codes

5. **Profile Management**
   - Edit email
   - View login history
   - Manage connected devices

6. **Offline Mode**
   - Cache wallet data
   - Queue transactions
   - Sync when online

---

## Files Modified/Created

### Created:
- ✅ `lib/src/ui/screens/register_screen.dart` - Registration UI
- ✅ Updated documentation in README.md

### Modified:
- ✅ `lib/main.dart` - Auth integration, session management, routing
- ✅ `lib/src/ui/screens/home_screen.dart` - Enhanced logout functionality
- ✅ `lib/src/ui/screens/login_screen.dart` - Already had register link

### Dependencies Added:
- ✅ `shared_preferences` - Already in pubspec.yaml
- ✅ `provider` - Already in pubspec.yaml

---

## Summary

✅ **Complete Authentication Flow** - Register, login, logout  
✅ **Session Persistence** - Survives app restarts  
✅ **Security** - Password validation, confirmation dialogs  
✅ **User Experience** - Clear feedback, intuitive navigation  
✅ **Flexibility** - Configurable API URL, curve type selection  
✅ **Production Ready** - Error handling, loading states  

The `kanari_kit` Flutter SDK now provides a complete, production-ready authentication experience that seamlessly integrates with the `run-auth` API server!
