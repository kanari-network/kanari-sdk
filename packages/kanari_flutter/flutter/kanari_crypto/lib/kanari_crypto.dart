/// kanari_crypto
///
/// Flutter-friendly bindings for the Kanari SDK cryptography. This
/// package exposes high-level helpers for key generation, deterministic
/// wallets (BIP39), signing, verification, and keystore helpers backed
/// by the native Kanari cryptography implementation.
///
/// Import the package to access the public API in `src/api.dart`.
library;

export 'src/api.dart';
export 'src/frb_generated.dart';

import 'dart:io';
import 'package:flutter/foundation.dart';
import 'src/frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

/// Initialize the Kanari Crypto library.
/// On Windows, it handles loading the native shared library.
Future<void> initKanariCrypto() async {
  if (RustLib.instance.initialized) {
    debugPrint("Kanari Crypto already initialized");
    return;
  }

  try {
    if (Platform.isWindows) {
      debugPrint("Initializing Kanari Crypto for Windows...");
      // Try to find the DLL in common locations or relative to the package
      // For development in this monorepo:
      final possiblePaths = [
        'rust.dll', // Current directory
        'kanari_crypto.dll', // Current directory (renamed)
        '../../packages/kanari_flutter/flutter/kanari_crypto/lib/src/rust.dll', // Relative to sdk/kanari_kit
      ];

      bool loaded = false;
      for (final path in possiblePaths) {
        try {
          if (File(path).existsSync() || path == 'rust.dll') {
            debugPrint("Attempting to load library from: $path");
            await RustLib.init(externalLibrary: ExternalLibrary.open(path));
            loaded = true;
            debugPrint("Kanari Crypto loaded successfully from $path");
            break;
          }
        } catch (e) {
          debugPrint("Failed to load from $path: $e");
        }
      }

      if (loaded) return;
      debugPrint(
        "Windows specific loading failed, falling back to default init",
      );
    }

    // Fallback to default init for other platforms (iOS, Android, macOS, Linux)
    // flutter_rust_bridge handles loading the library automatically on these platforms
    debugPrint(
      "Initializing Kanari Crypto with default settings (for Mobile/Other)...",
    );
    await RustLib.init();
    debugPrint("Kanari Crypto initialized successfully");
  } catch (e) {
    if (e.toString().contains('already initialized')) {
      debugPrint("Kanari Crypto was already initialized (caught exception)");
      return;
    }
    debugPrint("Kanari Crypto initialization failed: $e");
    debugPrint(
      "TIP: For Android, ensure 'librust.so' is in 'android/src/main/jniLibs/<arch>/'",
    );
    rethrow;
  }
}
