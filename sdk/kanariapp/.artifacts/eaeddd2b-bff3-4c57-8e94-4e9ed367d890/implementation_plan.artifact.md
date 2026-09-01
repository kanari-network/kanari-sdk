# Implementation Plan - Migrating Kanari Pay from Flutter to Kotlin

This plan outlines the steps to port the `kanari_pay` Flutter project to the native Android (Kotlin/Compose) project `kanariapp`.

## User Review Required

> [!IMPORTANT]
>
> - **Security**: The Flutter project uses `flutter_secure_storage` and `cryptography` for PIN/Biometric protection. In Kotlin, we will use `EncryptedSharedPreferences` and `BiometricPrompt`.
> - **Networking**: We will use Retrofit and OkHttp for REST APIs and JSON-RPC.
> - **Dependency Update**: New dependencies (Retrofit, Kotlinx Serialization, Navigation) will be added to `libs.versions.toml`.

## Proposed Changes

### Build Configuration

#### [MODIFY] [libs.versions.toml](file:///D:/move-jamae/kanari-sdk/sdk/kanariapp/gradle/libs.versions.toml)

- Add versions and libraries for Retrofit, OkHttp, Kotlinx Serialization, and Navigation Compose.

#### [MODIFY] [build.gradle.kts (app)](file:///D:/move-jamae/kanari-sdk/sdk/kanariapp/app/build.gradle.kts)

- Apply Kotlinx Serialization plugin.
- Add the new dependencies.

---

### Data Models

#### [NEW] `com.jamesatomc.kanariapp.network.models`

- Translate all models from `kanari_pay/lib/src/models`:
  - `AuthModels.kt` (RegisterRequest/Response, LoginRequest/Response, etc.)
  - `AccountModels.kt` (AccountInfo, TokenBalance, ObjectInfo)
  - `TransactionModels.kt` (TransactionDetails, TransactionResult)
  - `RpcModels.kt` (Generic RpcResponse)
  - `Environment.kt` (Mainnet, Testnet configurations)

---

### Network Clients

#### [NEW] `com.jamesatomc.kanariapp.network.AuthService`

- Retrofit interface for Auth API (`/register`, `/login`, `/logout`, etc.).

#### [NEW] `com.jamesatomc.kanariapp.network.KanariRpcService`

- Retrofit interface for Blockchain RPC API.

#### [NEW] `com.jamesatomc.kanariapp.network.KanariClient`

- Facade class coordinating RPC and Auth services, similar to the Flutter implementation.

---

### Wallet & Storage

#### [NEW] `com.jamesatomc.kanariapp.wallet.KanariWallet`

- Port `KanariWallet.dart` logic, wrapping `KanariCrypto` calls.

#### [NEW] `com.jamesatomc.kanariapp.wallet.WalletStorage`

- Port `WalletStorage.dart` logic using `EncryptedSharedPreferences` for secure storage of encrypted keys and PIN hashes.
- Implement PBKDF2 and AES-GCM for manual encryption of mnemonic/private keys.

---

### UI Screens (Compose)

#### [MODIFY] `com.jamesatomc.kanariapp.MainActivity.kt`

- Set up Navigation Host.

#### [NEW] `com.jamesatomc.kanariapp.ui.screens`

- `LoginScreen.kt`
- `RegisterScreen.kt`
- `DashboardScreen.kt` (Wallet info, balances)
- `SendScreen.kt` (Transfer tokens)
- `ReceiveScreen.kt` (QR code)
- `SettingsScreen.kt` (Change password, logout)

## Verification Plan

### Automated Tests

- Unit tests for `WalletStorage` encryption/decryption.
- Unit tests for `AuthClient` and `KanariClient` API mapping.
- UI tests for the Navigation flow.

### Manual Verification

- Deploy to emulator/device.
- Test registration and login flow against a local `run-auth` server.
- Test wallet generation and address display.
- Test biometric unlock (if available on device).
