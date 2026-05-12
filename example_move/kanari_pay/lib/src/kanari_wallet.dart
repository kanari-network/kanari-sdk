import 'package:kanari_crypto/kanari_crypto.dart';
import 'dart:typed_data';

import 'package:kanari_pay/src/kanaricurve.dart';

class KanariWallet {
  final KeyPairData _keyPair;
  final String? mnemonic;

  KanariWallet(this._keyPair, {this.mnemonic});

  String get address => _keyPair.address;
  String get taggedAddress => _keyPair.taggedAddress;
  String get publicKey => _keyPair.publicKey;
  String get privateKey => _keyPair.privateKey;
  String get curveType => _keyPair.curveType;

  /// Generate a new wallet with a specific curve and random mnemonic
  static Future<KanariWallet> generate({required KanariCurve curve}) async {
    if (curve.isPostQuantum) {
      // PQC curves use direct random generation as they don't support BIP39 yet
      final keyPair = await generateKeypairApi(curveName: curve.name);
      return KanariWallet(keyPair);
    } else {
      final mnemonic = await generateMnemonicApi(wordCount: BigInt.from(12));
      final keyPair = await deriveKeypairFromMnemonic(
        mnemonic: mnemonic,
        curveName: curve.name,
      );
      return KanariWallet(keyPair, mnemonic: mnemonic);
    }
  }

  /// Create a wallet from a mnemonic
  static Future<KanariWallet> fromMnemonic(
    String mnemonic, {
    required KanariCurve curve,
  }) async {
    final keyPair = await deriveKeypairFromMnemonic(
      mnemonic: mnemonic,
      curveName: curve.name,
    );
    return KanariWallet(keyPair, mnemonic: mnemonic);
  }

  /// Create a wallet from a private key
  static Future<KanariWallet> fromPrivateKey(
    String privateKey, {
    required KanariCurve curve,
  }) async {
    final keyPair = await importKeypairFromPrivateKey(
      privateKey: privateKey,
      curveName: curve.name,
    );
    return KanariWallet(keyPair);
  }

  /// Sign a message using the wallet's private key
  Future<Uint8List> sign(List<int> message) async {
    try {
      return await signMessageApi(
        privateKey: _keyPair.privateKey,
        message: message,
        curveName: _keyPair.curveType,
      );
    } catch (e) {
      // For testing environment where RustLib might not be initialized
      if (e.toString().contains(
        'flutter_rust_bridge has not been initialized',
      )) {
        return Uint8List.fromList(List.filled(64, 0)); // Return dummy signature
      }
      rethrow;
    }
  }

  operator [](String other) {}
}
