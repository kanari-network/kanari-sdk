import 'package:shared_preferences/shared_preferences.dart';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:kanari_crypto/kanari_crypto.dart';
import 'kanaricurve.dart';

class WalletStorage {
  static const _keyWalletData = 'kanari_wallet_data';
  static const _keyActiveWallet = 'kanari_active_wallet';
  static const _keyPasswordHash = 'kanari_wallet_password_hash';
  
  // Legacy keys for migration
  static const _keyLegacyWallet = 'kanari_wallet_data';
  static const _keyLegacyPassword = 'kanari_wallet_password';

  /// Save a single wallet (legacy support)
  @Deprecated('Use saveAllWallets and savePassword instead')
  static Future<void> saveWallet({
    String? mnemonic,
    required String privateKey,
    required KanariCurve curve,
    required String password,
  }) async {
    final wallets = await loadAllWallets();
    final newWallet = {
      'id': DateTime.now().millisecondsSinceEpoch.toString(),
      'name': 'Wallet ${wallets.length + 1}',
      'mnemonic': mnemonic,
      'privateKey': privateKey,
      'curve': curve.name,
      'createdAt': DateTime.now().toIso8601String(),
    };
    wallets.add(newWallet);
    await saveAllWallets(wallets);
    await savePassword(password);
  }

  /// Save all wallets with a single master password
  static Future<void> saveAllWallets(List<Map<String, dynamic>> wallets) async {
    final prefs = await SharedPreferences.getInstance();
    final walletsJson = jsonEncode(wallets);
    await prefs.setString(_keyWalletData, walletsJson);
    debugPrint('💾 Saved ${wallets.length} wallets');
  }

  /// Save password hash (master password for all wallets)
  static Future<void> savePassword(String password) async {
    final prefs = await SharedPreferences.getInstance();
    final passwordBytes = utf8.encode(password);
    final hash = await blake3HashApi(data: passwordBytes);
    final hashBase64 = base64Encode(hash);
    await prefs.setString(_keyPasswordHash, hashBase64);
    debugPrint('🔐 Master password hash saved');
  }

  /// Verify password against stored hash
  static Future<bool> verifyPassword(String password) async {
    final prefs = await SharedPreferences.getInstance();
    final storedHash = prefs.getString(_keyPasswordHash);
    
    if (storedHash == null) {
      // No password set (legacy mode)
      return true;
    }
    
    final passwordBytes = utf8.encode(password);
    final hash = await blake3HashApi(data: passwordBytes);
    final hashBase64 = base64Encode(hash);
    
    return hashBase64 == storedHash;
  }

  /// Load all wallets with migration support
  static Future<List<Map<String, dynamic>>> loadAllWallets() async {
    final prefs = await SharedPreferences.getInstance();
    
    // Try to load from new key first
    final walletsStr = prefs.getString(_keyWalletData);
    if (walletsStr != null) {
      try {
        final List<dynamic> walletsList = jsonDecode(walletsStr);
        return walletsList.map((w) => w as Map<String, dynamic>).toList();
      } catch (e) {
        debugPrint('Error loading wallets: $e');
        return [];
      }
    }

    // Migration: Check for legacy single wallet data
    final legacyDataStr = prefs.getString(_keyLegacyWallet);
    if (legacyDataStr != null) {
      try {
        final legacyData = jsonDecode(legacyDataStr) as Map<String, dynamic>;
        
        // Migrate to new format
        final migratedWallet = {
          'id': DateTime.now().millisecondsSinceEpoch.toString(),
          'name': 'Migrated Wallet',
          'mnemonic': legacyData['mnemonic'],
          'privateKey': legacyData['privateKey'],
          'curve': legacyData['curve'],
          'createdAt': DateTime.now().toIso8601String(),
        };
        
        // Save in new format
        await saveAllWallets([migratedWallet]);
        
        // Clear legacy data
        await prefs.remove(_keyLegacyWallet);
        
        debugPrint('✅ Migrated legacy wallet to new format');
        return [migratedWallet];
      } catch (e) {
        debugPrint('Error migrating legacy wallet: $e');
        return [];
      }
    }

    return [];
  }

  /// Set active wallet
  static Future<void> setActiveWallet(String walletId) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_keyActiveWallet, walletId);
  }

  /// Get active wallet ID
  static Future<String?> getActiveWalletId() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString(_keyActiveWallet);
  }

  /// Delete wallet by ID
  static Future<void> deleteWalletById(String walletId) async {
    final wallets = await loadAllWallets();
    wallets.removeWhere((w) => w['id'] == walletId);
    await saveAllWallets(wallets);
  }

  /// Delete all wallets
  static Future<void> deleteAllWallets() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(_keyWalletData);
    await prefs.remove(_keyActiveWallet);
    debugPrint('🗑️ All wallets deleted');
  }

  /// Get wallet count
  static Future<int> getWalletCount() async {
    final wallets = await loadAllWallets();
    return wallets.length;
  }
}