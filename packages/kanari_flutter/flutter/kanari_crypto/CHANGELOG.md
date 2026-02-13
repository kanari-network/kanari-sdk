## 0.2.5

- Added: Official Windows support with CMake configuration and automated native library loading.

## 0.2.4

- Fixed: Android build error caused by missing `kotlin-android` plugin.
- Fixed: Android compilation error where `KanariCryptoPlugin` class was not found.
- Improvement: Optimized `build.gradle` to fetch version from `pubspec.yaml` using Regex (removing SnakeYAML dependency).

## 0.2.3

- Added: Mobile support for Android and iOS.
- Fixed: "flutter_rust_bridge has not been initialized" error on mobile devices.
- Infrastructure: Added native library loading for Windows, Android (.so), and iOS (.a).
- Automation: Versioning in `build.gradle` and `podspec` is now synced automatically with `pubspec.yaml`.

## 0.2.2

- Added: `deriveMultipleAddressesApi` for HD Wallet address generation using path templates.
- Added: `deriveKeypairFromPathApi` for deriving a single keypair from a specific derivation path.
- Feature: Support for BIP44-style derivation paths (e.g., `m/44'/637'/0'/0/{index}`).
- Improvement: Updated internal Rust FFI to support new HD wallet functionalities.

## 0.2.1

- API Change: Removed `password` parameter from `deriveKeypairFromMnemonic` to align with the core Rust implementation.
- Infrastructure: Updated to Flutter Rust Bridge (FRB) v2 with new module-based code generation.
- Documentation: Added `CODEGEN_GUIDE.md` for FRB v2 generation instructions.

## 0.2.0

- Release: bump version to 0.2.0 for publication (no API changes).

## 0.1.4

- Release: updated README, added publish metadata placeholders, and
  improved package documentation for Flutter integration.

## 0.1.5

- Release: bump version to 0.1.5 for publication (no API changes).
