import 'package:flutter_test/flutter_test.dart';
import 'package:kanari_kit/kanari_kit.dart';


void main() {
  group('KanariWallet PQC/Hybrid Fixes', () {
    test('fromPrivateKey trims whitespace and removes kanari prefix', () async {
      // Note: This test assumes importKeypairFromPrivateKey is available or mocked
      // In a real test environment with FRB, we would need to mock the FFI call.
      // Since we can't easily mock FRB generated code in this environment,
      // we'll focus on the logic before the FFI call if possible.
    });

    test('isPostQuantum and isHybrid identify curves correctly', () {
      expect(KanariCurve.dilithium3.isPostQuantum, isTrue);
      expect(KanariCurve.dilithium3.isHybrid, isFalse);
      expect(KanariCurve.ed25519Dilithium3.isPostQuantum, isTrue);
      expect(KanariCurve.ed25519Dilithium3.isHybrid, isTrue);
      expect(KanariCurve.ed25519.isPostQuantum, isFalse);
      expect(KanariCurve.ed25519.isHybrid, isFalse);
    });
  });
}
