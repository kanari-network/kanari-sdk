/// Web plugin registration for Kanari Crypto
///
/// This file provides the web-specific plugin registration using
/// flutter_web_plugins.
library;

import 'package:flutter/foundation.dart';
import 'package:flutter_web_plugins/flutter_web_plugins.dart';

/// The web plugin class for Kanari Crypto.
///
/// This class is responsible for registering the plugin with the Flutter web engine.
class KanariCryptoWebPlugin {
  /// Register this plugin with the Flutter web engine.
  ///
  /// This method is called automatically by Flutter when the plugin is used
  /// in a web application. It initializes the Kanari Crypto library for web.
  static void registerWith(Registrar registrar) {
    // Initialize the Kanari Crypto library for web platform
    // The initialization is done lazily when first needed
    debugPrint('KanariCryptoWebPlugin registered');
  }
}
