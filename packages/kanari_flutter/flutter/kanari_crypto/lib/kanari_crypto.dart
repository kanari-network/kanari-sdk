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
import 'src/frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

/// Initialize the Kanari Crypto library.
/// On Windows, it handles loading the native shared library.
Future<void> initKanariCrypto() async {
  try {
    if (Platform.isWindows) {
      // Try to find the DLL in common locations or relative to the package
      // For development in this monorepo:
      final possiblePaths = [
        'rust.dll', // Current directory
        '../../packages/kanari_flutter/flutter/kanari_crypto/lib/src/rust.dll', // Relative to sdk/kanari_kit
        'C:/Users/Pukpuy/Desktop/kanari-sdk/packages/kanari_flutter/flutter/kanari_crypto/lib/src/rust.dll', // Absolute
      ];

      for (final path in possiblePaths) {
        try {
          if (File(path).existsSync() || path == 'rust.dll') {
            await RustLib.init(
              externalLibrary: ExternalLibrary.open(path),
            );
            return;
          }
        } catch (_) {}
      }
    }
    
    // Fallback to default init for other platforms or if Windows path finding fails
    await RustLib.init();
  } catch (e) {
    rethrow;
  }
}
