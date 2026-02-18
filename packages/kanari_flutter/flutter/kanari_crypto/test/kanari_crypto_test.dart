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
}
