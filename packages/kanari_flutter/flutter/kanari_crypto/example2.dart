import 'dart:convert';
import 'dart:io';
import 'package:kanari_crypto/kanari_crypto.dart';

String bytesToHex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

String bytesToBase64(List<int> bytes) => base64.encode(bytes);

/// Minimal non-interactive example:
/// - Initializes the native bridge
/// - Generates a mnemonic
/// - Derives a keypair for the first supported curve
/// - Signs and verifies a short message
Future<void> main() async {
  await RustLib.init();

  try {
    final curves = await listSupportedCurves();
    if (curves.isEmpty) {
      print('No curves available.');
      return;
    }

    // Use only P256 for this example.
    const desired = 'P256';
    final hasP256 = curves.any((c) => c.name == desired);
    if (!hasP256) {
      final available = curves.map((c) => c.name).join(', ');
      print('P256 not available in this build. Available curves: $available');
      return;
    }

    final curve = desired;
    print('Using curve: $curve');

    final mnemonic = await generateMnemonicApi(wordCount: BigInt.from(12));
    print('Mnemonic: $mnemonic');

    final kp = await deriveKeypairFromMnemonic(
      mnemonic: mnemonic,
      curveName: curve,
      password: '',
    );

    print('Derived address: ${kp.address}');

    final message = 'kanari example'.codeUnits;
    final sig = await signMessageApi(
      privateKey: kp.privateKey,
      message: message,
      curveName: curve,
    );

    print('Signature (hex): ${bytesToHex(sig)}');
    print('Signature (base64): ${bytesToBase64(sig)}');

    final addressForVerify = kp.rawPublicKey.isNotEmpty
        ? bytesToHex(kp.rawPublicKey)
        : kp.address;

    final ok = await verifySignatureApi(
      address: addressForVerify,
      message: message,
      signature: sig,
      curveName: curve,
    );

    print('Verification OK: $ok');
  } catch (e, st) {
    stderr.writeln('Example2 failed: $e');
    stderr.writeln(st);
  } finally {
    RustLib.dispose();
  }
}
