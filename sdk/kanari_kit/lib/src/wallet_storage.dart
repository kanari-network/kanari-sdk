import 'package:shared_preferences/shared_preferences.dart';
import 'dart:convert';
import 'package:kanari_crypto/kanari_crypto.dart';
import 'kanaricurve.dart';

class WalletStorage {
  static const _keyWallet = 'kanari_wallet_data';
  static const _keyPasswordHash = 'kanari_wallet_password_hash';

  /// Save wallet data
  static Future<void> saveWallet({
    String? mnemonic,
    required String privateKey,
    required KanariCurve curve,
    required String password,
  }) async {
    final prefs = await SharedPreferences.getInstance();
    final data = {
      'mnemonic': mnemonic,
      'privateKey': privateKey,
      'curve': curve.name,
    };

    // Store a hash of the password instead of plain text for verification
    final passwordBytes = utf8.encode(password);
    final hash = await blake3HashApi(data: passwordBytes);
    final hashBase64 = base64Encode(hash);

    await prefs.setString(_keyWallet, jsonEncode(data));
    await prefs.setString(_keyPasswordHash, hashBase64);
  }

  /// Load wallet data if password matches
  static Future<Map<String, dynamic>?> loadWallet(String password) async {
    final prefs = await SharedPreferences.getInstance();
    final savedHashBase64 = prefs.getString(_keyPasswordHash);
    
    // Support legacy plain text password for migration if needed
    // But for now, let's assume we want to be secure
    if (savedHashBase64 == null) {
      // Check if old plain text password exists
      final oldPassword = prefs.getString('kanari_wallet_password');
      if (oldPassword != null) {
        if (oldPassword != password) return null;
        // Migrate to hash
        final dataStr = prefs.getString(_keyWallet);
        if (dataStr != null) {
          final data = jsonDecode(dataStr) as Map<String, dynamic>;
          final curve = KanariCurve.values.firstWhere(
            (c) => c.name == data['curve'],
            orElse: () => KanariCurve.ed25519,
          );
          await saveWallet(
            mnemonic: data['mnemonic'],
            privateKey: data['privateKey'],
            curve: curve,
            password: password,
          );
          // Remove old password
          await prefs.remove('kanari_wallet_password');
        }
      } else {
        return null;
      }
    } else {
      final passwordBytes = utf8.encode(password);
      final hash = await blake3HashApi(data: passwordBytes);
      final hashBase64 = base64Encode(hash);

      if (savedHashBase64 != hashBase64) {
        return null;
      }
    }

    final dataStr = prefs.getString(_keyWallet);
    if (dataStr == null) return null;

    return jsonDecode(dataStr) as Map<String, dynamic>;
  }

  /// Check if a wallet is already saved
  static Future<bool> hasWallet() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.containsKey(_keyWallet);
  }

  /// Clear saved wallet
  static Future<void> deleteWallet() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(_keyWallet);
    await prefs.remove(_keyPasswordHash);
    await prefs.remove('kanari_wallet_password'); // Also clear legacy if exists
  }
}
