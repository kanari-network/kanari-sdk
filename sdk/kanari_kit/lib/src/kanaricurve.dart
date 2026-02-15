/// Supported cryptographic algorithms for Kanari Network
enum KanariCurve {
  /// Secp256k1 curve (used by Bitcoin and Ethereum)
  k256('K256'),

  /// Secp256r1 curve (NIST P-256)
  p256('P256'),

  /// Ed25519 curve (modern, fast signature scheme)
  ed25519('Ed25519'),

  /// Dilithium2 - Fast, ~2.5KB signatures, NIST Level 2 security
  dilithium2('Dilithium2'),

  /// Dilithium3 - Balanced, ~4KB signatures, NIST Level 3 security (Recommended)
  dilithium3('Dilithium3'),

  /// Dilithium5 - Maximum security, ~5KB signatures, NIST Level 5 security
  dilithium5('Dilithium5'),

  /// SPHINCS+ SHA256-256f-robust - Hash-based, ~50KB signatures, ultra-secure
  sphincsPlusSha256Robust('SphincsPlusSha256Robust'),

  /// Ed25519 + Dilithium3 hybrid (Best of both worlds)
  ed25519Dilithium3('Ed25519Dilithium3'),

  /// K256 + Dilithium3 hybrid (Bitcoin/Ethereum compatible + quantum-safe)
  k256Dilithium3('K256Dilithium3');

  final String name;
  const KanariCurve(this.name);

  bool get isPostQuantum =>
      this == KanariCurve.dilithium2 ||
      this == KanariCurve.dilithium3 ||
      this == KanariCurve.dilithium5 ||
      this == KanariCurve.sphincsPlusSha256Robust ||
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
