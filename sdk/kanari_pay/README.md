# Kanari Kit - Flutter SDK for Kanari Network

A comprehensive Flutter/Dart SDK for interacting with the Kanari Network blockchain, featuring email-based authentication, wallet management, and transaction signing.

## Features

✅ **Email-Based Authentication** - Register and login with email/password  
✅ **Wallet Management** - Create and manage Kanari wallets  
✅ **Transaction Signing** - Sign and submit transactions securely  
✅ **Multi-Curve Support** - Ed25519, K256, P256, Dilithium (post-quantum)  
✅ **Session Management** - Secure session handling with auto-expiration  
✅ **RPC Client** - Full JSON-RPC API support  
✅ **Cross-Platform** - Works on iOS, Android, Web, Desktop  

## Installation

Add to your `pubspec.yaml`:

```yaml
dependencies:
  kanari_kit:
    path: ../sdk/kanari_kit  # Adjust path as needed
```

Then run:

```bash
flutter pub get
```

## Quick Start

### 1. Initialize Auth Client

```dart
import 'package:kanari_kit/kanari_kit.dart';

// Connect to your run-auth API server
final authClient = KanariAuthClient('http://localhost:3000');
```

### 2. Register a New User

```dart
final response = await authClient.register(
  email: 'alice@example.com',
  password: 'SecurePass123!',
  curveType: 'ed25519', // optional: ed25519, k256, p256, dilithium2, dilithium3, dilithium5
);

if (response.success) {
  print('Wallet created: ${response.data!.walletAddress}');
} else {
  print('Error: ${response.error}');
}
```

### 3. Login

```dart
final response = await authClient.login(
  email: 'alice@example.com',
  password: 'SecurePass123!',
  sessionTimeoutHours: 24, // optional
);

if (response.success) {
  print('Logged in! Session: ${response.data!.sessionId}');
  print('Wallet: ${response.data!.walletAddress}');
}
```

### 4. Check Authentication Status

```dart
if (authClient.isAuthenticated) {
  print('User: ${authClient.userEmail}');
  print('Wallet: ${authClient.walletAddress}');
}
```

### 5. Execute an Object-Centric Transfer

```dart
final result = await rpcClient.transfer(
  wallet: wallet,
  recipient: '0xRecipientAddress...',
  amount: 1000000, // in mist
  gasLimit: 100000,
  gasPrice: 1000,
);

print('Transaction submitted: ${result.hash}');
```

### 6. Validate Session

```dart
final response = await authClient.validateSession();

if (response.success && response.data!.valid) {
  print('Session is valid');
} else {
  print('Session expired or invalid');
  authClient.clearSession();
}
```

### 7. Change Password

```dart
final response = await authClient.changePassword(
  oldPassword: 'OldPass123!',
  newPassword: 'NewSecurePass456!',
);

if (response.success) {
  print('Password changed. All sessions invalidated.');
  // Need to login again
}
```

### 8. Logout

```dart
// Logout current session
await authClient.logout();

// OR logout all sessions
await authClient.logoutAll();
```

### 9. Delete Account

```dart
final response = await authClient.deleteAccount(
  password: 'SecurePass123!',
);

if (response.success) {
  print('Account deleted successfully');
}
```

## Using with KanariClient (RPC)

```dart
import 'package:kanari_kit/kanari_kit.dart';

// Initialize RPC client
final rpcClient = KanariClient('http://localhost:3000/rpc');

// Query owner-centric state
final owner = await rpcClient.getOwner(wallet.address);
print('Owned objects: ${owner.ownedObjects?.length ?? 0}');

// Query objects directly
final objects = await rpcClient.getOwnedObjects(
  wallet.address,
  objectType: '0x2::coin::Coin<0x2::kanari::KANARI>',
);
print('Spendable coin objects: ${objects.length}');

// RPC builds the transfer and resolves coin/object inputs server-side
final transfer = await rpcClient.transfer(
  wallet: wallet,
  recipient: '0xRecipient...',
  amount: 1000000,
);

print('Transfer hash: ${transfer.hash}');
```

## Environment Configuration

Use predefined environments or create custom ones:

```dart
// Use testnet
final authClient = KanariAuthClient(KanariEnvironment.testnet.authUrl);

// Use mainnet
final authClient = KanariAuthClient(KanariEnvironment.mainnet.authUrl);

// Custom environment
final customEnv = KanariEnvironment(
  name: 'Custom',
  rpcUrl: 'http://my-node:3000/rpc',
  authUrl: 'http://my-auth:3000',
);
final authClient = KanariAuthClient(customEnv.authUrl);
```

## Password Requirements

Passwords must meet the following criteria:

- Minimum 8 characters
- At least one uppercase letter
- At least one lowercase letter
- At least one digit
- At least one special character (!@#$%^&*)

Examples of valid passwords:

- `SecurePass123!`
- `MyP@ssw0rd`
- `Test1234!`

## Supported Curve Types

| Curve Type | Description | Use Case |
|------------|-------------|----------|
| `ed25519` | Ed25519 curve | Default, fast signatures |
| `k256` | Secp256k1 | Bitcoin/Ethereum compatible |
| `p256` | NIST P-256 | Government/enterprise |
| `dilithium2` | Post-quantum | Fast PQC (~2.5KB sigs) |
| `dilithium3` | Post-quantum | Balanced PQC (~4KB sigs) ⭐ |
| `dilithium5` | Post-quantum | Max security PQC (~5KB sigs) |

## Session Management

Sessions are automatically managed by the `KanariAuthClient`:

- **Automatic Storage**: Session ID, email, and wallet address stored in memory
- **Expiration**: Sessions expire after configured timeout (default: 24 hours)
- **Validation**: Call `validateSession()` to check if session is still valid
- **Clearing**: Sessions are cleared on logout, password change, or account deletion

### Persisting Sessions

To persist sessions across app restarts:

```dart
import 'package:shared_preferences/shared_preferences.dart';

// Save session
final prefs = await SharedPreferences.getInstance();
prefs.setString('session_id', authClient.sessionId!);
prefs.setString('user_email', authClient.userEmail!);
prefs.setString('wallet_address', authClient.walletAddress!);

// Restore session
final sessionId = prefs.getString('session_id');
final userEmail = prefs.getString('user_email');
final walletAddress = prefs.getString('wallet_address');

if (sessionId != null && userEmail != null && walletAddress != null) {
  authClient.setSession(
    sessionId: sessionId,
    userEmail: userEmail,
    walletAddress: walletAddress,
  );
  
  // Validate restored session
  final response = await authClient.validateSession();
  if (!response.success || !response.data!.valid) {
    authClient.clearSession();
    prefs.remove('session_id');
    prefs.remove('user_email');
    prefs.remove('wallet_address');
  }
}
```

## Error Handling

All API calls return `ApiResponse<T>` with consistent error handling:

```dart
final response = await authClient.login(
  email: 'user@example.com',
  password: 'password',
);

if (response.success) {
  // Handle success
  final data = response.data!;
} else {
  // Handle error
  print('Error: ${response.error}');
  
  // Check HTTP status codes from error message
  if (response.error?.contains('401') == true) {
    print('Invalid credentials');
  } else if (response.error?.contains('403') == true) {
    print('Account locked');
  } else if (response.error?.contains('409') == true) {
    print('User already exists');
  }
}
```

## Complete Example App

See `lib/main.dart` for a complete Flutter app example with:

- Registration screen
- Login screen
- Dashboard with wallet info
- Transaction signing
- Settings (change password, logout, delete account)

## Architecture

```
┌─────────────────┐
│  Flutter App    │
└────────┬────────┘
         │
    ┌────▼─────┐
    │AuthClient│ ◄── Email/Password Authentication
    └────┬─────┘
         │
    ┌────▼──────┐
    │RPC Client │ ◄── Blockchain Interaction
    └────┬──────┘
         │
    ┌────▼──────────┐
    │Kanari Network │
    └───────────────┘
```

## Security Best Practices

1. **Never store passwords** - Use biometric authentication or secure enclaves
2. **Validate sessions regularly** - Check session validity before sensitive operations
3. **Use HTTPS in production** - Always use TLS for API communication
4. **Implement rate limiting** - Prevent brute force attacks on login endpoints
5. **Clear sessions on logout** - Always call `logout()` or `clearSession()`
6. **Use strong passwords** - Enforce password requirements in UI

## Testing

Run unit tests:

```bash
flutter test
```

## Troubleshooting

### "No active session" error

Make sure you've logged in successfully:

```dart
if (!authClient.isAuthenticated) {
  // Redirect to login screen
}
```

### Session expires too quickly

Increase session timeout during login:

```dart
await authClient.login(
  email: email,
  password: password,
  sessionTimeoutHours: 168, // 7 days
);
```

### Connection refused

Check that the `run-auth` API server is running:

```bash
# Start the auth server
cargo run -p run-auth
```

## API Reference

For complete API documentation, see:

- [KanariAuthClient](lib/src/auth_client.dart)
- [KanariClient](lib/src/kanari_client.dart)
- [Models](lib/src/models/)

## Backend Setup

This SDK requires the `run-auth` API server. See [run-auth documentation](../../crates/run-auth/README.md) for setup instructions.

## Contributing

Contributions welcome! Please read our contributing guidelines and submit pull requests.

## License

Apache-2.0
