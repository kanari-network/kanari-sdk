import 'package:shared_preferences/shared_preferences.dart';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:kanari_crypto/kanari_crypto.dart';
import 'kanaricurve.dart';

class WalletStorage {
  static const _keyWallets = 'kanari_wallets_list';
  static const _keyActiveWallet = 'kanari_active_wallet';
  static const _keyPasswordHash = 'kanari_wallet_password_hash';
  
  // Legacy keys for migration
  static const _keyLegacyWallet = 'kanari_wallet_data';
  static const _keyLegacyPassword = 'kanari_wallet_password';

  /// Save a single wallet (legacy support)
  @Deprecated('Use saveWallet and savePassword instead')
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

  /// Save all wallets
  static Future<void> saveAllWallets(
    List<Map<String, dynamic>> wallets,
  ) async {
    final prefs = await SharedPreferences.getInstance();

    // Encrypt and store wallets list
    final walletsJson = jsonEncode(wallets.map((w) => w).toList());
    await prefs.setString(_keyWallets, walletsJson);
  }

  /// Save password hash
  static Future<void> savePassword(String password) async {
    final prefs = await SharedPreferences.getInstance();
    final passwordBytes = utf8.encode(password);
    final hash = await blake3HashApi(data: passwordBytes);
    final hashBase64 = base64Encode(hash);
    await prefs.setString(_keyPasswordHash, hashBase64);
    debugPrint('🔐 Password hash saved');
  }

  /// Load all wallets with migration support
  static Future<List<Map<String, dynamic>>> loadAllWallets() async {
    final prefs = await SharedPreferences.getInstance();
    
    // Try to load from new key first
    final walletsStr = prefs.getString(_keyWallets);
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
        final wallets = [migratedWallet];
        final walletsJson = jsonEncode(wallets);
        await prefs.setString(_keyWallets, walletsJson);
        
        // Migrate password hash if exists
        final oldPassword = prefs.getString(_keyLegacyPassword);
        if (oldPassword != null) {
          final passwordBytes = utf8.encode(oldPassword);
          final hash = await blake3HashApi(data: passwordBytes);
          final hashBase64 = base64Encode(hash);
          await prefs.setString(_keyPasswordHash, hashBase64);
          await prefs.remove(_keyLegacyPassword);
        }
        
        // Remove legacy key after successful migration
        await prefs.remove(_keyLegacyWallet);
        
        debugPrint('Successfully migrated legacy wallet to new format');
        return wallets;
      } catch (e) {
        debugPrint('Error migrating legacy wallet: $e');
        return [];
      }
    }

    return [];
  }

  /// Load wallet data if password matches
  static Future<Map<String, dynamic>?> loadWallet(String password) async {
    final prefs = await SharedPreferences.getInstance();
    final savedHashBase64 = prefs.getString(_keyPasswordHash);

    debugPrint('🔐 Loading wallet...');
    debugPrint('  - Password hash saved: ${savedHashBase64 != null ? "yes" : "no"}');
    
    if (savedHashBase64 == null) {
      debugPrint('  - No password hash, checking legacy...');
      final oldPassword = prefs.getString('kanari_wallet_password');
      if (oldPassword != null) {
        debugPrint('  - Legacy password found: ${oldPassword.length} chars');
        if (oldPassword != password) {
          debugPrint('  - ❌ Legacy password mismatch');
          return null;
        }
        final dataStr = prefs.getString(_keyWallets);
        if (dataStr != null) {
          final List<dynamic> walletsList = jsonDecode(dataStr);
          if (walletsList.isNotEmpty) {
            final data = walletsList.first as Map<String, dynamic>;
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
            await prefs.remove('kanari_wallet_password');
            debugPrint('  - ✅ Migrated from legacy format');
          }
        }
      } else {
        // No password hash and no legacy password
        // Check if wallets exist - if yes, allow access without password
        debugPrint('  - No password set, checking if wallets exist...');
        final wallets = await loadAllWallets();
        if (wallets.isNotEmpty) {
          debugPrint('  - ✅ Wallets found (${wallets.length}), allowing access without password');
          debugPrint('  - First wallet data: ${wallets.first}');
          
          // Return first wallet or active wallet
          final activeWalletId = prefs.getString(_keyActiveWallet);
          if (activeWalletId != null) {
            final wallet = wallets.cast<Map<String, dynamic>?>().firstWhere(
              (w) => w?['id'] == activeWalletId,
              orElse: () => wallets.first as Map<String, dynamic>,
            );
            debugPrint('  - Returning active wallet: ${(wallet as Map<String, dynamic>)['name']}');
            return wallet as Map<String, dynamic>;
          }
          
          final firstWallet = wallets.first as Map<String, dynamic>;
          debugPrint('  - Returning first wallet: ${firstWallet['name']}');
          return firstWallet;
        } else {
          debugPrint('  - ❌ No wallets and no password');
          return null;
        }
      }
    } else {
      debugPrint('  - Password hash found, validating...');
      final passwordBytes = utf8.encode(password);
      final hash = await blake3HashApi(data: passwordBytes);
      final hashBase64 = base64Encode(hash);

      debugPrint('  - Input hash: ${hashBase64.substring(0, 16)}...');
      debugPrint('  - Saved hash: ${savedHashBase64.substring(0, 16)}...');
      
      if (savedHashBase64 != hashBase64) {
        debugPrint('  - ❌ Password hash mismatch!');
        return null;
      }
      debugPrint('  - ✅ Password validated');
    }

    final activeWalletId = prefs.getString(_keyActiveWallet);
    final wallets = await loadAllWallets();

    debugPrint('  - Wallets in storage: ${wallets.length}');
    
    if (wallets.isEmpty) return null;

    // Return active wallet or first wallet
    if (activeWalletId != null) {
      final wallet = wallets.cast<Map<String, dynamic>?>().firstWhere(
        (w) => w?['id'] == activeWalletId,
        orElse: () => wallets.first as Map<String, dynamic>,
      );
      debugPrint('  - ✅ Returning wallet: ${(wallet as Map<String, dynamic>)['name']}');
      return wallet as Map<String, dynamic>;
    }

    final firstWallet = wallets.first as Map<String, dynamic>;
    debugPrint('  - ✅ Returning first wallet: ${firstWallet['name']}');
    return firstWallet;
  }

  /// Check if a wallet is already saved
  static Future<bool> hasWallet() async {
    final wallets = await loadAllWallets();
    return wallets.isNotEmpty;
  }

  /// Clear saved wallet
  static Future<void> deleteWallet() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(_keyWallets);
    await prefs.remove(_keyActiveWallet);
    await prefs.remove(_keyPasswordHash);
    await prefs.remove('kanari_wallet_password');
  }

  /// Delete specific wallet by ID
  static Future<void> deleteWalletById(String walletId) async {
    final wallets = await loadAllWallets();
    wallets.removeWhere((w) => w['id'] == walletId);
    await saveAllWallets(wallets);

    // Clear active wallet if it was deleted
    final prefs = await SharedPreferences.getInstance();
    final activeId = prefs.getString(_keyActiveWallet);
    if (activeId == walletId) {
      await prefs.remove(_keyActiveWallet);
    }
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

  /// Get wallet count
  static Future<int> getWalletCount() async {
    final wallets = await loadAllWallets();
    return wallets.length;
  }
}
