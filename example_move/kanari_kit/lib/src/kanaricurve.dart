/// Supported cryptographic algorithms for Kanari Network
enum KanariCurve {
  /// Secp256k1 curve (used by Bitcoin and Ethereum)
  k256('K256'),

  /// Secp256r1 curve (NIST P-256)
  p256('P256'),

  /// Ed25519 curve (modern, fast signature scheme)
  ed25519('Ed25519'),

  /// Ed25519 + Dilithium3 hybrid (Best of both worlds)
  ed25519Dilithium3('Ed25519Dilithium3'),

  /// K256 + Dilithium3 hybrid (Bitcoin/Ethereum compatible + quantum-safe)
  k256Dilithium3('K256Dilithium3');

  final String name;
  const KanariCurve(this.name);

  bool get isPostQuantum =>
      this == KanariCurve.ed25519Dilithium3 ||
      this == KanariCurve.k256Dilithium3;

  bool get isHybrid =>
      this == KanariCurve.ed25519Dilithium3 ||
      this == KanariCurve.k256Dilithium3;

  static KanariCurve fromString(String curve) {
    return KanariCurve.values.firstWhere(
      (e) => e.name.toLowerCase() == curve.toLowerCase(),
      orElse: () => KanariCurve.ed25519,
    );
  }
}
