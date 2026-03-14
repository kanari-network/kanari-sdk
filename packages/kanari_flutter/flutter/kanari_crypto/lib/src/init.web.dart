/// Web-specific initialization for Kanari Crypto
///
/// On Web platforms, flutter_rust_bridge uses WASM modules instead of
/// native libraries. This file handles the web-specific loading logic.
library;

import 'package:flutter/foundation.dart';
import 'frb_generated.dart';

/// Initialize the Kanari Crypto library for Web platform.
/// Uses WASM module loading through flutter_rust_bridge.
Future<void> initKanariCrypto() async {
  if (RustLib.instance.initialized) {
    debugPrint("Kanari Crypto already initialized");
    return;
  }

  try {
    debugPrint("Initializing Kanari Crypto for Web...");

    // For Web, flutter_rust_bridge automatically loads the WASM module
    // The WASM binary should be built and placed in the correct location
    // using wasm-pack with target web

    await RustLib.init();
    debugPrint("Kanari Crypto initialized successfully for Web");
  } catch (e) {
    if (e.toString().contains('already initialized')) {
      debugPrint("Kanari Crypto was already initialized (caught exception)");
      return;
    }
    debugPrint("Kanari Crypto Web initialization failed: $e");
    debugPrint("TIP: For Web, ensure you have built the WASM module using:");
    debugPrint(
      "  cargo install wasm-pack && wasm-pack build --target web --release",
    );
    debugPrint("in the rust/ directory of the kanari_crypto package.");
    rethrow;
  }
}
