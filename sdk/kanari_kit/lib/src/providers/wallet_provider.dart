import 'package:flutter/material.dart';
import 'package:kanari_kit/kanari_kit.dart';

class WalletState extends ChangeNotifier {
  KanariClient? _client;
  KanariWallet? _wallet;
  int _balance = 0;
  bool _isLoading = false;
  String? _error;
  bool _hasSavedWallet = false;
  KanariEnvironment _environment = KanariEnvironment.local;

  KanariClient? get client => _client;
  KanariWallet? get wallet => _wallet;
  int get balance => _balance;
  bool get isLoading => _isLoading;
  String? get error => _error;
  bool get hasSavedWallet => _hasSavedWallet;
  KanariEnvironment get environment => _environment;

  Future<void> initialize() async {
    _updateClient();
    _hasSavedWallet = await WalletStorage.hasWallet();
    notifyListeners();
  }

  void _updateClient() {
    _client = KanariClient.fromEnvironment(_environment);
  }

  Future<void> setEnvironment(KanariEnvironment env) async {
    if (_environment == env) return;
    _environment = env;
    _updateClient();
    if (_wallet != null) {
      await refreshBalance();
    }
    notifyListeners();
  }

  Future<void> createNewWallet({
    KanariCurve curve = KanariCurve.ed25519,
    required String password,
    bool rememberMe = true,
  }) async {
    _setLoading(true);
    _error = null;
    try {
      debugPrint("Starting wallet generation with curve: ${curve.name}...");
      _wallet = await KanariWallet.generate(curve: curve);
      debugPrint("Wallet generated: ${_wallet?.address}");

      if (rememberMe && _wallet != null) {
        await WalletStorage.saveWallet(
          mnemonic: _wallet!.mnemonic,
          privateKey: _wallet!.privateKey,
          curve: curve,
          password: password,
        );
        _hasSavedWallet = true;
      }

      await refreshBalance();
    } catch (e, stack) {
      _error = "Creation failed: $e";
      debugPrint(_error);
      debugPrint(stack.toString());
    } finally {
      _setLoading(false);
    }
  }

  Future<void> unlockWallet(String password) async {
    _setLoading(true);
    _error = null;
    try {
      final data = await WalletStorage.loadWallet(password);
      if (data == null) {
        _error = "Invalid password or no saved wallet";
        return;
      }

      final curve = KanariCurve.values.firstWhere(
        (c) => c.name == data['curve'],
        orElse: () => KanariCurve.ed25519,
      );

      if (data['mnemonic'] != null &&
          data['mnemonic'].toString().isNotEmpty &&
          !curve.isPostQuantum) {
        _wallet = await KanariWallet.fromMnemonic(
          data['mnemonic'],
          curve: curve,
        );
      } else {
        _wallet = await KanariWallet.fromPrivateKey(
          data['privateKey'],
          curve: curve,
        );
      }

      await refreshBalance();
    } catch (e) {
      _error = "Unlock failed: $e";
    } finally {
      _setLoading(false);
    }
  }

  Future<void> importFromPrivateKey(
    String pk, {
    KanariCurve curve = KanariCurve.ed25519,
    String? password,
  }) async {
    _setLoading(true);
    _error = null;
    try {
      String cleanPk = pk.trim();
      _wallet = await KanariWallet.fromPrivateKey(cleanPk, curve: curve);

      if (password != null && _wallet != null) {
        await WalletStorage.saveWallet(
          mnemonic: '', // No mnemonic for PK import
          privateKey: _wallet!.privateKey,
          curve: curve,
          password: password,
        );
        _hasSavedWallet = true;
      }

      await refreshBalance();
    } catch (e) {
      _error = "Import PK failed: $e";
      debugPrint(_error);
    } finally {
      _setLoading(false);
    }
  }

  void logout() {
    _wallet = null;
    _balance = 0;
    _error = null;
    notifyListeners();
  }

  Future<void> deleteSavedWallet() async {
    await WalletStorage.deleteWallet();
    _hasSavedWallet = false;
    logout();
  }

  Future<void> importFromMnemonic(
    String mnemonic, {
    KanariCurve curve = KanariCurve.ed25519,
    String? password,
  }) async {
    _setLoading(true);
    _error = null;
    try {
      _wallet = await KanariWallet.fromMnemonic(mnemonic, curve: curve);

      if (password != null && _wallet != null) {
        await WalletStorage.saveWallet(
          mnemonic: mnemonic,
          privateKey: _wallet!.privateKey,
          curve: curve,
          password: password,
        );
        _hasSavedWallet = true;
      }

      await refreshBalance();
    } catch (e) {
      _error = "Import Mnemonic failed: $e";
      debugPrint(_error);
    } finally {
      _setLoading(false);
    }
  }

  Future<void> refreshBalance() async {
    if (_client != null && _wallet != null) {
      try {
        _balance = await _client!.getBalance(_wallet!.address);
        _error = null; // Clear error on success
        notifyListeners();
      } catch (e) {
        _error = "Refresh balance failed: $e";
        debugPrint(_error); // Log to console instead of just keeping in state
        notifyListeners();
      }
    }
  }

  Future<String?> transfer(String recipient, int amount) async {
    if (_client == null || _wallet == null) return "Client not initialized";
    _setLoading(true);
    try {
      final result = await _client!.transfer(
        wallet: _wallet!,
        recipient: recipient,
        amount: amount,
      );
      await refreshBalance();
      _setLoading(false);
      return "Success: Hash ${result.hash}";
    } catch (e) {
      _setLoading(false);
      return "Error: $e";
    }
  }

  Future<String?> executeFunction({
    required String packageAddress,
    required String module,
    required String function,
    List<String> typeArgs = const [],
    List<List<int>> args = const [],
  }) async {
    if (_client == null || _wallet == null) return "Client not initialized";
    _setLoading(true);
    try {
      final result = await _client!.executeFunction(
        wallet: _wallet!,
        package: packageAddress,
        module: module,
        function: function,
        typeArgs: typeArgs,
        args: args,
      );
      await refreshBalance();
      _setLoading(false);
      return "Success: Hash ${result.hash}";
    } catch (e) {
      _setLoading(false);
      return "Error: $e";
    }
  }

  Future<String?> burn(int amount) async {
    if (_client == null || _wallet == null) return "Client not initialized";
    _setLoading(true);
    try {
      final result = await _client!.burn(wallet: _wallet!, amount: amount);
      await refreshBalance();
      _setLoading(false);
      return "Success: Hash ${result.hash}";
    } catch (e) {
      _setLoading(false);
      return "Error: $e";
    }
  }

  void _setLoading(bool value) {
    _isLoading = value;
    notifyListeners();
  }
}
