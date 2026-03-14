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
export 'src/init.dart'
    if (dart.library.io) 'src/init.io.dart';

