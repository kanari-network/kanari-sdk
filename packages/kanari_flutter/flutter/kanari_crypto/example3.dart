import 'dart:convert';
import 'dart:io';
import 'package:kanari_crypto/kanari_crypto.dart';

String bytesToHex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

String bytesToBase64(List<int> bytes) => base64.encode(bytes);

/// Dilithium5-only example:
/// - Initializes the native bridge
/// - Generates a Dilithium5 keypair
/// - Signs and verifies a short message
Future<void> main() async {
  await RustLib.init();
  const bool isDev = !bool.fromEnvironment('dart.vm.product');

  try {
    final curves = await listSupportedCurves();
    final desired = 'Dilithium5';
    if (!curves.any((c) => c.name == desired)) {
      final available = curves.map((c) => c.name).join(', ');
      print('Dilithium5 not available. Available curves: $available');
      return;
    }

    print('Generating Dilithium5 keypair...');
    final kp = await generateKeypairApi(curveName: desired);

    print('Address: ${kp.address}');
    print('Public key (hex): ${kp.publicKey}');
    print('Raw public key length: ${kp.rawPublicKey.length} bytes');

    if (isDev) {
      final dynamic priv = kp.privateKey;
      if (priv is List<int>) {
        print('WARNING: Running in DEV mode — exposing private key');
        print('Private key (hex): ${bytesToHex(priv)}');
        print('Private key (base64): ${bytesToBase64(priv)}');
      } else {
        print('WARNING: Running in DEV mode — exposing private key');
        print('Private key: $priv');
      }
    }

    final message = 'kanari pq test'.codeUnits;
    final sig = await signMessageApi(
      privateKey: kp.privateKey,
      message: message,
      curveName: desired,
    );

    print('Signature (base64): ${bytesToBase64(sig)}');

    final addressForVerify = kp.rawPublicKey.isNotEmpty
        ? bytesToHex(kp.rawPublicKey)
        : kp.address;

    final ok = await verifySignatureApi(
      address: addressForVerify,
      message: message,
      signature: sig,
      curveName: desired,
    );

    print('Verification result: $ok');
  } catch (e, st) {
    stderr.writeln('Example3 failed: $e');
    stderr.writeln(st);
  } finally {
    RustLib.dispose();
  }
}
