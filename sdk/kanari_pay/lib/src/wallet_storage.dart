import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'package:cryptography/cryptography.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:kanari_crypto/kanari_crypto.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'models/account.dart';

class WalletStorage {
  static const _keyWalletData = 'kanari_wallet_data';
  static const _keyActiveWallet = 'kanari_active_wallet';
  static const _keyPasswordHash = 'kanari_wallet_password_hash';
  static const _keyPasswordSalt = 'kanari_wallet_password_salt';
  static const _keyPasswordVerifier = 'kanari_wallet_password_verifier';
  static const _keyFailedPinAttempts = 'kanari_failed_pin_attempts';
  static const _keyPinLockedUntil = 'kanari_pin_locked_until';
  static const _keyBiometricEnabled = 'kanari_biometric_enabled';
  static const _keyBiometricUnlockPin = 'kanari_biometric_unlock_pin';
  static const _keyBalanceCache = 'kanari_wallet_balance_cache';
  static const _pinLength = 6;
  static const _kdfIterations = 210000;
  static const _maxFailedAttempts = 5;
  static const _lockoutDuration = Duration(seconds: 30);

  static final _pbkdf2 = Pbkdf2(
    macAlgorithm: Hmac.sha256(),
    iterations: _kdfIterations,
    bits: 256,
  );
  static final _aesGcm = AesGcm.with256bits();
  static final _random = Random.secure();
  static final _biometricStorage = FlutterSecureStorage(
    aOptions: const AndroidOptions.biometric(
      enforceBiometrics: true,
      biometricType: AndroidBiometricType.strongBiometricOnly,
      storageNamespace: 'kanari_biometric_unlock',
      biometricPromptTitle: 'Unlock Kanari Wallet',
      biometricPromptSubtitle: 'Authenticate to unlock your wallet',
      biometricPromptNegativeButton: 'Cancel',
    ),
    iOptions: const IOSOptions(
      accessibility: KeychainAccessibility.unlocked,
      accessControlFlags: [AccessControlFlag.biometryCurrentSet],
      useSecureEnclave: true,
    ),
  );

  static Future<void> saveAllWallets(
    List<Map<String, dynamic>> wallets, {
    String? pin,
  }) async {
    final prefs = await SharedPreferences.getInstance();
    final encryptedWallets = await _encryptedWalletRecords(wallets, pin: pin);

    await prefs.setString(_keyWalletData, jsonEncode(encryptedWallets));
    debugPrint('Saved ${encryptedWallets.length} encrypted wallet records');
  }

  static Future<void> savePassword(String password) async {
    _assertValidPin(password);

    final prefs = await SharedPreferences.getInstance();
    final salt = _randomBytes(16);
    final verifier = await _derivePinVerifier(password, salt);

    await prefs.setString(_keyPasswordSalt, base64Encode(salt));
    await prefs.setString(_keyPasswordVerifier, base64Encode(verifier));
    await prefs.remove(_keyPasswordHash);
    await _resetPinFailures(prefs);
    await _syncBiometricUnlockPinIfEnabled(password);
    debugPrint('Master PIN verifier saved');
  }

  static Future<void> savePasswordAndWallets(
    String password,
    List<Map<String, dynamic>> wallets,
  ) async {
    _assertValidPin(password);

    final prefs = await SharedPreferences.getInstance();
    final encryptedWallets = await _encryptedWalletRecords(
      wallets,
      pin: password,
    );
    final salt = _randomBytes(16);
    final verifier = await _derivePinVerifier(password, salt);

    await prefs.setString(_keyWalletData, jsonEncode(encryptedWallets));
    await prefs.setString(_keyPasswordSalt, base64Encode(salt));
    await prefs.setString(_keyPasswordVerifier, base64Encode(verifier));
    await prefs.remove(_keyPasswordHash);
    await _resetPinFailures(prefs);
    await _syncBiometricUnlockPinIfEnabled(password);
    debugPrint('Master PIN verifier and encrypted wallets saved');
  }

  static Future<bool> verifyPassword(String password) async {
    if (!_isValidPin(password)) return false;

    final prefs = await SharedPreferences.getInstance();
    if (await _isLockedOut(prefs)) return false;

    final saltBase64 = prefs.getString(_keyPasswordSalt);
    final verifierBase64 = prefs.getString(_keyPasswordVerifier);

    if (saltBase64 != null && verifierBase64 != null) {
      final verifier = base64Decode(verifierBase64);
      final candidate = await _derivePinVerifier(
        password,
        base64Decode(saltBase64),
      );
      final success = _constantTimeEquals(candidate, verifier);

      if (success) {
        await _resetPinFailures(prefs);
        await _syncBiometricUnlockPinIfEnabled(password);
      } else {
        await _recordFailedPin(prefs);
      }

      return success;
    }

    final legacyHash = prefs.getString(_keyPasswordHash);
    if (legacyHash == null) {
      await _recordFailedPin(prefs);
      return false;
    }

    final legacyBytes = utf8.encode(password);
    final legacyHashBase64 = base64Encode(
      await blake3HashApi(data: legacyBytes),
    );
    final success = _constantTimeEquals(
      utf8.encode(legacyHashBase64),
      utf8.encode(legacyHash),
    );

    if (success) {
      await savePassword(password);
    } else {
      await _recordFailedPin(prefs);
    }

    return success;
  }

  static Future<bool> hasPassword() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString(_keyPasswordVerifier) != null ||
        prefs.getString(_keyPasswordHash) != null;
  }

  static Future<Duration?> pinLockRemaining() async {
    final prefs = await SharedPreferences.getInstance();
    final lockedUntil = prefs.getInt(_keyPinLockedUntil);
    if (lockedUntil == null) return null;

    final remaining = DateTime.fromMillisecondsSinceEpoch(
      lockedUntil,
    ).difference(DateTime.now());
    return remaining.isNegative ? null : remaining;
  }

  static Future<bool> isBiometricEnabled() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getBool(_keyBiometricEnabled) ?? false;
  }

  static Future<void> enableBiometricUnlock(String pin) async {
    _assertValidPin(pin);
    await _writeBiometricUnlockPin(pin);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_keyBiometricEnabled, true);
  }

  static Future<void> disableBiometricUnlock() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_keyBiometricEnabled, false);
    await deleteBiometricUnlockPin();
  }

  static Future<String?> readBiometricUnlockPin() async {
    if (!await isBiometricEnabled()) return null;
    if (!_supportsPersistentBiometricUnlock()) return null;

    try {
      return await _biometricStorage.read(key: _keyBiometricUnlockPin);
    } catch (e) {
      debugPrint('Failed to read biometric unlock PIN: $e');
      return null;
    }
  }

  static Future<void> deleteBiometricUnlockPin() async {
    try {
      await _biometricStorage.delete(key: _keyBiometricUnlockPin);
    } catch (e) {
      debugPrint('Failed to delete biometric unlock PIN: $e');
    }
  }

  static Future<List<Map<String, dynamic>>> loadAllWallets({
    String? pin,
  }) async {
    final prefs = await SharedPreferences.getInstance();
    final walletsStr = prefs.getString(_keyWalletData);
    if (walletsStr == null) return [];

    try {
      final List<dynamic> walletsList = jsonDecode(walletsStr);
      final wallets = walletsList
          .map((wallet) => Map<String, dynamic>.from(wallet as Map))
          .toList();

      if (pin == null) {
        return wallets.map(_publicWalletRecord).toList();
      }

      return Future.wait(
        wallets.map((wallet) => _walletForRuntime(wallet, pin: pin)),
      );
    } catch (e) {
      debugPrint('Error loading wallets: $e');
      return [];
    }
  }

  static Future<Map<String, dynamic>?> loadWalletById(
    String walletId, {
    required String pin,
  }) async {
    final wallets = await loadAllWallets(pin: pin);
    for (final wallet in wallets) {
      if (wallet['id'] == walletId) {
        return wallet;
      }
    }
    return null;
  }

  static Future<void> setActiveWallet(String walletId) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_keyActiveWallet, walletId);
  }

  static Future<String?> getActiveWalletId() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString(_keyActiveWallet);
  }

  static Future<void> saveCachedBalances(
    String walletAddress,
    List<TokenBalance> balances,
  ) async {
    final prefs = await SharedPreferences.getInstance();
    final raw = prefs.getString(_keyBalanceCache);
    final cache = raw == null
        ? <String, dynamic>{}
        : Map<String, dynamic>.from(jsonDecode(raw) as Map);
    cache[walletAddress] = balances.map((item) => item.toJson()).toList();
    await prefs.setString(_keyBalanceCache, jsonEncode(cache));
  }

  static Future<List<TokenBalance>> loadCachedBalances(
    String walletAddress,
  ) async {
    final prefs = await SharedPreferences.getInstance();
    final raw = prefs.getString(_keyBalanceCache);
    if (raw == null) return const [];

    try {
      final cache = Map<String, dynamic>.from(jsonDecode(raw) as Map);
      final entries = cache[walletAddress];
      if (entries is! List) return const [];

      return entries
          .map(
            (item) =>
                TokenBalance.fromJson(Map<String, dynamic>.from(item as Map)),
          )
          .toList();
    } catch (e) {
      debugPrint('Failed to load cached balances: $e');
      return const [];
    }
  }

  static Future<void> clearCachedBalances([String? walletAddress]) async {
    final prefs = await SharedPreferences.getInstance();
    if (walletAddress == null) {
      await prefs.remove(_keyBalanceCache);
      return;
    }

    final raw = prefs.getString(_keyBalanceCache);
    if (raw == null) return;

    try {
      final cache = Map<String, dynamic>.from(jsonDecode(raw) as Map);
      cache.remove(walletAddress);
      if (cache.isEmpty) {
        await prefs.remove(_keyBalanceCache);
      } else {
        await prefs.setString(_keyBalanceCache, jsonEncode(cache));
      }
    } catch (e) {
      debugPrint('Failed to clear cached balances: $e');
    }
  }

  static Future<void> deleteWalletById(String walletId, {String? pin}) async {
    final wallets = await loadAllWallets(pin: pin);
    wallets.removeWhere((wallet) => wallet['id'] == walletId);
    await saveAllWallets(wallets, pin: pin);
  }

  static Future<void> deleteAllWallets() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(_keyWalletData);
    await prefs.remove(_keyActiveWallet);
    await prefs.remove(_keyPasswordHash);
    await prefs.remove(_keyPasswordSalt);
    await prefs.remove(_keyPasswordVerifier);
    await prefs.remove(_keyBiometricEnabled);
    await _resetPinFailures(prefs);
    await deleteBiometricUnlockPin();
    await prefs.remove(_keyBalanceCache);
    debugPrint('All wallets and PIN data deleted');
  }

  static Future<int> getWalletCount() async {
    final wallets = await loadAllWallets();
    return wallets.length;
  }

  static Future<Map<String, dynamic>> _walletForStorage(
    Map<String, dynamic> wallet, {
    required String? pin,
  }) async {
    final output = Map<String, dynamic>.from(wallet);
    final privateKey = output.remove('privateKey')?.toString();
    final mnemonic = output.remove('mnemonic')?.toString();
    final alreadyEncrypted = output['privateKeyEncrypted'] != null;

    if (alreadyEncrypted && privateKey == null && mnemonic == null) {
      return output;
    }

    if (pin == null || !_isValidPin(pin)) {
      throw StateError('PIN is required to encrypt wallet secrets');
    }

    if (privateKey != null && privateKey.isNotEmpty) {
      output['privateKeyEncrypted'] = await _encryptText(privateKey, pin);
    }

    if (mnemonic != null && mnemonic.isNotEmpty) {
      output['mnemonicEncrypted'] = await _encryptText(mnemonic, pin);
    }

    output['encryption'] = 'pin_aes_gcm_pbkdf2_v1';
    return output;
  }

  static Future<List<Map<String, dynamic>>> _encryptedWalletRecords(
    List<Map<String, dynamic>> wallets, {
    required String? pin,
  }) {
    return Future.wait(
      wallets.map((wallet) => _walletForStorage(wallet, pin: pin)),
    );
  }

  static Future<Map<String, dynamic>> _walletForRuntime(
    Map<String, dynamic> wallet, {
    required String pin,
  }) async {
    final output = Map<String, dynamic>.from(wallet);
    final privateKeyEncrypted = output.remove('privateKeyEncrypted');
    final mnemonicEncrypted = output.remove('mnemonicEncrypted');

    if (privateKeyEncrypted != null) {
      output['privateKey'] = await _decryptText(
        Map<String, dynamic>.from(privateKeyEncrypted as Map),
        pin,
      );
    }

    if (mnemonicEncrypted != null) {
      output['mnemonic'] = await _decryptText(
        Map<String, dynamic>.from(mnemonicEncrypted as Map),
        pin,
      );
    }

    return output;
  }

  static Map<String, dynamic> _publicWalletRecord(Map<String, dynamic> wallet) {
    final output = Map<String, dynamic>.from(wallet);
    output.remove('privateKey');
    output.remove('mnemonic');
    return output;
  }

  static Future<Map<String, dynamic>> _encryptText(
    String value,
    String pin,
  ) async {
    final salt = _randomBytes(16);
    final nonce = _randomBytes(12);
    final secretBox = await _aesGcm.encrypt(
      utf8.encode(value),
      secretKey: await _deriveEncryptionKey(pin, salt),
      nonce: nonce,
    );

    return {
      'salt': base64Encode(salt),
      'nonce': base64Encode(secretBox.nonce),
      'cipherText': base64Encode(secretBox.cipherText),
      'mac': base64Encode(secretBox.mac.bytes),
      'iterations': _kdfIterations,
      'algorithm': 'aes_gcm_256_pbkdf2_sha256',
    };
  }

  static Future<String> _decryptText(
    Map<String, dynamic> encrypted,
    String pin,
  ) async {
    final salt = base64Decode(encrypted['salt'] as String);
    final secretBox = SecretBox(
      base64Decode(encrypted['cipherText'] as String),
      nonce: base64Decode(encrypted['nonce'] as String),
      mac: Mac(base64Decode(encrypted['mac'] as String)),
    );

    final clearBytes = await _aesGcm.decrypt(
      secretBox,
      secretKey: await _deriveEncryptionKey(pin, salt),
    );
    return utf8.decode(clearBytes);
  }

  static Future<SecretKey> _deriveEncryptionKey(String pin, List<int> salt) {
    return _pbkdf2.deriveKey(
      secretKey: SecretKey(utf8.encode(pin)),
      nonce: salt,
    );
  }

  static Future<List<int>> _derivePinVerifier(
    String pin,
    List<int> salt,
  ) async {
    final key = await _pbkdf2.deriveKey(
      secretKey: SecretKey(utf8.encode(pin)),
      nonce: salt,
    );
    return key.extractBytes();
  }

  static bool _isValidPin(String pin) {
    return RegExp('^\\d{$_pinLength}\$').hasMatch(pin);
  }

  static void _assertValidPin(String pin) {
    if (!_isValidPin(pin)) {
      throw ArgumentError('PIN must be exactly $_pinLength digits');
    }
  }

  static List<int> _randomBytes(int length) {
    return List<int>.generate(length, (_) => _random.nextInt(256));
  }

  static bool _constantTimeEquals(List<int> a, List<int> b) {
    if (a.length != b.length) return false;

    var diff = 0;
    for (var i = 0; i < a.length; i++) {
      diff |= a[i] ^ b[i];
    }
    return diff == 0;
  }

  static Future<bool> _isLockedOut(SharedPreferences prefs) async {
    final lockedUntil = prefs.getInt(_keyPinLockedUntil);
    if (lockedUntil == null) return false;

    if (DateTime.now().millisecondsSinceEpoch < lockedUntil) {
      return true;
    }

    await _resetPinFailures(prefs);
    return false;
  }

  static Future<void> _recordFailedPin(SharedPreferences prefs) async {
    final attempts = prefs.getInt(_keyFailedPinAttempts) ?? 0;
    final nextAttempts = attempts + 1;
    await prefs.setInt(_keyFailedPinAttempts, nextAttempts);

    if (nextAttempts >= _maxFailedAttempts) {
      await prefs.setInt(
        _keyPinLockedUntil,
        DateTime.now().add(_lockoutDuration).millisecondsSinceEpoch,
      );
    }
  }

  static Future<void> _resetPinFailures(SharedPreferences prefs) async {
    await prefs.remove(_keyFailedPinAttempts);
    await prefs.remove(_keyPinLockedUntil);
  }

  static Future<void> _syncBiometricUnlockPinIfEnabled(String pin) async {
    if (!await isBiometricEnabled()) return;
    if (!_supportsPersistentBiometricUnlock()) return;

    try {
      await _writeBiometricUnlockPin(pin);
    } catch (e) {
      debugPrint('Failed to sync biometric unlock PIN: $e');
    }
  }

  static Future<void> _writeBiometricUnlockPin(String pin) async {
    if (!_supportsPersistentBiometricUnlock()) {
      throw UnsupportedError(
        'Persistent biometric unlock is only available on Android, iOS, and macOS.',
      );
    }

    await _biometricStorage.write(key: _keyBiometricUnlockPin, value: pin);
  }

  static bool _supportsPersistentBiometricUnlock() {
    return !kIsWeb &&
        (Platform.isAndroid || Platform.isIOS || Platform.isMacOS);
  }
}
