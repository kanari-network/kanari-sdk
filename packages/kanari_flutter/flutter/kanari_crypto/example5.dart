import 'package:kanari_crypto/kanari_crypto.dart';

void main() async {
  // Initialize the library
  await RustLib.init();

  try {
    // 1. Generate a new random mnemonic
    print('--- Mnemonic Generation Example ---');
    final newMnemonic = await generateMnemonicApi(wordCount: BigInt.from(12));
    print('Generated 12-word Mnemonic: $newMnemonic');

    final newMnemonic24 = await generateMnemonicApi(wordCount: BigInt.from(24));
    print('Generated 24-word Mnemonic: $newMnemonic24');

    // 2. Mnemonic Derivation Example
    print('\n--- Mnemonic Derivation Example ---');
    const mnemonic =
        'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';
    print('Using Mnemonic: $mnemonic');

    // 3. HD Wallet Generation Example
    const pathTemplate = "m/44'/637'/0'/0/{index}";
    const curveName = 'P256';
    const count = 5;

    print('\n--- HD Wallet Generation Example ---');
    print('Path Template: $pathTemplate');
    print('Curve: $curveName');
    print('Generating $count wallets...');

    final keyPairs = await deriveMultipleAddressesApi(
      mnemonic: mnemonic,
      pathTemplate: pathTemplate,
      curveName: curveName,
      count: BigInt.from(count),
    );

    for (var i = 0; i < keyPairs.length; i++) {
      print('\nWallet #${i + 1}:');
      print('  Address: ${keyPairs[i].address}');
      print('  Private Key: ${keyPairs[i].privateKey}');
      print('  Curve: ${keyPairs[i].curveType}');
    }

    // 4. Path Derivation Example (Single Address)
    const derivationPath = "m/44'/637'/0'/0/1";
    print('\n--- Path Derivation Example (Single) ---');
    print('Derivation Path: $derivationPath');

    final pathKeyPair = await deriveKeypairFromPathApi(
      mnemonic: mnemonic,
      derivationPath: derivationPath,
      curveName: curveName,
    );

    print('\nPath Derivation Successful:');
    print('  Address: ${pathKeyPair.address}');
    print('  Private Key: ${pathKeyPair.privateKey}');
    print('  Curve: ${pathKeyPair.curveType}');

    // 5. Single Derivation Example
    const selectedCurve = 'P256';
    print('\n--- Single Derivation Example ---');
    print('Selected Curve: $selectedCurve');

    final result = await deriveKeypairFromMnemonic(
      mnemonic: mnemonic,
      curveName: selectedCurve,
    );

    print('\nDerivation Successful:');
    print('Address: ${result.address}');
    print('Public Key: ${result.publicKey}');
    print('Curve Type: ${result.curveType}');
    print('\nPrivate Key length: ${result.privateKey.length} characters');
  } catch (e) {
    print('Error during derivation: $e');
  }
}
