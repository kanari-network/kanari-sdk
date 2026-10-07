// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

import 'package:flutter_test/flutter_test.dart';
import 'package:kanari_crypto/kanari_crypto.dart';
import 'dart:typed_data';

class MockRustApi extends RustLibApi {
  @override
  Future<KeyPairData> crateApiGenerateKeypairApi({
    required String curveName,
  }) async {
    return KeyPairData(
      privateKey: 'priv_$curveName',
      publicKey: 'pub_$curveName',
      address: '0x_addr_$curveName',
      taggedAddress: 'tagged_$curveName',
      rawPublicKey: Uint8List.fromList([]),
      curveType: curveName,
    );
  }

  @override
  Future<Uint8List> crateApiBlake3HashApi({required List<int> data}) async {
    return Uint8List.fromList([1, 2, 3]);
  }

  @override
  Future<String> crateApiGenerateMnemonicApi({
    required BigInt wordCount,
  }) async {
    return 'alpha beta gamma';
  }

  @override
  Future<List<CurveInfo>> crateApiListSupportedCurves() async {
    return const [
      CurveInfo(
        name: 'K256',
        isPostQuantum: false,
        isHybrid: false,
        securityLevel: 128,
      ),
    ];
  }

  @override
  Future<Uint8List> crateApiSignMessageApi({
    required String privateKey,
    required List<int> message,
    required String curveName,
  }) async {
    return Uint8List.fromList([1, 2, 3]);
  }

  @override
  Future<bool> crateApiVerifySignatureApi({
    required String address,
    required List<int> message,
    required List<int> signature,
    required String curveName,
  }) async {
    return true;
  }

  @override
  Future<KeyPairData> crateApiDeriveKeypairFromMnemonic({
    required String mnemonic,
    required String curveName,
  }) async {
    return crateApiGenerateKeypairApi(curveName: curveName);
  }

  @override
  Future<KeyPairData> crateApiImportKeypairFromPrivateKey({
    required String privateKey,
    required String curveName,
  }) async {
    return crateApiGenerateKeypairApi(curveName: curveName);
  }

  @override
  Future<KeyPairData> crateApiDeriveKeypairFromSeedApi({
    required List<int> seed,
    required String curveName,
  }) async {
    // Mirror the real contract: seeds shorter than 64 bytes are rejected.
    if (seed.length < 64) {
      throw Exception(
        'Seed derivation failed: Seed material must be at least 64 bytes',
      );
    }
    // Deterministic mock material (test-only): fold seed bytes into a tag so
    // determinism and cross-curve distinctness are assertable.
    var tag = 0;
    for (var i = 0; i < seed.length; i++) {
      tag = ((tag * 31) + seed[i]) & 0xFFFFFFFF;
    }
    final hexTag = tag.toRadixString(16).padLeft(8, '0');
    return KeyPairData(
      privateKey: 'seedpriv_${curveName}_$hexTag',
      publicKey: 'seedpub_${curveName}_$hexTag',
      address: '0x_seed_${curveName}_$hexTag',
      taggedAddress: 'tagged_${curveName}_$hexTag',
      rawPublicKey: Uint8List.fromList([]),
      curveType: curveName,
    );
  }

  @override
  Future<KeyPairData> crateApiDeriveKeypairFromPathApi({
    required String mnemonic,
    required String derivationPath,
    required String curveName,
  }) async {
    return crateApiGenerateKeypairApi(curveName: curveName);
  }

  @override
  Future<List<KeyPairData>> crateApiDeriveMultipleAddressesApi({
    required String mnemonic,
    required String pathTemplate,
    required String curveName,
    required BigInt count,
  }) async {
    final list = <KeyPairData>[];
    for (var i = 0; i < count.toInt(); i++) {
      list.add(await crateApiGenerateKeypairApi(curveName: curveName));
    }
    return list;
  }

  @override
  Future<ZkLoginNonceData> crateApiZkloginPrepareNonce({
    required BigInt maxEpoch,
  }) async {
    return ZkLoginNonceData(
      ephemeralPubkey: Uint8List.fromList(List<int>.filled(32, 1)),
      ephemeralSecret: Uint8List.fromList(List<int>.filled(32, 2)),
      randomness: Uint8List.fromList(List<int>.filled(32, 3)),
      maxEpoch: maxEpoch,
      nonce: 'mock-nonce-${maxEpoch.toInt()}',
    );
  }

  @override
  Future<ZkLoginClaimsData> crateApiZkloginVerifyJwt({
    required String jwt,
    required String jwksJson,
    required String expectedIss,
    required String expectedAud,
    String? expectedNonce,
    required BigInt nowSecs,
  }) async {
    return ZkLoginClaimsData(
      iss: expectedIss,
      aud: expectedAud,
      sub: 'mock-sub',
      exp: nowSecs + BigInt.from(3600),
      nonce: expectedNonce,
    );
  }

  @override
  Future<String> crateApiZkloginDeriveAddress({
    required String iss,
    required String aud,
    required String sub,
    required List<int> salt,
  }) async {
    return 'ZkLogin:mock-$sub-${salt.length}';
  }

  @override
  Future<Uint8List> crateApiZkloginDeterministicSalt({
    required String iss,
    required String aud,
    required String sub,
  }) async {
    return Uint8List.fromList(List<int>.filled(32, 7));
  }

  @override
  Future<Uint8List> crateApiZkloginSignEphemeral({
    required List<int> secret,
    required List<int> message,
  }) async {
    return Uint8List.fromList(List<int>.filled(64, 9));
  }

  @override
  Future<bool> crateApiZkloginVerifyEphemeral({
    required List<int> pubkey,
    required List<int> message,
    required List<int> signature,
  }) async {
    return signature.length == 64;
  }

  @override
  Future<Uint8List> crateApiZkloginBuildBundle({
    required String jwt,
    required String jwksJson,
    required String iss,
    required String aud,
    required List<int> salt,
    required List<int> randomness,
    required List<int> ephemeralPubkey,
    required List<int> ephemeralSig,
    required BigInt maxEpoch,
  }) async {
    return Uint8List.fromList([
      salt.length,
      randomness.length,
      maxEpoch.toInt() & 0xFF,
    ]);
  }
}

void main() {
  setUpAll(() {
    RustLib.initMock(api: MockRustApi());
  });

  tearDownAll(() {
    RustLib.dispose();
  });

  test('generate mnemonic', () async {
    final mnemonic = await generateMnemonicApi(wordCount: BigInt.from(12));
    expect(mnemonic, isNotEmpty);
  });

  test('generate keypair and sign/verify', () async {
    final kp = await generateKeypairApi(curveName: 'K256');
    expect(kp.address, startsWith('0x'));

    final sig = await signMessageApi(
      privateKey: kp.privateKey,
      message: 'hello'.codeUnits,
      curveName: 'K256',
    );
    expect(sig, isNotEmpty);

    final verified = await verifySignatureApi(
      address: kp.address,
      message: 'hello'.codeUnits,
      signature: sig,
      curveName: 'K256',
    );
    expect(verified, isTrue);
  });

  test('list supported curves', () async {
    final curves = await listSupportedCurves();
    expect(curves, isNotEmpty);
    expect(curves.first.name, 'K256');
  });
  test('Dilithium5 keypair, sign and verify', () async {
    final kp = await generateKeypairApi(curveName: 'Dilithium5');
    expect(kp.curveType, contains('Dilithium5'));

    final sig = await signMessageApi(
      privateKey: kp.privateKey,
      message: 'dilithium'.codeUnits,
      curveName: 'Dilithium5',
    );
    expect(sig, isNotEmpty);

    final verified = await verifySignatureApi(
      address: kp.publicKey,
      message: 'dilithium'.codeUnits,
      signature: sig,
      curveName: 'Dilithium5',
    );
    expect(verified, isTrue);
  });

  test('PQC mnemonic derivation works for all post-quantum curves', () async {
    // NOTE: these run against MockRustApi (binding surface only).
    // Real cryptographic correctness is frozen by kanari-crypto KAT vectors.
    const mnemonic =
        'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';
    const pqcCurves = [
      'Dilithium2',
      'Dilithium3',
      'Dilithium5',
      'SphincsPlusSha256Robust',
      'Falcon512',
      'Falcon1024',
      'Ed25519Dilithium3',
      'K256Dilithium3',
    ];
    for (final curve in pqcCurves) {
      final kp = await deriveKeypairFromMnemonic(
        mnemonic: mnemonic,
        curveName: curve,
      );
      expect(kp.address, startsWith('0x'));
      expect(kp.curveType, contains(curve));
    }
  });

  test('seed derivation: all curves, deterministic, distinct', () async {
    const curves = [
      'K256',
      'P256',
      'Ed25519',
      'Dilithium2',
      'Dilithium3',
      'Dilithium5',
      'SphincsPlusSha256Robust',
      'Falcon512',
      'Falcon1024',
      'Ed25519Dilithium3',
      'K256Dilithium3',
    ];
    final seed = List<int>.generate(64, (i) => (i * 7 + 3) % 256);
    final addresses = <String>{};
    for (final curve in curves) {
      final kp = await deriveKeypairFromSeedApi(seed: seed, curveName: curve);
      expect(kp.address, startsWith('0x'));
      expect(kp.curveType, contains(curve));

      // Determinism: same seed reproduces the same keypair.
      final again = await deriveKeypairFromSeedApi(
        seed: seed,
        curveName: curve,
      );
      expect(again.address, kp.address);

      // Cross-curve distinctness.
      expect(addresses.add(kp.address), isTrue, reason: 'collision for $curve');
    }
  });

  test('seed derivation rejects short seeds', () async {
    expect(
      deriveKeypairFromSeedApi(
        seed: List<int>.filled(32, 1),
        curveName: 'Dilithium3',
      ),
      throwsException,
    );
  });

  test('zkLogin binding surface round-trips through the mock', () async {
    // NOTE: runs against MockRustApi (binding surface only).
    final prepared = await zkloginPrepareNonce(maxEpoch: BigInt.from(100));
    expect(prepared.ephemeralPubkey, hasLength(32));
    expect(prepared.nonce, isNotEmpty);

    final claims = await zkloginVerifyJwt(
      jwt: 'mock.jwt.token',
      jwksJson: '{}',
      expectedIss: 'https://accounts.google.com',
      expectedAud: 'mock-aud',
      nowSecs: BigInt.from(1_700_000_000),
    );
    expect(claims.sub, isNotEmpty);

    final salt = await zkloginDeterministicSalt(
      iss: claims.iss,
      aud: claims.aud,
      sub: claims.sub,
    );
    expect(salt, hasLength(32));

    final address = await zkloginDeriveAddress(
      iss: claims.iss,
      aud: claims.aud,
      sub: claims.sub,
      salt: salt,
    );
    expect(address, startsWith('ZkLogin:'));

    final sig = await zkloginSignEphemeral(
      secret: prepared.ephemeralSecret,
      message: 'hello'.codeUnits,
    );
    expect(
      await zkloginVerifyEphemeral(
        pubkey: prepared.ephemeralPubkey,
        message: 'hello'.codeUnits,
        signature: sig,
      ),
      isTrue,
    );

    final bundle = await zkloginBuildBundle(
      jwt: 'mock.jwt.token',
      jwksJson: '{}',
      iss: claims.iss,
      aud: claims.aud,
      salt: salt,
      randomness: prepared.randomness,
      ephemeralPubkey: prepared.ephemeralPubkey,
      ephemeralSig: sig,
      maxEpoch: prepared.maxEpoch,
    );
    expect(bundle, isNotEmpty);
  });
}
