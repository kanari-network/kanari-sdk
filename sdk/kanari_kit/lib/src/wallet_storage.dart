import 'package:shared_preferences/shared_preferences.dart';
import 'dart:convert';
import 'kanaricurve.dart';

class WalletStorage {
  static const _keyWallet = 'kanari_wallet_data';
  static const _keyPassword = 'kanari_wallet_password';

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
    
    await prefs.setString(_keyWallet, jsonEncode(data));
    await prefs.setString(_keyPassword, password);
  }

  /// Load wallet data if password matches
  static Future<Map<String, dynamic>?> loadWallet(String password) async {
    final prefs = await SharedPreferences.getInstance();
    final savedPassword = prefs.getString(_keyPassword);
    if (savedPassword == null || savedPassword != password) {
      return null;
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
    await prefs.remove(_keyPassword);
  }
}
