import 'package:flutter/material.dart';
import 'package:kanari_pay/kanari_pay.dart';
import '../core/token_utils.dart' as token_utils;

class WalletState extends ChangeNotifier {
  static const String kanariTokenType = token_utils.kanariTokenType;

  KanariClient? _client;
  KanariWallet? _wallet;
  List<Map<String, dynamic>> _wallets = [];
  List<TokenBalance> _tokenBalances = [];

  bool _isLoading = false;
  String? _error;
  String? _activeWalletId;
  String? _authenticatedWalletId;
  KanariEnvironment _environment = KanariEnvironment.dev;
  bool _isUnlocked = false;

  KanariClient? get client => _client;
  KanariWallet? get wallet => _wallet;
  List<Map<String, dynamic>> get wallets => _wallets;
  List<TokenBalance> get tokenBalances => _tokenBalances;
  bool get isLoading => _isLoading;
  String? get error => _error;
  String? get activeWalletId => _activeWalletId;
  String? get authenticatedWalletId => _authenticatedWalletId;
  bool get hasWallet => _wallets.isNotEmpty;
  KanariEnvironment get environment => _environment;
  bool get isUnlocked => _isUnlocked;

  TokenBalance? get kanariTokenBalance {
    for (final token in _tokenBalances) {
      if (token_utils.isKanariToken(token)) {
        return token;
      }
    }
    return null;
  }

  int get kanariBalance => kanariTokenBalance?.amount ?? 0;

  Future<void> initialize() async {
    _updateClient();
    _wallets = await WalletStorage.loadAllWallets();
    notifyListeners();
  }

  Future<bool> syncWalletWithAddress(String walletAddress) async {
    final normalizedTarget = _normalizeWalletAddress(walletAddress);
    _wallets = await WalletStorage.loadAllWallets();
    _authenticatedWalletId = null;

    for (final walletData in _wallets) {
      try {
        final candidateWallet = await _walletFromData(walletData);
        if (_normalizeWalletAddress(candidateWallet.address) !=
            normalizedTarget) {
          continue;
        }

        await WalletStorage.setActiveWallet(walletData['id']);
        _activeWalletId = walletData['id'];
        _authenticatedWalletId = walletData['id'];
        _wallet = candidateWallet;
        _isUnlocked = true;
        await refreshBalance();
        notifyListeners();
        return true;
      } catch (e) {
        debugPrint('Failed to compare wallet ${walletData['id']}: $e');
      }
    }

    notifyListeners();
    return false;
  }

  void _updateClient() {
    _client = KanariClient.fromEnvironment(_environment);
  }

  Future<void> setEnvironment(KanariEnvironment env) async {
    if (_environment == env) return;

    _environment = env;
    _updateClient();

    if (_wallet == null) {
      notifyListeners();
      return;
    }

    await _withLoading(() async {
      _clearBalances(notify: true);
      await refreshBalance(notifyListenersOnSuccess: false);
    });
  }

  Future<void> loadWallets() async {
    _wallets = await WalletStorage.loadAllWallets();
    if (_wallets.isNotEmpty) {
      await _loadActiveWallet();
    }
  }

  Future<void> _loadActiveWallet() async {
    final activeId = await WalletStorage.getActiveWalletId();
    _activeWalletId = activeId;
    Map<String, dynamic>? activeWalletData;

    if (activeId != null) {
      activeWalletData = _wallets.cast<Map<String, dynamic>?>().firstWhere(
        (wallet) => wallet?['id'] == activeId,
        orElse: () => null,
      );
    }

    activeWalletData ??= _wallets.isNotEmpty ? _wallets.first : null;
    if (activeWalletData == null) {
      return;
    }

    await _instantiateWallet(activeWalletData);
  }

  Future<KanariWallet> _walletFromData(Map<String, dynamic> data) async {
    final curve = KanariCurve.values.firstWhere(
      (item) => item.name == data['curve'],
      orElse: () => KanariCurve.ed25519,
    );

    if (data['mnemonic'] != null &&
        data['mnemonic'].toString().isNotEmpty &&
        !curve.isPostQuantum) {
      return KanariWallet.fromMnemonic(data['mnemonic'], curve: curve);
    }

    return KanariWallet.fromPrivateKey(data['privateKey'], curve: curve);
  }

  Future<void> _instantiateWallet(Map<String, dynamic> data) async {
    _wallet = await _walletFromData(data);
    await refreshBalance();
  }

  Future<void> switchWallet(String walletId) async {
    final walletData = _wallets.cast<Map<String, dynamic>?>().firstWhere(
      (wallet) => wallet?['id'] == walletId,
      orElse: () => null,
    );

    if (walletData == null) return;

    await WalletStorage.setActiveWallet(walletId);
    _activeWalletId = walletId;
    await _instantiateWallet(walletData);
    notifyListeners();
  }

  Future<void> addWallet(Map<String, dynamic> walletData, [String? pin]) async {
    _wallets.add(walletData);

    if (pin != null && pin.isNotEmpty && _wallets.length == 1) {
      await WalletStorage.savePassword(pin);
    }

    await WalletStorage.saveAllWallets(_wallets);
    await switchWallet(walletData['id']);
    notifyListeners();
  }

  Future<void> removeWallet(String walletId) async {
    _wallets.removeWhere((wallet) => wallet['id'] == walletId);
    await WalletStorage.saveAllWallets(_wallets);

    if (_authenticatedWalletId == walletId) {
      _authenticatedWalletId = null;
    }

    final activeId = await WalletStorage.getActiveWalletId();
    if (activeId == walletId && _wallets.isNotEmpty) {
      await switchWallet(_wallets.first['id']);
    } else if (_wallets.isEmpty) {
      _wallet = null;
      _clearBalances();
    }

    notifyListeners();
  }

  Future<void> createNewWallet({
    KanariCurve curve = KanariCurve.ed25519,
    required String pin,
  }) async {
    await _withLoading(() async {
      _error = null;
      try {
        final wallet = await KanariWallet.generate(curve: curve);
        final walletData = {
          'id': DateTime.now().millisecondsSinceEpoch.toString(),
          'name': 'Wallet ${_wallets.length + 1}',
          'mnemonic': wallet.mnemonic,
          'privateKey': wallet.privateKey,
          'curve': curve.name,
          'createdAt': DateTime.now().toIso8601String(),
        };

        await addWallet(walletData, pin);
        _isUnlocked = true;
        await refreshBalance(notifyListenersOnSuccess: false);
      } catch (e) {
        _error = 'Creation failed: $e';
      }
    });
  }

  Future<void> unlockWallet(String pin) async {
    await _withLoading(() async {
      _error = null;

      final isValid = await WalletStorage.verifyPassword(pin);
      if (!isValid) {
        _error = 'Invalid PIN';
        notifyListeners();
        return;
      }

      await loadWallets();
      if (_wallets.isEmpty) {
        _error = 'No saved wallets';
        notifyListeners();
        return;
      }

      _isUnlocked = true;
      _error = null;
    });
  }

  Future<void> importFromPrivateKey(
    String pk, {
    KanariCurve curve = KanariCurve.ed25519,
    String? pin,
  }) async {
    await _withLoading(() async {
      _error = null;
      try {
        final wallet = await KanariWallet.fromPrivateKey(
          pk.trim(),
          curve: curve,
        );
        final walletData = {
          'id': DateTime.now().millisecondsSinceEpoch.toString(),
          'name': 'Imported Wallet ${_wallets.length + 1}',
          'mnemonic': '',
          'privateKey': wallet.privateKey,
          'curve': curve.name,
          'createdAt': DateTime.now().toIso8601String(),
        };

        await addWallet(walletData, pin);
        _isUnlocked = true;
        await refreshBalance(notifyListenersOnSuccess: false);
      } catch (e) {
        _error = 'Import PK failed: $e';
      }
    });
  }

  Future<void> importFromMnemonic(
    String mnemonic, {
    KanariCurve curve = KanariCurve.ed25519,
    String? pin,
  }) async {
    await _withLoading(() async {
      _error = null;
      try {
        final wallet = await KanariWallet.fromMnemonic(mnemonic, curve: curve);
        final walletData = {
          'id': DateTime.now().millisecondsSinceEpoch.toString(),
          'name': 'Imported Wallet ${_wallets.length + 1}',
          'mnemonic': mnemonic,
          'privateKey': wallet.privateKey,
          'curve': curve.name,
          'createdAt': DateTime.now().toIso8601String(),
        };

        await addWallet(walletData, pin);
        _isUnlocked = true;
        await refreshBalance(notifyListenersOnSuccess: false);
      } catch (e) {
        _error = 'Import Mnemonic failed: $e';
      }
    });
  }

  void logout() {
    _wallet = null;
    _error = null;
    _activeWalletId = null;
    _authenticatedWalletId = null;
    _isUnlocked = false;
    _clearBalances();
    notifyListeners();
  }

  Future<void> deleteAllWallets() async {
    await WalletStorage.deleteAllWallets();
    _wallets = [];
    _wallet = null;
    _error = null;
    _activeWalletId = null;
    _authenticatedWalletId = null;
    _isUnlocked = false;
    _clearBalances();
    notifyListeners();
  }

  Future<void> refreshBalance({bool notifyListenersOnSuccess = true}) async {
    if (_client == null || _wallet == null) {
      return;
    }

    try {
      _tokenBalances = await _client!.getAllBalances(_wallet!.address);
      _error = null;
    } catch (e) {
      _clearBalances();
      _error = 'Refresh balance failed: $e';
      debugPrint(_error);
    }

    if (notifyListenersOnSuccess) {
      notifyListeners();
    }
  }

  Future<String?> transfer(String recipient, int amount) async {
    if (_client == null || _wallet == null) return 'Client not initialized';
    return _runTransaction(() async {
      final result = await _client!.transfer(
        wallet: _wallet!,
        recipient: recipient,
        amount: amount,
      );
      return 'Success: Hash ${result.hash}';
    });
  }

  Future<String?> executeFunction({
    required String packageAddress,
    required String module,
    required String function,
    List<String> typeArgs = const [],
    List<List<int>> args = const [],
  }) async {
    if (_client == null || _wallet == null) return 'Client not initialized';
    return _runTransaction(() async {
      final result = await _client!.executeFunction(
        wallet: _wallet!,
        package: packageAddress,
        module: module,
        function: function,
        typeArgs: typeArgs,
        args: args,
      );
      return 'Success: Hash ${result.hash}';
    });
  }

  Future<String?> burn(int amount) async {
    if (_client == null || _wallet == null) return 'Client not initialized';
    return _runTransaction(() async {
      final result = await _client!.burn(wallet: _wallet!, amount: amount);
      return 'Success: Hash ${result.hash}';
    });
  }

  Future<String?> transferToken(
    String recipient,
    String tokenType,
    int amount,
  ) async {
    if (_client == null || _wallet == null) return 'Client not initialized';
    return _runTransaction(() async {
      final result = await _client!.transferToken(
        wallet: _wallet!,
        recipient: recipient,
        tokenType: tokenType,
        amount: amount,
      );
      return 'Success: Hash ${result.hash}';
    });
  }

  Future<bool> changePin(String oldPin, String newPin) async {
    try {
      final isValid = await WalletStorage.verifyPassword(oldPin);
      if (!isValid) return false;

      await WalletStorage.savePassword(newPin);
      await WalletStorage.saveAllWallets(_wallets);
      notifyListeners();
      return true;
    } catch (_) {
      return false;
    }
  }

  Future<String?> _runTransaction(Future<String> Function() action) async {
    return _withLoading(() async {
      try {
        final message = await action();
        await refreshBalance(notifyListenersOnSuccess: false);
        return message;
      } catch (e) {
        return 'Error: $e';
      }
    });
  }

  Future<T> _withLoading<T>(Future<T> Function() action) async {
    _setLoading(true);
    try {
      return await action();
    } finally {
      _setLoading(false);
    }
  }

  void _setLoading(bool value) {
    _isLoading = value;
    notifyListeners();
  }

  void _clearBalances({bool notify = false}) {
    _tokenBalances = [];
    if (notify) {
      notifyListeners();
    }
  }

  String _normalizeWalletAddress(String address) {
    final lower = address.trim().toLowerCase();
    return lower.startsWith('0x') ? lower.substring(2) : lower;
  }
}
