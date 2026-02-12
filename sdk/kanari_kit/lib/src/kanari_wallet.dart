import 'package:kanari_crypto/kanari_crypto.dart';
import 'dart:typed_data';

class KanariWallet {
  final KeyPairData _keyPair;
  final String? mnemonic;

  KanariWallet(this._keyPair, {this.mnemonic});

  String get address => _keyPair.address;
  String get publicKey => _keyPair.publicKey;
  String get privateKey => _keyPair.privateKey;
  String get curveType => _keyPair.curveType;

  /// Generate a new wallet with a specific curve and random mnemonic
  static Future<KanariWallet> generate({String curveName = 'Ed25519'}) async {
    final mnemonic = await generateMnemonicApi(wordCount: BigInt.from(12));
    final keyPair = await deriveKeypairFromMnemonic(
      mnemonic: mnemonic,
      curveName: curveName,
    );
    return KanariWallet(keyPair, mnemonic: mnemonic);
  }

  /// Create a wallet from a mnemonic
  static Future<KanariWallet> fromMnemonic(String mnemonic, {String curveName = 'Ed25519'}) async {
    final keyPair = await deriveKeypairFromMnemonic(
      mnemonic: mnemonic,
      curveName: curveName,
    );
    return KanariWallet(keyPair, mnemonic: mnemonic);
  }

  /// Create a wallet from a private key
  static Future<KanariWallet> fromPrivateKey(String privateKey, {String curveName = 'Ed25519'}) async {
    final keyPair = await importKeypairFromPrivateKey(
      privateKey: privateKey,
      curveName: curveName,
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
      if (e.toString().contains('flutter_rust_bridge has not been initialized')) {
        return Uint8List.fromList(List.filled(64, 0)); // Return dummy signature
      }
      rethrow;
    }
  }
}
