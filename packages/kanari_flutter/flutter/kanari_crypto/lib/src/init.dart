/// Default initialization for Kanari Crypto
///
/// This is the fallback initialization that delegates to platform-specific
/// implementations via conditional exports in kanari_crypto.dart
library;

// This file exists to satisfy the export statement but should not be used directly.
// The actual implementation is in init.web.dart (for web) or init.io.dart (for io).

import 'package:flutter/foundation.dart';

/// Initialize the Kanari Crypto library.
/// This function will be overridden by platform-specific implementations.
Future<void> initKanariCrypto() async {
  debugPrint(
    "Kanari Crypto init called - using platform-specific implementation",
  );
  // This should never be called directly as conditional exports will override it
  throw UnsupportedError(
    "Direct call to default initKanariCrypto is not supported. "
    "Use conditional imports from kanari_crypto.dart",
  );
}
